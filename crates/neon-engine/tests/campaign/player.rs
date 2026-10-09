//! The optimistic player at the command level: it plays the real campaign through the plain
//! frontend contract (`Game::handle` with the lines and answers a frontend sends, using the
//! prompts the game gives), from a new game to the epilogue, and checks everything it sees.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as _;
use std::sync::OnceLock;

use neon_engine::campaign::{CampaignGame, Difficulty};
use neon_engine::event::{Event, Role};
use neon_engine::prompt::{Input, Prompt, Step};
use neon_engine::save::SaveRequest;
use neon_engine::text::{Arg, Catalog, Lang, RenderMode, Text, render};
use neon_engine::{Game, content::Content};

use crate::brain::{Brain, Move, Plan};

/// Safety net: the longest walk is a few hundred commands.
const MAX_STEPS: usize = 2_500;
/// Prompts one command may cause (a conversation with a few topics, a decision).
const MAX_PROMPTS: usize = 24;
/// The same move this many times in a row means the walk is going round in circles.
const MAX_REPEATS: usize = 12;
/// The handle of the walker.
const HANDLE: &str = "Walker";

/// What the walk does with the saves the game asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reload {
    /// Nothing: the game plays on.
    Never,
    /// At every checkpoint and slot, the game is saved and loaded back, and the loaded copy
    /// plays on (the walk must not notice).
    Checkpoints,
    /// The same at every save the game asks for, autosaves included: mid-menu too.
    EverySave,
}

fn catalogs() -> &'static Vec<(Lang, Catalog)> {
    static CATALOGS: OnceLock<Vec<(Lang, Catalog)>> = OnceLock::new();
    CATALOGS.get_or_init(|| {
        Lang::ALL
            .iter()
            .map(|lang| (*lang, Catalog::embedded(*lang).unwrap()))
            .collect()
    })
}

/// FNV-1a: a stable hash, so a transcript hash means the same on every run.
fn fnv(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// What a finished walk reports.
#[derive(Debug)]
pub(crate) struct Walked {
    pub(crate) steps: usize,
    /// FNV-1a of everything the game said (as data, so it does not depend on a language).
    pub(crate) hash: u64,
    pub(crate) checkpoints: usize,
    pub(crate) reloads: usize,
    /// The ending titles seen, in order.
    pub(crate) endings: Vec<String>,
    /// The epilogue lines seen, in order.
    pub(crate) epilogue: Vec<String>,
    /// How many times each quest was announced as completed.
    pub(crate) completed: BTreeMap<String, u32>,
    /// The credits announced as rewards.
    pub(crate) rewarded: i64,
    /// Every command line typed.
    pub(crate) typed: Vec<String>,
    /// Every line a plain frontend would read to send the same inputs (`y` for a yes, an
    /// empty line for "continue", `skip` for backing out): the script of the walk.
    pub(crate) script: Vec<String>,
    pub(crate) game: CampaignGame,
}

pub(crate) struct Player {
    game: CampaignGame,
    plan: Plan,
    reload: Reload,
    modes: Vec<RenderMode>,
    steps: usize,
    hash: u64,
    checkpoints: usize,
    reloads: usize,
    endings: Vec<String>,
    epilogue: Vec<String>,
    completed: BTreeMap<String, u32>,
    rewarded: i64,
    typed: Vec<String>,
    script: Vec<String>,
    recent: VecDeque<String>,
    last_said: String,
    saved_slot: bool,
    last_save: Option<SaveRequest>,
}

impl Player {
    /// A new campaign. The renderings checked at every step are English and French in full
    /// mode, and with `thorough` also the screen-reader and ASCII modes.
    pub(crate) fn new(plan: &Plan, difficulty: Difficulty, reload: Reload, thorough: bool) -> Self {
        let mut modes = vec![RenderMode::FULL];
        if thorough {
            modes.extend([
                RenderMode::SCREEN_READER,
                RenderMode::ASCII,
                RenderMode {
                    screen_reader: true,
                    ascii: true,
                },
            ]);
        }
        Self {
            game: CampaignGame::new(difficulty, 7).unwrap(),
            plan: plan.clone(),
            reload,
            modes,
            steps: 0,
            hash: 0xcbf2_9ce4_8422_2325,
            checkpoints: 0,
            reloads: 0,
            endings: Vec::new(),
            epilogue: Vec::new(),
            completed: BTreeMap::new(),
            rewarded: 0,
            typed: Vec::new(),
            script: Vec::new(),
            recent: VecDeque::new(),
            last_said: String::new(),
            saved_slot: false,
            last_save: None,
        }
    }

    // ------------------------------------------------------------------ the walk

    /// Plays from a new game to the end of the campaign, then leaves.
    pub(crate) fn play(mut self) -> Walked {
        let start = self.game.start();
        self.observe("(start)", &start);
        self.opening();
        let mut last: Option<Move> = None;
        let mut repeats = 0;
        loop {
            if self.steps > MAX_STEPS {
                self.fail(&format!("more than {MAX_STEPS} steps"));
            }
            if !self.saved_slot && self.steps > 40 {
                self.saved_slot = true;
                self.type_line("save 3");
                assert_eq!(
                    self.last_save,
                    Some(SaveRequest::Slot(3)),
                    "`save 3` asks for slot 3"
                );
            }
            let next = Brain::new(&self.game, &self.plan).next_move();
            match next {
                Some(mv) => {
                    // The same move again and again is a move that does not do what it
                    // should (three conversations at most are ever asked for in a row).
                    repeats = if last.as_ref() == Some(&mv) {
                        repeats + 1
                    } else {
                        0
                    };
                    if repeats >= MAX_REPEATS {
                        self.fail(&format!(
                            "{mv:?} done {MAX_REPEATS} times in a row to no avail"
                        ));
                    }
                    self.perform(&mv);
                    last = Some(mv);
                }
                None => break,
            }
        }
        self.finish()
    }

    /// The handle, the prologue skipped, the tutorial declined.
    fn opening(&mut self) {
        self.send("(handle)", Input::Line(HANDLE.to_owned()));
        assert!(matches!(self.game.prompt(), Prompt::Continue));
        // A plain frontend turns the word `skip` at a page into backing out.
        self.send("(skip)", Input::Cancel);
        assert!(matches!(self.game.prompt(), Prompt::Confirm { .. }));
        self.send("(no tutorial)", Input::Confirm(false));
        assert_eq!(self.game.prompt(), Prompt::Command);
    }

    fn finish(mut self) -> Walked {
        let brain = Brain::new(&self.game, &self.plan);
        let pending = brain.describe_pending();
        let done = self.game.state().missions().status(&"m14".parse().unwrap())
            == neon_engine::content::schema::QuestStatus::Completed;
        if !done {
            self.fail(&format!(
                "the walk ran out of moves before the end:\n{pending}"
            ));
        }
        // Leaving saves and ends the game.
        self.type_line("quit");
        self.settle(&Move::Tick);
        assert_eq!(self.game.prompt(), Prompt::End, "quit ends the game");
        Walked {
            steps: self.steps,
            hash: self.hash,
            checkpoints: self.checkpoints,
            reloads: self.reloads,
            endings: self.endings,
            epilogue: self.epilogue,
            completed: self.completed,
            rewarded: self.rewarded,
            typed: self.typed,
            script: self.script,
            game: self.game,
        }
    }

    // --------------------------------------------------------------------- a move

    fn perform(&mut self, mv: &Move) {
        let line = match mv {
            Move::Accept(quest) => format!("accept {quest}"),
            Move::Talk { contact, .. } => format!("talk {contact}"),
            Move::Buy(item) => format!("buy {item}"),
            Move::Hack(site) => format!("hack {site}"),
            Move::Read(id) => {
                let content = self.game.content();
                let command = match content.readable(id).map(|r| r.kind) {
                    Some(neon_engine::content::schema::ReadableKind::Mail) => "read",
                    _ => "archives",
                };
                format!("{command} {id}")
            }
            Move::Decrypt(id) => format!("decrypt {id}"),
            Move::Link(contact) => format!("link {contact}"),
            Move::Use(command) => (*command).to_owned(),
            Move::Cool(service) => format!("laylow {service}"),
            Move::Tick => "talk echo7".to_owned(),
        };
        self.type_line(&line);
        self.settle(mv);
    }

    /// Types a line at the command prompt.
    fn type_line(&mut self, line: &str) {
        assert_eq!(
            self.game.prompt(),
            Prompt::Command,
            "`{line}` typed away from the command prompt"
        );
        self.typed.push(line.to_owned());
        self.send(line, Input::Line(line.to_owned()));
    }

    /// Answers the prompts a command causes, until the command prompt is back.
    fn settle(&mut self, mv: &Move) {
        for _ in 0..MAX_PROMPTS {
            match self.game.prompt() {
                Prompt::Command | Prompt::End => return,
                Prompt::Confirm { .. } => self.send("(yes)", Input::Confirm(true)),
                Prompt::Continue => self.send("(continue)", Input::Continue),
                Prompt::Text { .. } => self.fail("a text prompt in the middle of the walk"),
                Prompt::Choice(choice) => {
                    let ids: Vec<(&str, bool)> = choice
                        .options
                        .iter()
                        .map(|o| (o.id.as_str(), o.available.is_ok()))
                        .collect();
                    let answer = Self::pick(mv, &ids);
                    self.send(&format!("(menu {answer})"), Input::Line(answer));
                }
            }
        }
        self.fail("a menu that does not end");
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

    // ----------------------------------------------------------------- one step

    fn send(&mut self, what: &str, input: Input) {
        self.recent.push_back(what.to_owned());
        if self.recent.len() > 30 {
            self.recent.pop_front();
        }
        self.script.push(match &input {
            Input::Line(line) => line.clone(),
            Input::Confirm(true) => "y".to_owned(),
            Input::Confirm(false) => "n".to_owned(),
            Input::Continue => String::new(),
            Input::Cancel => "skip".to_owned(),
            Input::Choice(_) | Input::Eof => unreachable!("the walk never sends {input:?}"),
        });
        let step = self.game.handle(input);
        self.steps += 1;
        self.observe(what, &step);
    }

    /// Everything checked about a step.
    fn observe(&mut self, what: &str, step: &Step) {
        self.hash = fnv(self.hash, format!("{step:?}").as_bytes());
        if step.prompt != self.game.prompt() {
            self.fail(&format!("`{what}`: the step's prompt is not the game's"));
        }
        // Every prompt has a way forward: a menu has an entry that can be taken, or a way out.
        if let Prompt::Choice(choice) = &step.prompt
            && choice.cancel.is_none()
            && choice
                .options
                .iter()
                .all(|option| option.available.is_err())
        {
            self.fail(&format!("`{what}`: a menu with no way forward"));
        }
        self.last_said.clear();
        self.render_all(what, step);
        self.check_drafts(what, step);
        self.tally(what, step);
        self.last_save = step.save;
        match step.save {
            Some(SaveRequest::Checkpoint) => {
                self.checkpoints += 1;
                self.round_trip(what, self.reload != Reload::Never);
            }
            Some(SaveRequest::Slot(_)) => self.round_trip(what, self.reload != Reload::Never),
            Some(SaveRequest::Autosave) => {
                self.round_trip(what, self.reload == Reload::EverySave);
            }
            None => {}
        }
    }

    /// Both languages render the step without a missing key or an argument left over.
    fn render_all(&mut self, what: &str, step: &Step) {
        for (lang, catalog) in catalogs() {
            for mode in &self.modes {
                let mut said = String::new();
                for event in &step.events {
                    said.push_str(&render_event(event, catalog, *mode));
                }
                said.push_str(&render_prompt(&step.prompt, catalog, *mode));
                // The side panel reads the view: the objectives and the gauge's band.
                let view = self.game.view();
                for text in view
                    .objectives
                    .iter()
                    .chain(view.gauges.iter().map(|gauge| &gauge.band))
                {
                    said.push_str(&render(text, catalog, *mode));
                }
                if said.contains("<missing:") || said.contains("<?") {
                    self.fail(&format!(
                        "`{what}` in {lang:?} {mode:?} has a missing key or argument:\n{said}"
                    ));
                }
                if *lang == Lang::En && *mode == RenderMode::FULL {
                    self.last_said = said;
                }
            }
        }
    }

    /// The texts of chapters 3 to 6 are still `TODO <key>` drafts, and the walk is allowed to
    /// say them; every other text it needs is written: the interface, the opening, and the
    /// quests of the first two chapters.
    fn check_drafts(&self, what: &str, step: &Step) {
        let mut keys = Vec::new();
        for text in step_texts(step) {
            collect_keys(text, &mut keys);
        }
        for key in keys {
            let narrative = NARRATIVE.iter().any(|prefix| key.starts_with(prefix));
            let chapter_one_or_two = ["quest.m01.", "quest.m02.", "quest.m03.", "quest.m04."]
                .iter()
                .any(|prefix| key.starts_with(prefix));
            if narrative && !chapter_one_or_two {
                continue;
            }
            for (lang, catalog) in catalogs() {
                // The key alone: its own template, without what its arguments bring.
                let own = render(&Text::dynamic(key.clone()), catalog, RenderMode::FULL);
                if own.starts_with("TODO ") {
                    self.fail(&format!("`{what}`: `{key}` is still a draft in {lang:?}"));
                }
            }
        }
    }

    /// Reads what the step said: errors are failures, the rest is counted.
    fn tally(&mut self, what: &str, step: &Step) {
        for event in &step.events {
            let Event::Message { role, text, .. } = event else {
                continue;
            };
            let key = text.key.as_ref();
            match role {
                Role::Error => self.fail(&format!("`{what}` was refused: {key}")),
                Role::Reward if key == "campaign.out.credits" => {
                    if let Some((_, Arg::Int(amount))) =
                        text.args.iter().find(|(name, _)| *name == "amount")
                    {
                        self.rewarded += amount;
                    }
                }
                _ => {}
            }
            if matches!(
                key,
                "campaign.out.completed_quest" | "campaign.out.completed_contract"
            ) && let Some(title) = nested_key(text, "title")
            {
                *self.completed.entry(title).or_insert(0) += 1;
            }
            if key.starts_with("ending.") && key.rsplit('.').next() == Some("title") {
                self.endings.push(key.to_owned());
            }
            if key.starts_with("epilogue.") {
                self.epilogue.push(key.to_owned());
            }
        }
    }

    /// The game is saved and loaded back: the copy must be the same game, and (when the walk
    /// asks for it) is the one that plays on.
    fn round_trip(&mut self, what: &str, swap: bool) {
        let text = self.game.snapshot().unwrap();
        let loaded = CampaignGame::from_save(&text)
            .unwrap_or_else(|e| self.fail(&format!("`{what}`: the save does not load: {e}")));
        if loaded.snapshot().unwrap() != text {
            self.fail(&format!("`{what}`: a loaded game saves differently"));
        }
        // A game that is over is not saved as over: a loaded game is never finished.
        let over = self.game.prompt() == Prompt::End;
        if (!over && loaded.prompt() != self.game.prompt()) || loaded.view() != self.game.view() {
            self.fail(&format!("`{what}`: a loaded game shows something else"));
        }
        if swap && !over {
            self.game = loaded;
            self.reloads += 1;
        }
    }

    /// Stops the walk with the plan, the last things typed and the last thing said.
    fn fail(&self, why: &str) -> ! {
        let mut tail = String::new();
        for line in &self.recent {
            let _ = writeln!(tail, "    {line}");
        }
        let pending = Brain::new(&self.game, &self.plan).describe_pending();
        panic!(
            "plan [{}]: {why}\n  last inputs:\n{tail}  last said:\n{}\n  state:\n{pending}",
            self.plan.name(),
            self.last_said
        );
    }
}

/// The families of keys that carry the story (`narrative_prefixes` of the glossary): their
/// texts are the ones still being written.
const NARRATIVE: &[&str] = &[
    "quest.",
    "contact.",
    "frag.",
    "doc.",
    "mail.",
    "cutscene.",
    "decision.",
    "ending.",
    "epilogue.",
    "node.",
    "file.",
    "item.",
];

/// Every text of a step: its events and its prompt.
fn step_texts(step: &Step) -> Vec<&Text> {
    let mut texts = Vec::new();
    for event in &step.events {
        match event {
            Event::Message { text, .. } => texts.push(text),
            Event::Screen(table) => {
                texts.push(&table.title);
                texts.extend(&table.columns);
                texts.extend(table.rows.iter().flatten());
            }
            Event::Changed { band, .. } => texts.push(band),
            Event::Decor { alt, .. } => texts.push(alt),
            Event::Break => {}
        }
    }
    match &step.prompt {
        Prompt::Text { label, .. } => texts.push(label),
        Prompt::Confirm { question, .. } => texts.push(question),
        Prompt::Choice(choice) => {
            texts.push(&choice.title);
            for option in &choice.options {
                texts.push(&option.label);
                if let Err(reason) = &option.available {
                    texts.push(reason);
                }
            }
            texts.extend(&choice.cancel);
        }
        Prompt::Command | Prompt::Continue | Prompt::End => {}
    }
    texts
}

/// The key of a text and of every text nested in its arguments.
fn collect_keys(text: &Text, keys: &mut Vec<String>) {
    keys.push(text.key.to_string());
    for (_, arg) in &text.args {
        if let Arg::Text(inner) = arg {
            collect_keys(inner, keys);
        }
    }
}

/// The key of a text nested in an argument.
fn nested_key(text: &Text, name: &str) -> Option<String> {
    text.args.iter().find_map(|(arg, value)| match value {
        Arg::Text(inner) if *arg == name => Some(inner.key.to_string()),
        _ => None,
    })
}

/// An event as text, in a language and a mode.
fn render_event(event: &Event, catalog: &Catalog, mode: RenderMode) -> String {
    match event {
        Event::Message { text, .. } => format!("{}\n", render(text, catalog, mode)),
        Event::Screen(table) => {
            let mut out = format!("{}\n", render(&table.title, catalog, mode));
            for column in &table.columns {
                out.push_str(&render(column, catalog, mode));
                out.push('|');
            }
            for row in &table.rows {
                for cell in row {
                    out.push_str(&render(cell, catalog, mode));
                    out.push('|');
                }
                out.push('\n');
            }
            out
        }
        Event::Changed { band, .. } => format!("{}\n", render(band, catalog, mode)),
        Event::Decor { alt, .. } => format!("{}\n", render(alt, catalog, mode)),
        Event::Break => "\n".to_owned(),
    }
}

/// A prompt as text: the question, the entries of a menu and the reasons they are closed.
fn render_prompt(prompt: &Prompt, catalog: &Catalog, mode: RenderMode) -> String {
    match prompt {
        Prompt::Command | Prompt::Continue | Prompt::End => String::new(),
        Prompt::Text { label, .. } => render(label, catalog, mode),
        Prompt::Confirm { question, .. } => render(question, catalog, mode),
        Prompt::Choice(choice) => {
            let mut out = render(&choice.title, catalog, mode);
            for option in &choice.options {
                out.push('\n');
                out.push_str(&render(&option.label, catalog, mode));
                if let Err(reason) = &option.available {
                    out.push_str(&render(reason, catalog, mode));
                }
            }
            if let Some(cancel) = &choice.cancel {
                out.push('\n');
                out.push_str(&render(cancel, catalog, mode));
            }
            out
        }
    }
}

/// The ending expected for a plan, from the rules of the game's story (decision D3, the echo
/// choice and the advantage of the echo log).
pub(crate) fn expected_ending(plan: &Plan) -> &'static str {
    let choice = |decision: &str| {
        plan.choices
            .iter()
            .find(|(d, _)| d.as_str() == decision)
            .map(|(_, c)| c.as_str())
    };
    match choice("d3") {
        Some("liberate") if matches!(choice("echo"), Some("wait" | "believe")) => "e1a",
        Some("liberate") => "e1b",
        Some("entrust") => "e2",
        Some("burn") => "e3",
        _ => "e4",
    }
}

/// The contacts that have an epilogue line, from the content.
pub(crate) fn epilogue_contacts(c: &Content) -> BTreeSet<String> {
    c.epilogue
        .iter()
        .map(|line| line.contact.to_string())
        .collect()
}
