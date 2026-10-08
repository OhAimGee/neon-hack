//! A tiny complete game that exercises the whole engine contract.
//!
//! It has a prologue (continue, free text, confirmation), commands, a nested menu (the
//! stall), alerts, rewards, a gauge and a clean end. It is not the real game: it exists so
//! that both frontends and the tests talk to a real engine while the real one is designed.

use serde::{Deserialize, Serialize};

use crate::command::{
    ArgKind, ArgRef, ArgSpec, Availability, Capabilities, CommandSpec, Context, HandledBy, Lookup,
    NoLists, Registry,
};
use crate::event::{Event, Gauge, Severity};
use crate::game::{Game, GaugeReading, View};
use crate::ids::ContactId;
use crate::prompt::{Choice, ChoiceOption, Input, Prompt, Resolution, Step};
use crate::rng::Pcg32;
use crate::save::{self, SLOT_COUNT, SaveError, SaveMeta, SaveRequest, SaveState};
use crate::text::Text;

const DEFAULT_NAME: &str = "Case";
const NAME_MAX_CHARS: usize = 20;
const TRACE_MAX: i32 = 100;
const TRACE_TENSE: i32 = 30;
const TRACE_CRITICAL: i32 = 70;
const STARTING_CREDITS: i32 = 50;
/// How much trace `laylow` removes.
const LAY_LOW_COOLING: i32 = 25;
/// The price of the mission item, shown in the objectives.
const DECK_PRICE: i32 = 120;
const MAX_CREDITS: i32 = 1_000_000;
/// `save` reads its slot itself, so that its messages for a bad slot stay the demo's own:
/// the rest of the line is all it asks of the registry.
const SAVE_ARGS: &[ArgSpec] = &[ArgSpec::optional(ArgKind::Path)];
const SPECS: &[CommandSpec] = &[
    CommandSpec {
        name: "help",
        aliases: &["h"],
        help: "demo.help.help",
        context: Context::Anywhere,
        handled_by: HandledBy::Engine,
        args: &[],
    },
    CommandSpec {
        name: "laylow",
        aliases: &[],
        help: "demo.help.laylow",
        context: Context::Anywhere,
        handled_by: HandledBy::Engine,
        args: &[],
    },
    CommandSpec {
        name: "quit",
        aliases: &["exit"],
        help: "demo.help.quit",
        context: Context::Anywhere,
        handled_by: HandledBy::Engine,
        args: &[],
    },
    CommandSpec {
        name: "save",
        aliases: &[],
        help: "demo.help.save",
        context: Context::Anywhere,
        handled_by: HandledBy::Engine,
        args: SAVE_ARGS,
    },
    CommandSpec {
        name: "scan",
        aliases: &[],
        help: "demo.help.scan",
        context: Context::Anywhere,
        handled_by: HandledBy::Engine,
        args: &[],
    },
    CommandSpec {
        name: "shop",
        aliases: &["buy"],
        help: "demo.help.shop",
        context: Context::Anywhere,
        handled_by: HandledBy::Engine,
        args: &[],
    },
    CommandSpec {
        name: "status",
        aliases: &["st"],
        help: "demo.help.status",
        context: Context::Anywhere,
        handled_by: HandledBy::Engine,
        args: &[],
    },
];
/// Every command of the demo: dispatch, help and completion read this one table.
const COMMANDS: Registry = Registry::new(SPECS);

/// The demo is all at the hub, and has no command that depends on the frontend.
const CONTEXT: Context = Context::Hub;
const FRONTEND: Capabilities = Capabilities::PLAIN;

/// The demo has no locked command: all of them are open all the time, in every context.
fn open(_: &CommandSpec) -> Availability {
    Availability::Open
}
const BANNER: &[&str] = &["== N E O N   H A C K =="];

struct ItemDef {
    id: &'static str,
    label_key: &'static str,
    name_key: &'static str,
    price: i32,
}

const ITEMS: [ItemDef; 3] = [
    ItemDef {
        id: "proxy",
        label_key: "demo.item.proxy.label",
        name_key: "demo.item.proxy.name",
        price: 30,
    },
    ItemDef {
        id: "cloak",
        label_key: "demo.item.cloak.label",
        name_key: "demo.item.cloak.name",
        price: 60,
    },
    ItemDef {
        id: "deck",
        label_key: "demo.item.deck.label",
        name_key: "demo.item.deck.name",
        price: DECK_PRICE,
    },
];

/// A step of the game that needs its own prompt. The stack of flows is what lets menus
/// nest, be cloned and be saved in the middle of a menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Flow {
    Intro,
    AskName,
    ConfirmName { name: String },
    Shop,
    ConfirmQuit,
}

/// The demo game. Its fields are its save file; `over` is not saved, because a game that
/// is loaded is never finished.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoGame {
    rng: Pcg32,
    name: String,
    trace: i32,
    credits: i32,
    turn: u32,
    owned: Vec<String>,
    flows: Vec<Flow>,
    #[serde(skip)]
    over: bool,
}

impl DemoGame {
    /// A new game. The same seed always plays the same way.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Pcg32::new(seed, 54),
            name: DEFAULT_NAME.to_owned(),
            trace: 0,
            credits: STARTING_CREDITS,
            turn: 0,
            owned: Vec::new(),
            flows: vec![Flow::Intro],
            over: false,
        }
    }

    /// Loads a game from save text.
    ///
    /// # Errors
    ///
    /// [`SaveError`] when the text is damaged, from a newer game, or not a state the demo
    /// can be in. A failed load changes nothing: this builds a new game.
    pub fn from_save(text: &str) -> Result<Self, SaveError> {
        save::decode::<Self>(text).map(|(_, game)| game)
    }

    /// Builds a step whose prompt is derived from the state, so it can never disagree
    /// with [`Game::prompt`].
    fn step(&self, events: Vec<Event>, save: Option<SaveRequest>) -> Step {
        Step {
            events,
            prompt: self.prompt(),
            save,
        }
    }

    fn roll(&mut self, bound: u32) -> i32 {
        self.rng
            .next_below(bound)
            .and_then(|value| i32::try_from(value).ok())
            .unwrap_or(0)
    }

    fn intro(&mut self, input: &Input, events: &mut Vec<Event>) {
        if *input == Input::Continue {
            self.flows.pop();
            self.flows.push(Flow::AskName);
            events.push(Event::Break);
        } else {
            invalid(events);
        }
    }

    fn ask_name(&mut self, input: &Input, events: &mut Vec<Event>) {
        match input {
            Input::Line(line) => {
                self.flows.pop();
                self.flows.push(Flow::ConfirmName {
                    name: clean_name(line),
                });
            }
            Input::Cancel => {
                self.flows.pop();
                self.flows.push(Flow::ConfirmName {
                    name: DEFAULT_NAME.to_owned(),
                });
            }
            _ => invalid(events),
        }
    }

    fn confirm_name(&mut self, name: String, input: &Input, events: &mut Vec<Event>) {
        match input {
            Input::Confirm(true) => {
                self.flows.pop();
                events.push(Event::say(
                    echo7(),
                    Text::new("demo.echo.greeting").with_str("name", name.clone()),
                ));
                events.push(Event::say(echo7(), Text::new("demo.echo.mission")));
                events.push(Event::say(echo7(), Text::new("demo.echo.ready")));
                self.name = name;
            }
            Input::Confirm(false) | Input::Cancel => {
                self.flows.pop();
                self.flows.push(Flow::AskName);
            }
            _ => invalid(events),
        }
    }

    /// Returns the save the command asks for, if any.
    fn command(&mut self, input: &Input, events: &mut Vec<Event>) -> Option<SaveRequest> {
        match input {
            Input::Line(line) => self.run_command(line.trim(), events),
            Input::Cancel => None,
            _ => {
                invalid(events);
                None
            }
        }
    }

    fn run_command(&mut self, line: &str, events: &mut Vec<Event>) -> Option<SaveRequest> {
        let lookup = COMMANDS.parse(line, CONTEXT, FRONTEND, open, &NoLists);
        let (spec, args) = match lookup {
            Lookup::Empty => return None,
            Lookup::Found { spec, args } => (spec, args),
            Lookup::Locked { reason, .. } | Lookup::WrongContext { reason, .. } => {
                events.push(Event::error(reason));
                return None;
            }
            Lookup::BadArg { error, .. } => {
                events.push(Event::error(error.text()));
                return None;
            }
            Lookup::Unknown(word) => {
                events.push(unknown_command(&word));
                return None;
            }
        };
        self.turn = self.turn.saturating_add(1);
        match spec.name {
            "help" => {
                events.push(help_table());
                None
            }
            "status" => {
                self.status(events);
                None
            }
            "save" => save_command(
                args.first()
                    .and_then(ArgRef::as_path)
                    .and_then(|path| path.split_whitespace().next()),
                events,
            ),
            "scan" => self.scan(events),
            "laylow" => {
                self.lay_low(events);
                Some(SaveRequest::Autosave)
            }
            "shop" => {
                self.flows.push(Flow::Shop);
                // Buying is permanent: the player can come back to the moment before.
                Some(SaveRequest::Checkpoint)
            }
            "quit" => {
                self.flows.push(Flow::ConfirmQuit);
                None
            }
            // A command declared in `SPECS` without a handler here: a test runs them all.
            other => {
                events.push(unknown_command(other));
                None
            }
        }
    }

    fn status(&self, events: &mut Vec<Event>) {
        events.push(Event::system(
            Text::new("demo.status.name").with_str("name", self.name.clone()),
        ));
        events.push(Event::system(
            Text::new("demo.status.credits").with_int("credits", i64::from(self.credits)),
        ));
        events.push(Event::system(
            Text::new("demo.status.trace")
                .with_int("value", i64::from(self.trace))
                .with_int("max", i64::from(TRACE_MAX))
                .with_text("band", band(self.trace)),
        ));
    }

    fn owns(&self, item: &str) -> bool {
        self.owned.iter().any(|owned| owned == item)
    }

    /// Scans for ports. Returns the save it asks for: none when the scan ends the game.
    fn scan(&mut self, events: &mut Vec<Event>) -> Option<SaveRequest> {
        let found = self.roll(5) + 1;
        let credits = found * 2;
        // The cloak module halves the trace a scan leaves (rounded up).
        let mut gain = found * 5;
        if self.owns("cloak") {
            gain = (gain + 1) / 2;
        }
        let from = self.trace;
        self.trace = (self.trace + gain).min(TRACE_MAX);
        self.credits += credits;
        events.push(Event::system(
            Text::new("demo.scan.found").with_int("found", i64::from(found)),
        ));
        events.push(Event::reward(
            Text::new("demo.scan.reward").with_int("credits", i64::from(credits)),
        ));
        events.extend(trace_changed(from, self.trace));
        if from < TRACE_TENSE && self.trace >= TRACE_TENSE {
            events.push(Event::alert(
                Severity::Warning,
                Text::new("demo.alert.tense"),
            ));
        }
        if from < TRACE_CRITICAL && self.trace >= TRACE_CRITICAL {
            events.push(Event::alert(
                Severity::Danger,
                Text::new("demo.alert.critical"),
            ));
        }
        if self.trace >= TRACE_MAX {
            self.lose(events);
            // The last save is kept: the game resumes from before the fatal scan.
            return None;
        }
        Some(SaveRequest::Autosave)
    }

    /// Lies low: the trace cools down.
    fn lay_low(&mut self, events: &mut Vec<Event>) {
        let from = self.trace;
        self.trace = (self.trace - LAY_LOW_COOLING).max(0);
        events.push(Event::narration(Text::new("demo.laylow.done")));
        events.extend(trace_changed(from, self.trace));
    }

    /// The mission is done: the deck is bought.
    fn win(&mut self, events: &mut Vec<Event>) {
        self.over = true;
        events.push(Event::say(
            echo7(),
            Text::new("demo.end.win.echo").with_str("name", self.name.clone()),
        ));
        events.push(Event::reward(
            Text::new("demo.end.win.summary")
                .with_int("turns", i64::from(self.turn))
                .with_int("credits", i64::from(self.credits)),
        ));
    }

    /// The trace is full: the game is lost.
    fn lose(&mut self, events: &mut Vec<Event>) {
        self.over = true;
        events.push(Event::alert(
            Severity::Danger,
            Text::new("demo.end.lose.caught"),
        ));
        events.push(Event::system(Text::new("demo.end.lose.hint")));
    }

    fn shop_choice(&self) -> Choice {
        let options = ITEMS
            .iter()
            .map(|item| ChoiceOption {
                id: item.id.to_owned(),
                label: Text::new(item.label_key).with_int("price", i64::from(item.price)),
                available: if self.owns(item.id) {
                    Err(Text::new("demo.shop.owned"))
                } else if self.credits < item.price {
                    Err(Text::new("demo.shop.cannot_afford")
                        .with_int("missing", i64::from(item.price - self.credits)))
                } else {
                    Ok(())
                },
            })
            .collect();
        Choice {
            title: Text::new("demo.shop.title"),
            options,
            cancel: Some(Text::new("demo.shop.cancel")),
        }
    }

    /// Returns the save the input asks for, if any.
    fn shop(&mut self, input: &Input, events: &mut Vec<Event>) -> Option<SaveRequest> {
        let choice = self.shop_choice();
        match choice.resolve(input) {
            Resolution::Cancel => {
                self.flows.pop();
                None
            }
            Resolution::Invalid => {
                events.push(Event::error(Text::new("ui.invalid_choice")));
                None
            }
            Resolution::Pick(index) => self.buy(&choice, index, events),
        }
    }

    fn buy(
        &mut self,
        choice: &Choice,
        index: usize,
        events: &mut Vec<Event>,
    ) -> Option<SaveRequest> {
        let (Some(option), Some(item)) = (choice.options.get(index), ITEMS.get(index)) else {
            invalid(events);
            return None;
        };
        if let Err(reason) = &option.available {
            events.push(Event::error(reason.clone()));
            return None;
        }
        self.credits -= item.price;
        self.owned.push(item.id.to_owned());
        events.push(Event::reward(
            Text::new("demo.shop.bought").with_text("item", Text::new(item.name_key)),
        ));
        if item.id == "deck" {
            // The mission item: buying it ends the game, and the last save is kept.
            self.win(events);
            return None;
        }
        if item.id == "proxy" {
            let from = self.trace;
            self.trace = (self.trace - 10).max(0);
            events.push(Event::narration(Text::new("demo.shop.proxy_effect")));
            events.extend(trace_changed(from, self.trace));
        }
        Some(SaveRequest::Autosave)
    }

    /// Leaving saves the game at the command line, so the next launch picks up from there.
    fn confirm_quit(&mut self, input: &Input, events: &mut Vec<Event>) -> Option<SaveRequest> {
        match input {
            Input::Confirm(true) => {
                self.flows.pop();
                self.over = true;
                events.push(Event::system(
                    Text::new("demo.quit.bye").with_str("name", self.name.clone()),
                ));
                Some(SaveRequest::Autosave)
            }
            Input::Confirm(false) | Input::Cancel => {
                self.flows.pop();
                None
            }
            _ => {
                invalid(events);
                None
            }
        }
    }
}

impl Game for DemoGame {
    fn start(&mut self) -> Step {
        let events = vec![
            Event::Decor {
                art: BANNER,
                alt: Text::new("demo.banner.alt"),
            },
            Event::flavor(Text::new("demo.intro.1")),
            Event::narration(Text::new("demo.intro.2")),
        ];
        self.step(events, None)
    }

    fn handle(&mut self, input: Input) -> Step {
        if self.over {
            return self.step(Vec::new(), None);
        }
        if input == Input::Eof {
            self.over = true;
            return self.step(vec![Event::system(Text::new("ui.eof"))], None);
        }
        let mut events = Vec::new();
        let mut save = None;
        match self.flows.last().cloned() {
            None => save = self.command(&input, &mut events),
            Some(Flow::Intro) => self.intro(&input, &mut events),
            Some(Flow::AskName) => self.ask_name(&input, &mut events),
            Some(Flow::ConfirmName { name }) => self.confirm_name(name, &input, &mut events),
            Some(Flow::Shop) => save = self.shop(&input, &mut events),
            Some(Flow::ConfirmQuit) => save = self.confirm_quit(&input, &mut events),
        }
        self.step(events, save)
    }

    fn prompt(&self) -> Prompt {
        if self.over {
            return Prompt::End;
        }
        match self.flows.last() {
            None => Prompt::Command,
            Some(Flow::Intro) => Prompt::Continue,
            Some(Flow::AskName) => Prompt::Text {
                label: Text::new("demo.prompt.name"),
                max_chars: NAME_MAX_CHARS,
                default: Some(DEFAULT_NAME.to_owned()),
            },
            Some(Flow::ConfirmName { name }) => Prompt::Confirm {
                question: Text::new("demo.prompt.confirm_name").with_str("name", name.clone()),
                default: true,
            },
            Some(Flow::Shop) => Prompt::Choice(self.shop_choice()),
            Some(Flow::ConfirmQuit) => Prompt::Confirm {
                question: Text::new("demo.quit.confirm"),
                default: false,
            },
        }
    }

    fn snapshot(&self) -> Result<String, SaveError> {
        let meta = SaveMeta {
            player: self.name.clone(),
            turn: self.turn,
        };
        save::encode(&meta, self)
    }

    fn view(&self) -> View {
        View {
            player: self.name.clone(),
            gauges: vec![GaugeReading {
                gauge: Gauge::Trace,
                value: self.trace,
                max: TRACE_MAX,
                band: band(self.trace),
            }],
            objectives: vec![
                Text::new("demo.objective.deck").with_int("price", i64::from(DECK_PRICE)),
                Text::new("demo.objective.trace").with_int("limit", i64::from(TRACE_MAX)),
            ],
        }
    }

    fn complete(&self, line: &str) -> Vec<String> {
        let prefix = line.to_ascii_lowercase();
        match self.prompt() {
            Prompt::Command if !line.contains(char::is_whitespace) => {
                COMMANDS.complete(line, CONTEXT, FRONTEND, open)
            }
            Prompt::Choice(choice) => choice
                .options
                .into_iter()
                .map(|option| option.id)
                .filter(|id| id.starts_with(&prefix))
                .collect(),
            _ => Vec::new(),
        }
    }
}

impl SaveState for DemoGame {
    fn validate(&self) -> Result<(), String> {
        if !self.rng.is_valid() {
            return Err("the random generator state is not one the game can produce".to_owned());
        }
        if !(0..=TRACE_MAX).contains(&self.trace) {
            return Err(format!("trace {} is out of range", self.trace));
        }
        if !(0..=MAX_CREDITS).contains(&self.credits) {
            return Err(format!("credits {} are out of range", self.credits));
        }
        if clean_name(&self.name) != self.name {
            return Err("the player name is not a clean name".to_owned());
        }
        // Buying the deck ends the game and is never saved: a save that owns it could not
        // finish the mission, so it is not a state the game can be in.
        if self.owns("deck") {
            return Err("the deck is the mission item: owning it means the game is won".to_owned());
        }
        for (index, owned) in self.owned.iter().enumerate() {
            if !ITEMS.iter().any(|item| item.id == owned) {
                return Err(format!("unknown item `{owned}`"));
            }
            if self
                .owned
                .iter()
                .take(index)
                .any(|earlier| earlier == owned)
            {
                return Err(format!("item `{owned}` is owned twice"));
            }
        }
        // Every menu is opened from the command line, so at most one flow is ever stacked.
        match self.flows.as_slice() {
            [] | [Flow::Intro | Flow::AskName | Flow::Shop | Flow::ConfirmQuit] => Ok(()),
            [Flow::ConfirmName { name }] if clean_name(name) == *name => Ok(()),
            other => Err(format!(
                "{} stacked flows do not make a state of the game",
                other.len()
            )),
        }
    }
}

/// `save` or `save N`: a manual slot, 1 by default.
fn save_command(argument: Option<&str>, events: &mut Vec<Event>) -> Option<SaveRequest> {
    let slot = match argument {
        None => Some(1),
        Some(text) => text
            .parse::<u8>()
            .ok()
            .filter(|slot| (1..=SLOT_COUNT).contains(slot)),
    };
    if slot.is_none() {
        events.push(Event::error(
            Text::new("demo.save.bad_slot").with_int("max", i64::from(SLOT_COUNT)),
        ));
    }
    slot.map(SaveRequest::Slot)
}

fn invalid(events: &mut Vec<Event>) {
    events.push(Event::error(Text::new("ui.invalid_input")));
}

fn echo7() -> ContactId {
    match ContactId::new("echo7") {
        Ok(id) => id,
        Err(error) => unreachable!("the static id `echo7` is valid: {error}"),
    }
}

fn band(trace: i32) -> Text {
    if trace >= TRACE_CRITICAL {
        Text::new("band.critical")
    } else if trace >= TRACE_TENSE {
        Text::new("band.tense")
    } else {
        Text::new("band.calm")
    }
}

/// The event for a trace that moved; nothing when it did not (already at an end of its range).
fn trace_changed(from: i32, to: i32) -> Option<Event> {
    (from != to).then(|| Event::Changed {
        gauge: Gauge::Trace,
        from,
        to,
        band: band(to),
    })
}

fn help_table() -> Event {
    COMMANDS.help(
        Text::new("demo.help.title"),
        &[
            Text::new("demo.help.col_command"),
            Text::new("demo.help.col_effect"),
        ],
        CONTEXT,
        FRONTEND,
        open,
    )
}

fn unknown_command(word: &str) -> Event {
    Event::error(Text::new("demo.unknown_command").with_str("command", word))
}

/// A typed name: control characters removed, trimmed, cut to 20 characters, never empty.
fn clean_name(raw: &str) -> String {
    let cleaned: String = raw.chars().filter(|c| !c.is_control()).collect();
    let name: String = cleaned.trim().chars().take(NAME_MAX_CHARS).collect();
    if name.is_empty() {
        DEFAULT_NAME.to_owned()
    } else {
        name
    }
}

#[cfg(test)]
mod tests;
