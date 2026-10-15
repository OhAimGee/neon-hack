//! The driver: plays a whole campaign through the frontend contract.
//!
//! It does what a plain frontend does and nothing more: it sends the lines and the answers the
//! game asks for (`Game::handle`), reads the prompts the game gives, and never touches the
//! state. What to type comes from the [`Brain`]; what is seen of each step
//! goes to an [`Observer`], which is where the tests check texts and saves and where the
//! harness counts. A game that cannot be played on is a [`Stuck`], never a panic: the harness
//! reports it and goes on with the next game.

use std::collections::VecDeque;
use std::fmt;

use neon_engine::Game;
use neon_engine::campaign::CampaignGame;
use neon_engine::content::schema::{QuestStatus, ReadableKind};
use neon_engine::prompt::{Input, Prompt, Step};
use neon_engine::run::Action;
use neon_engine::run::solve::{Goal, Outcome, PROOF_EXPANSIONS, Resolution, auto_resolve, solve};
use neon_engine::save::SaveRequest;

use crate::brain::{Brain, Move, Plan};
use crate::sites::full_loot_mission;

/// Prompts one command may cause (a conversation with a few topics, a decision).
const MAX_PROMPTS: usize = 24;
/// The same move this many times in a row means the walk is going round in circles.
const MAX_REPEATS: usize = 12;
/// The inputs remembered for a failure report.
const RECENT: usize = 30;

/// What a driver is told to do besides playing the plan.
#[derive(Debug, Clone)]
pub struct Config {
    /// The handle typed at the start.
    pub handle: String,
    /// Safety net: the longest walk is a few hundred commands.
    pub max_steps: usize,
    /// Types `save 3` once, after this many steps (a slot save is a path of its own).
    pub slot_save_after: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            handle: "Walker".to_owned(),
            max_steps: 2_500,
            slot_save_after: None,
        }
    }
}

/// A walk that cannot go on: what went wrong, the last inputs and what was still to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stuck {
    /// What went wrong.
    pub why: String,
    /// The last inputs sent, oldest first.
    pub recent: Vec<String>,
    /// The quests still open and why (from the brain).
    pub pending: String,
    /// What the observer wants said with the failure (the last thing the game said).
    pub context: String,
}

impl fmt::Display for Stuck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\n  last inputs:", self.why)?;
        for line in &self.recent {
            write!(f, "\n    {line}")?;
        }
        write!(f, "\n  state:\n{}", self.pending)?;
        if !self.context.is_empty() {
            write!(f, "  last said:\n{}", self.context)?;
        }
        Ok(())
    }
}

impl std::error::Error for Stuck {}

/// What looks at each step of the walk.
pub trait Observer {
    /// Called with every step the game gives, the first one (the start) included. The game may
    /// be replaced here (a reload through a save): the walk plays on with the new one. An
    /// error stops the walk.
    ///
    /// # Errors
    ///
    /// The reason the step is not acceptable.
    fn observe(&mut self, game: &mut CampaignGame, what: &str, step: &Step) -> Result<(), String>;

    /// What to add to the report of a walk that cannot go on.
    fn context(&self) -> String {
        String::new()
    }
}

/// An observer that sees nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct Blind;

impl Observer for Blind {
    fn observe(&mut self, _: &mut CampaignGame, _: &str, _: &Step) -> Result<(), String> {
        Ok(())
    }
}

/// What a finished walk leaves.
#[derive(Debug)]
pub struct Walk<O> {
    /// The inputs sent to the game.
    pub steps: usize,
    /// Every command line typed.
    pub typed: Vec<String>,
    /// Every line a plain frontend would read to send the same inputs (`y` for a yes, an
    /// empty line for "continue", `skip` for backing out): the script of the walk.
    pub script: Vec<String>,
    /// The game, over.
    pub game: CampaignGame,
    /// The observer, with what it saw.
    pub observer: O,
}

/// Plays a campaign from a new game to the epilogue.
#[derive(Debug)]
pub struct Driver<O> {
    game: CampaignGame,
    plan: Plan,
    observer: O,
    config: Config,
    steps: usize,
    typed: Vec<String>,
    script: Vec<String>,
    recent: VecDeque<String>,
    saved_slot: bool,
    last_save: Option<SaveRequest>,
}

impl<O: Observer> Driver<O> {
    /// A driver for a new game.
    #[must_use]
    pub fn new(game: CampaignGame, plan: Plan, observer: O, config: Config) -> Self {
        Self {
            game,
            plan,
            observer,
            config,
            steps: 0,
            typed: Vec::new(),
            script: Vec::new(),
            recent: VecDeque::new(),
            saved_slot: false,
            last_save: None,
        }
    }

    /// The game being played.
    #[must_use]
    pub fn game(&self) -> &CampaignGame {
        &self.game
    }

    /// The observer.
    #[must_use]
    pub fn observer(&self) -> &O {
        &self.observer
    }

    /// The report of a walk that cannot go on.
    #[must_use]
    pub fn stuck(&self, why: &str) -> Stuck {
        Stuck {
            why: why.to_owned(),
            recent: self.recent.iter().cloned().collect(),
            pending: Brain::new(&self.game, &self.plan).describe_pending(),
            context: self.observer.context(),
        }
    }

    // ------------------------------------------------------------------ the walk

    /// Plays from a new game to the end of the campaign, then leaves.
    ///
    /// # Errors
    ///
    /// A [`Stuck`] when the walk cannot go on: a move the game refuses, a menu that does not
    /// end, the plan running out of moves before the epilogue, or the observer's refusal.
    pub fn play(mut self) -> Result<Walk<O>, Stuck> {
        let start = self.game.start();
        self.observe("(start)", &start)?;
        self.opening()?;
        let mut last: Option<Move> = None;
        let mut repeats = 0;
        loop {
            if self.steps > self.config.max_steps {
                return Err(self.stuck(&format!("more than {} steps", self.config.max_steps)));
            }
            if let Some(after) = self.config.slot_save_after
                && !self.saved_slot
                && self.steps > after
            {
                self.saved_slot = true;
                self.type_line("save 3")?;
                if self.last_save != Some(SaveRequest::Slot(3)) {
                    return Err(self.stuck("`save 3` does not ask for slot 3"));
                }
            }
            let next = Brain::new(&self.game, &self.plan).next_move();
            let Some(mv) = next else { break };
            // The same move again and again is a move that does not do what it should (three
            // conversations at most are ever asked for in a row).
            repeats = if last.as_ref() == Some(&mv) {
                repeats + 1
            } else {
                0
            };
            if repeats >= MAX_REPEATS {
                return Err(self.stuck(&format!(
                    "{mv:?} done {MAX_REPEATS} times in a row to no avail"
                )));
            }
            self.perform(&mv)?;
            last = Some(mv);
        }
        self.finish()
    }

    /// The handle, the prologue skipped, the tutorial declined.
    fn opening(&mut self) -> Result<(), Stuck> {
        self.send("(handle)", Input::Line(self.config.handle.clone()))?;
        if !matches!(self.game.prompt(), Prompt::Continue) {
            return Err(self.stuck("no prologue after the handle"));
        }
        // A plain frontend turns the word `skip` at a page into backing out.
        self.send("(skip)", Input::Cancel)?;
        if !matches!(self.game.prompt(), Prompt::Confirm { .. }) {
            return Err(self.stuck("no offer of the tutorial after the prologue"));
        }
        self.send("(no tutorial)", Input::Confirm(false))?;
        if self.game.prompt() != Prompt::Command {
            return Err(self.stuck("no command prompt after the tutorial"));
        }
        Ok(())
    }

    fn finish(mut self) -> Result<Walk<O>, Stuck> {
        let done = "m14"
            .parse()
            .is_ok_and(|id| self.game.state().missions().status(&id) == QuestStatus::Completed);
        if !done {
            return Err(self.stuck("the walk ran out of moves before the end"));
        }
        // Leaving saves and ends the game.
        self.type_line("quit")?;
        self.settle(&Move::Tick)?;
        if self.game.prompt() != Prompt::End {
            return Err(self.stuck("`quit` does not end the game"));
        }
        Ok(Walk {
            steps: self.steps,
            typed: self.typed,
            script: self.script,
            game: self.game,
            observer: self.observer,
        })
    }

    // --------------------------------------------------------------------- a move

    fn perform(&mut self, mv: &Move) -> Result<(), Stuck> {
        let line = match mv {
            Move::Accept(quest) => format!("accept {quest}"),
            Move::Talk { contact, .. } => format!("talk {contact}"),
            Move::Buy(item) => format!("buy {item}"),
            Move::Hack(site) => format!("hack {site}"),
            Move::Read(id) => {
                let mail = self
                    .game
                    .content()
                    .readable(id)
                    .is_some_and(|r| r.kind == ReadableKind::Mail);
                format!("{} {id}", if mail { "read" } else { "archives" })
            }
            Move::Decrypt(id) => format!("decrypt {id}"),
            Move::Link(contact) => format!("link {contact}"),
            Move::Use(command) => (*command).to_owned(),
            Move::Cool(service) => format!("laylow {service}"),
            Move::Tick => "talk echo7".to_owned(),
        };
        self.type_line(&line)?;
        self.settle(mv)?;
        if matches!(mv, Move::Hack(_)) {
            self.play_intrusion(mv)?;
        }
        Ok(())
    }

    /// Plays the intrusion that `hack` opened to its end, typing the commands. The player takes
    /// all the loot (the files are what the quests ask for), by the best plan the bounded solver
    /// finds for a mission whose every loot is an objective: the same rules, so each of its
    /// commands is a command of the real intrusion.
    fn play_intrusion(&mut self, mv: &Move) -> Result<(), Stuck> {
        while let Some(intrusion) = self.game.intrusion() {
            let content = self.game.content();
            let site = intrusion.site().clone();
            let Some(mission) =
                full_loot_mission(content, &site, self.game.state().difficulty().pace())
            else {
                return Err(self.stuck("the full-loot variant of an intrusion does not build"));
            };
            let state = *intrusion.state();
            // The bounded chain of `skip` is enough for the small intrusions; a heavy one, with
            // all its loot to take, gets the wide bound of the tools instead: the harness is not
            // the game, and what it asks is whether the intrusion can be won.
            let plan = match auto_resolve(&mission, &state) {
                Resolution::Optimal(plan) | Resolution::Best(plan) => plan,
                Resolution::Nominal => {
                    match solve(&mission, &state, Goal::Feasible, PROOF_EXPANSIONS, u8::MAX).outcome
                    {
                        Outcome::Solved { plan, .. }
                        | Outcome::Unknown {
                            best: Some(plan), ..
                        } => plan,
                        Outcome::Unknown { best: None, .. } | Outcome::Unsolvable => {
                            return Err(self.stuck(&format!("no plan for the intrusion of {site}")));
                        }
                    }
                }
            };
            for action in &plan.actions {
                if self.game.intrusion().is_none() {
                    break;
                }
                let line = match action {
                    Action::Move(node) => format!("move {}", node.number()),
                    Action::Breach { node, program } => {
                        format!("breach {} {}", node.number(), program.id())
                    }
                    Action::Use(utility) => format!("use {}", utility.program().id()),
                    Action::Loot => "loot".to_owned(),
                    Action::End => "end".to_owned(),
                    Action::Jackout => "jackout".to_owned(),
                };
                self.type_line(&line)?;
                self.settle(mv)?;
            }
            if self.game.intrusion().is_some() {
                return Err(self.stuck("a plan that does not end the intrusion"));
            }
        }
        Ok(())
    }

    /// Types a line at the command prompt.
    fn type_line(&mut self, line: &str) -> Result<(), Stuck> {
        if self.game.prompt() != Prompt::Command {
            return Err(self.stuck(&format!("`{line}` typed away from the command prompt")));
        }
        self.typed.push(line.to_owned());
        self.send(line, Input::Line(line.to_owned()))
    }

    /// Answers the prompts a command causes, until the command prompt is back.
    fn settle(&mut self, mv: &Move) -> Result<(), Stuck> {
        for _ in 0..MAX_PROMPTS {
            match self.game.prompt() {
                Prompt::Command | Prompt::End => return Ok(()),
                Prompt::Confirm { .. } => self.send("(yes)", Input::Confirm(true))?,
                Prompt::Continue => self.send("(continue)", Input::Continue)?,
                Prompt::Text { .. } => {
                    return Err(self.stuck("a text prompt in the middle of the walk"));
                }
                Prompt::Choice(choice) => {
                    let ids: Vec<(&str, bool)> = choice
                        .options
                        .iter()
                        .map(|o| (o.id.as_str(), o.available.is_ok()))
                        .collect();
                    let answer = pick(mv, &ids);
                    self.send(&format!("(menu {answer})"), Input::Line(answer))?;
                }
            }
        }
        Err(self.stuck("a menu that does not end"))
    }

    // ----------------------------------------------------------------- one step

    fn send(&mut self, what: &str, input: Input) -> Result<(), Stuck> {
        self.recent.push_back(what.to_owned());
        if self.recent.len() > RECENT {
            self.recent.pop_front();
        }
        self.script.push(match &input {
            Input::Line(line) => line.clone(),
            Input::Confirm(true) => "y".to_owned(),
            Input::Confirm(false) => "n".to_owned(),
            Input::Cancel => "skip".to_owned(),
            Input::Continue | Input::Choice(_) | Input::Eof => String::new(),
        });
        let step = self.game.handle(input);
        self.steps += 1;
        self.observe(what, &step)
    }

    /// Everything checked about a step: the driver's own checks, then the observer's.
    fn observe(&mut self, what: &str, step: &Step) -> Result<(), Stuck> {
        if step.prompt != self.game.prompt() {
            return Err(self.stuck(&format!("`{what}`: the step's prompt is not the game's")));
        }
        // Every prompt has a way forward: a menu has an entry that can be taken, or a way out.
        if let Prompt::Choice(choice) = &step.prompt
            && choice.cancel.is_none()
            && choice
                .options
                .iter()
                .all(|option| option.available.is_err())
        {
            return Err(self.stuck(&format!("`{what}`: a menu with no way forward")));
        }
        self.last_save = step.save;
        self.observer
            .observe(&mut self.game, what, step)
            .map_err(|why| self.stuck(&why))
    }
}

/// The menu entry to take: what the move came for, else a topic not yet heard, else out.
fn pick(mv: &Move, options: &[(&str, bool)]) -> String {
    let open = |id: &str| options.iter().any(|(o, ok)| *ok && *o == id);
    if let Move::Talk { pay, decide, .. } = mv {
        if let Some(quest) = pay {
            let id = format!("pay:{quest}");
            if open(&id) {
                return id;
            }
        }
        if let Some((decision, choice)) = decide {
            // The decision menu: the options are the choices themselves.
            if options.iter().all(|(o, _)| !o.contains(':')) && open(choice.as_str()) {
                return choice.to_string();
            }
            let id = format!("decide:{decision}");
            if open(&id) {
                return id;
            }
        }
    }
    options
        .iter()
        .find(|(id, ok)| *ok && id.starts_with("topic:"))
        .map_or_else(|| "0".to_owned(), |(id, _)| (*id).to_owned())
}
