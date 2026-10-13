//! An intrusion under way, with the words: the rules of [`crate::run`] and what is said of them.

use thiserror::Error;

use crate::campaign::Difficulty;
use crate::command::{ArgKind, ArgRef, CommandSpec, Listing, Resolver};
use crate::content::Content;
use crate::content::ids::SiteId;
use crate::content::runs::site_tier;
use crate::event::{Event, Severity};
use crate::game::View;
use crate::prompt::Prompt;
use crate::run::solve::{Resolution, auto_resolve};
use crate::run::{Action, Mission, NodeId, Program, Run, RunState, Status, Undo, UndoError, Utility};
use crate::text::Text;

use super::gloss::{gloss_event, gloss_sections, gloss_view};
use super::lists::Lists;
use super::panel;
use super::speak::{Voice, key_of};
use super::view::RunView;

/// Why an intrusion cannot begin.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StartError {
    /// No such site.
    #[error("there is no site called `{0}`")]
    UnknownSite(String),
    /// The site has no intrusion (the content tests rule this out for the shipped sites).
    #[error("the site `{0}` has no intrusion")]
    NoIntrusion(String),
    /// A saved intrusion that is not a turn of this site's intrusion.
    #[error("the saved intrusion on `{site}` cannot be taken up: {reason}")]
    BadSave {
        /// The site.
        site: String,
        /// What is wrong with it.
        reason: &'static str,
    },
}

/// Something the intrusion asks before it does what cannot be taken back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    /// `jackout` with loot still there.
    Jackout {
        /// How much loot is left behind.
        left: u8,
    },
    /// `skip`: play the intrusion automatically.
    Skip,
    /// `quit`: leave the intrusion under way.
    Quit,
}

impl Question {
    /// The question, as the prompt that asks it (no is the default: nothing is lost by it).
    #[must_use]
    pub fn prompt(self) -> Prompt {
        let question = match self {
            Self::Jackout { left } => {
                Text::new("run.jackout.confirm").with_int("left", i64::from(left))
            }
            Self::Skip => Text::new("run.skip.confirm"),
            Self::Quit => Text::new("run.quit.confirm"),
        };
        Prompt::Confirm {
            question,
            default: false,
        }
    }
}

/// What a command of the intrusion answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply {
    /// It was done, or refused with its reason.
    Done,
    /// It asks first.
    Ask(Question),
}

/// How an intrusion that played itself ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Automatic {
    /// A plan of the solver was played.
    Plan,
    /// No plan was found: the intrusion ended at the Trace its tier calls nominal.
    Nominal,
}


/// An intrusion on a site.
#[derive(Debug, Clone)]
pub struct Intrusion {
    site: SiteId,
    /// The site as it is written in a text key.
    key: String,
    difficulty: Difficulty,
    run: Run,
    /// The final Trace of an automatic intrusion that no plan could play out.
    nominal_trace: u8,
    automatic: Option<Automatic>,
}

impl Intrusion {
    /// Begins the intrusion on a site, at the pace and with the undos of the difficulty.
    ///
    /// # Errors
    ///
    /// [`StartError`] when the site does not exist or has no intrusion.
    pub fn new(
        content: &Content,
        site: &SiteId,
        difficulty: Difficulty,
    ) -> Result<Self, StartError> {
        let (mission, nominal) = Self::mission_of(content, site, difficulty)?;
        Ok(Self::from_mission(site.clone(), difficulty, mission).with_nominal_trace(nominal))
    }

    /// Takes up an intrusion that was saved at the start of a turn.
    ///
    /// # Errors
    ///
    /// [`StartError`] when the site is gone, or the saved turn is not one of its intrusion.
    pub fn resume(
        content: &Content,
        site: &SiteId,
        difficulty: Difficulty,
        turn_start: RunState,
        undo: Undo,
    ) -> Result<Self, StartError> {
        let (mission, nominal) = Self::mission_of(content, site, difficulty)?;
        if let Some(reason) = mission.turn_start_error(&turn_start) {
            return Err(StartError::BadSave {
                site: site.as_str().to_owned(),
                reason,
            });
        }
        Ok(Self {
            key: key_of(site.as_str()),
            run: Run::resume(mission, turn_start, undo),
            site: site.clone(),
            difficulty,
            nominal_trace: nominal,
            automatic: None,
        })
    }

    fn mission_of(
        content: &Content,
        site: &SiteId,
        difficulty: Difficulty,
    ) -> Result<(Mission, u8), StartError> {
        let def = content
            .site(site)
            .ok_or_else(|| StartError::UnknownSite(site.as_str().to_owned()))?;
        let mission = content
            .mission(site, difficulty.pace())
            .ok_or_else(|| StartError::NoIntrusion(site.as_str().to_owned()))?;
        let tier = usize::from(site_tier(def)).max(1);
        let nominal = content
            .auto_resolve
            .nominal_trace
            .get(tier - 1)
            .copied()
            .unwrap_or(50);
        Ok((mission, nominal))
    }

    /// An intrusion on a mission built elsewhere (the tests build small ones).
    #[must_use]
    pub fn from_mission(site: SiteId, difficulty: Difficulty, mission: Mission) -> Self {
        Self {
            key: key_of(site.as_str()),
            run: Run::new(mission, difficulty.undo()),
            site,
            difficulty,
            nominal_trace: 50,
            automatic: None,
        }
    }

    fn with_nominal_trace(mut self, trace: u8) -> Self {
        self.nominal_trace = trace;
        self
    }

    fn voice(&self) -> Voice<'_> {
        Voice {
            mission: self.run.mission(),
            site: &self.key,
            site_id: self.site.as_str(),
            horizon: if self.difficulty == Difficulty::Story {
                2
            } else {
                1
            },
        }
    }

    /// The site.
    #[must_use]
    pub fn site(&self) -> &SiteId {
        &self.site
    }

    /// The difficulty the intrusion is played at.
    #[must_use]
    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    /// The run, for the tests and tools that read an intrusion.
    #[must_use]
    pub fn run(&self) -> &Run {
        &self.run
    }

    /// The state of the run.
    #[must_use]
    pub fn state(&self) -> &RunState {
        self.run.state()
    }

    /// What a save keeps of the intrusion: the start of the turn under way, and the undos left.
    #[must_use]
    pub fn saved(&self) -> (RunState, Undo) {
        (*self.run.turn_start(), self.run.undo_left())
    }

    /// Whether the run is over (won or burned).
    #[must_use]
    pub fn is_over(&self) -> bool {
        self.run.status() != Status::Running
    }

    /// How the intrusion played itself, if it did.
    #[must_use]
    pub fn automatic(&self) -> Option<Automatic> {
        self.automatic
    }

    /// Whether `skip` is open: in Story difficulty only.
    #[must_use]
    pub fn can_skip(&self) -> bool {
        self.difficulty == Difficulty::Story
    }

    /// The loot nodes taken so far.
    #[must_use]
    pub fn looted(&self) -> Vec<NodeId> {
        self.run
            .mission()
            .nodes()
            .filter(|node| self.run.state().is_looted(*node))
            .collect()
    }

    /// How much loot is still there to be taken.
    #[must_use]
    pub fn loot_left(&self) -> u8 {
        let (taken, total) = self.voice().loot_count(self.run.state());
        total.saturating_sub(taken)
    }

    /// What the intrusion says when it begins: the site, the turn, the forecast.
    #[must_use]
    pub fn opening(&self) -> Vec<Event> {
        let mut events = vec![Event::system(
            Text::new("run.start").with_text("name", Text::raw(self.site.as_str())),
        )];
        events.extend(self.situation());
        events
    }

    /// Where the player stands: the banner of the turn and its forecast. What a game that is
    /// taken over, or loaded, says first; never the opening.
    #[must_use]
    pub fn situation(&self) -> Vec<Event> {
        let voice = self.voice();
        let state = self.run.state();
        let mut events = vec![voice.turn_banner(state)];
        events.extend(voice.forecast_events(state));
        events
    }

    /// Does an action. Returns whether it was done; a refused action says why and changes
    /// nothing.
    pub fn act(&mut self, action: Action, events: &mut Vec<Event>) -> bool {
        let before = *self.run.state();
        match self.run.act(action) {
            Ok(facts) => {
                let after = *self.run.state();
                let voice = self.voice();
                events.extend(if action == Action::End {
                    voice.after_end(&before, &after, &facts)
                } else {
                    voice.after_action(&before, &after, &facts)
                });
                true
            }
            Err(error) => {
                events.push(Voice::refusal(error));
                false
            }
        }
    }

    fn node_arg(args: &[ArgRef], position: usize) -> Option<NodeId> {
        let number: u8 = args.get(position)?.id()?.parse().ok()?;
        NodeId::from_number(number)
    }

    fn program_arg(args: &[ArgRef], position: usize) -> Option<Program> {
        Program::from_id(args.get(position)?.id()?)
    }

    /// Runs a command of the intrusion whose arguments are read (`map`, `move`, `breach`,
    /// `use`, `loot`, `end`, `jackout`, `undo`, `skip`).
    pub fn execute(&mut self, name: &str, args: &[ArgRef], events: &mut Vec<Event>) -> Reply {
        match name {
            "map" => events.extend(self.map()),
            "move" => {
                if let Some(node) = Self::node_arg(args, 0) {
                    self.act(Action::Move(node), events);
                }
            }
            "breach" => {
                if let (Some(node), Some(program)) =
                    (Self::node_arg(args, 0), Self::program_arg(args, 1))
                {
                    self.act(Action::Breach { node, program }, events);
                }
            }
            "use" => {
                if let Some(utility) = Self::program_arg(args, 0).and_then(Utility::of) {
                    self.act(Action::Use(utility), events);
                }
            }
            "loot" => {
                self.act(Action::Loot, events);
            }
            "end" => {
                self.act(Action::End, events);
            }
            "jackout" => {
                let left = self.loot_left();
                if self.run.state().objectives_done(self.run.mission()) && left > 0 {
                    return Reply::Ask(Question::Jackout { left });
                }
                self.act(Action::Jackout, events);
            }
            "undo" => self.undo(events),
            "skip" => {
                if self.can_skip() {
                    return Reply::Ask(Question::Skip);
                }
                events.push(Event::error(Text::new("run.error.skip")));
            }
            other => events.push(Event::error(
                Text::new("campaign.frontend.unhandled").with_str("command", other),
            )),
        }
        Reply::Done
    }

    /// The answer to a [`Question::Jackout`] or [`Question::Skip`]; `quit` is for whoever runs
    /// the intrusion to answer.
    pub fn answer(&mut self, question: Question, yes: bool, events: &mut Vec<Event>) {
        if !yes {
            return;
        }
        match question {
            Question::Jackout { .. } => {
                self.act(Action::Jackout, events);
            }
            Question::Skip => self.skip(),
            Question::Quit => {}
        }
    }

    /// Plays the intrusion by itself: the best plan the bounded solver finds, or, when it finds
    /// none, the end the tier calls nominal.
    fn skip(&mut self) {
        let start = *self.run.state();
        let played = match auto_resolve(self.run.mission(), &start) {
            Resolution::Optimal(plan) | Resolution::Best(plan) => {
                plan.actions.iter().all(|action| self.run.act(*action).is_ok())
                    && self.run.status() == Status::Won
            }
            Resolution::Nominal => false,
        };
        if played {
            self.automatic = Some(Automatic::Plan);
        } else {
            let settled = self.run.mission().settled_at(&start, self.nominal_trace);
            self.run.settle(settled);
            self.automatic = Some(Automatic::Nominal);
        }
    }

    /// Takes back the turn under way (or the one before).
    pub fn undo(&mut self, events: &mut Vec<Event>) {
        match self.run.undo() {
            Ok(()) => {
                let voice = self.voice();
                let state = self.run.state();
                let done = match self.run.undo_left() {
                    Undo::Unlimited => Text::new("run.undo.done_unlimited"),
                    Undo::Left(count) => {
                        Text::new("run.undo.done").with_int("left", i64::from(count))
                    }
                };
                events.push(Event::system(
                    done.with_int("number", i64::from(state.turn()))
                        .with_int("value", voice.shown(state.trace()))
                        .with_int("max", i64::from(voice.mission.trace_cap()))
                        .with_text("band", Voice::band(state.trace(), false)),
                ));
                events.extend(self.situation());
            }
            Err(UndoError::RunOver) => events.push(Event::error(Text::new("run.error.over"))),
            Err(UndoError::NoneLeft) => events.push(Event::error(Text::new("run.undo.none"))),
            Err(UndoError::NothingToTakeBack) => {
                events.push(Event::error(Text::new("run.undo.nothing")));
            }
        }
    }

    /// `map`.
    #[must_use]
    pub fn map(&self) -> Vec<Event> {
        self.voice().map(self.run.state(), self.run.undo_left())
    }

    /// `status`.
    #[must_use]
    pub fn status(&self) -> Vec<Event> {
        self.voice().status(self.run.state(), self.run.undo_left())
    }

    /// The result and the report that close the intrusion; nothing while it is under way.
    #[must_use]
    pub fn closing(&self) -> Vec<Event> {
        let state = self.run.state();
        let voice = self.voice();
        let result = match state.status() {
            Status::Running => return Vec::new(),
            Status::Won => Event::system(
                Text::new("run.result.won").with_text("name", Text::raw(self.site.as_str())),
            ),
            Status::Burned => Event::alert(
                Severity::Danger,
                Text::new("run.result.burned")
                    .with_int("max", i64::from(voice.mission.trace_cap())),
            ),
        };
        let mut events = vec![result];
        events.extend(self.automatic.map(|automatic| {
            Event::system(Text::new(match automatic {
                Automatic::Plan => "run.result.automatic",
                Automatic::Nominal => "run.result.approximate",
            }))
        }));
        events.push(Event::Screen(voice.report(state)));
        events
    }

    /// What an interface draws its panel from.
    #[must_use]
    pub fn view(&self) -> RunView {
        self.voice().view(self.run.state(), self.run.undo_left())
    }

    /// The panel and the status bar of the full-screen interface, with the terms every text
    /// of the intrusion cites (`player` is the name the bar starts with).
    #[must_use]
    pub fn panel_view(&self, player: String) -> View {
        let mut run = self.view();
        gloss_view(&mut run);
        let mut sections = panel::sections(&run);
        let mut status = panel::status_items(&run);
        gloss_sections(&mut sections, &mut status);
        View {
            player,
            gauges: vec![run.trace],
            sections,
            status,
        }
    }

    /// Gives the texts of events the terms an intrusion cites, for a game that says them in its
    /// own steps.
    pub fn gloss(events: &mut [Event]) {
        events.iter_mut().for_each(gloss_event);
    }

    /// The things a command of the intrusion can name: the nodes by number, the programs of the
    /// deck by identifier.
    #[must_use]
    pub fn listing(&self, kind: ArgKind, command: &CommandSpec) -> Listing {
        Lists {
            mission: self.run.mission(),
            state: self.run.state(),
        }
        .list(kind, command)
    }
}
