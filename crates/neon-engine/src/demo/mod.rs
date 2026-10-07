//! A tiny complete game that exercises the whole engine contract.
//!
//! It has a prologue (continue, free text, confirmation), commands, a nested menu (the
//! stall), alerts, rewards, a gauge and a clean end. It is not the real game: it exists so
//! that both frontends and the tests talk to a real engine while the real one is designed.

mod text;

pub use text::{catalog_en, catalog_fr};

use crate::event::{Event, Gauge, Severity, Table};
use crate::game::{Game, GaugeReading, View};
use crate::ids::ContactId;
use crate::prompt::{Choice, ChoiceOption, Input, Prompt, Resolution, Step};
use crate::rng::Pcg32;
use crate::text::Text;

const DEFAULT_NAME: &str = "Case";
const NAME_MAX_CHARS: usize = 20;
const TRACE_MAX: i32 = 100;
const TRACE_TENSE: i32 = 30;
const TRACE_CRITICAL: i32 = 70;
const STARTING_CREDITS: i32 = 50;
const COMMANDS: [&str; 5] = ["help", "quit", "scan", "shop", "status"];
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
        label_key: "demo.item.proxy",
        name_key: "demo.item.proxy.name",
        price: 30,
    },
    ItemDef {
        id: "cloak",
        label_key: "demo.item.cloak",
        name_key: "demo.item.cloak.name",
        price: 60,
    },
    ItemDef {
        id: "deck",
        label_key: "demo.item.deck",
        name_key: "demo.item.deck.name",
        price: 200,
    },
];

/// A step of the game that needs its own prompt. The stack of flows is what lets menus
/// nest, be cloned and be saved in the middle of a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Flow {
    Intro,
    AskName,
    ConfirmName(String),
    Shop,
    ConfirmQuit,
}

/// The demo game.
#[derive(Debug, Clone)]
pub struct DemoGame {
    rng: Pcg32,
    name: String,
    trace: i32,
    credits: i32,
    owned: Vec<&'static str>,
    flows: Vec<Flow>,
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
            owned: Vec::new(),
            flows: vec![Flow::Intro],
            over: false,
        }
    }

    /// Builds a step whose prompt is derived from the state, so it can never disagree
    /// with [`Game::prompt`].
    fn step(&self, events: Vec<Event>, save_requested: bool) -> Step {
        Step {
            events,
            prompt: self.prompt(),
            save_requested,
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
                self.flows.push(Flow::ConfirmName(clean_name(line)));
            }
            Input::Cancel => {
                self.flows.pop();
                self.flows.push(Flow::ConfirmName(DEFAULT_NAME.to_owned()));
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

    /// Returns whether the game should be saved.
    fn command(&mut self, input: &Input, events: &mut Vec<Event>) -> bool {
        match input {
            Input::Line(line) => self.run_command(line.trim(), events),
            Input::Cancel => false,
            _ => {
                invalid(events);
                false
            }
        }
    }

    fn run_command(&mut self, line: &str, events: &mut Vec<Event>) -> bool {
        let word = line
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match word.as_str() {
            "" => false,
            "help" => {
                events.push(help_table());
                false
            }
            "status" => {
                self.status(events);
                false
            }
            "scan" => {
                self.scan(events);
                true
            }
            "shop" => {
                self.flows.push(Flow::Shop);
                false
            }
            "quit" => {
                self.flows.push(Flow::ConfirmQuit);
                false
            }
            other => {
                events.push(Event::error(
                    Text::new("demo.unknown_command").with_str("command", other),
                ));
                false
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

    fn scan(&mut self, events: &mut Vec<Event>) {
        let found = self.roll(5) + 1;
        let credits = found * 2;
        let from = self.trace;
        self.trace = (self.trace + found * 5).min(TRACE_MAX);
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
    }

    fn shop_choice(&self) -> Choice {
        let options = ITEMS
            .iter()
            .map(|item| ChoiceOption {
                id: item.id.to_owned(),
                label: Text::new(item.label_key).with_int("price", i64::from(item.price)),
                available: if self.owned.contains(&item.id) {
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

    /// Returns whether the game should be saved.
    fn shop(&mut self, input: &Input, events: &mut Vec<Event>) -> bool {
        let choice = self.shop_choice();
        match choice.resolve(input) {
            Resolution::Cancel => {
                self.flows.pop();
                false
            }
            Resolution::Invalid => {
                events.push(Event::error(Text::new("ui.invalid_choice")));
                false
            }
            Resolution::Pick(index) => self.buy(&choice, index, events),
        }
    }

    fn buy(&mut self, choice: &Choice, index: usize, events: &mut Vec<Event>) -> bool {
        let (Some(option), Some(item)) = (choice.options.get(index), ITEMS.get(index)) else {
            invalid(events);
            return false;
        };
        if let Err(reason) = &option.available {
            events.push(Event::error(reason.clone()));
            return false;
        }
        self.credits -= item.price;
        self.owned.push(item.id);
        events.push(Event::reward(
            Text::new("demo.shop.bought").with_text("item", Text::new(item.name_key)),
        ));
        if item.id == "proxy" {
            let from = self.trace;
            self.trace = (self.trace - 10).max(0);
            events.push(Event::narration(Text::new("demo.shop.proxy_effect")));
            events.extend(trace_changed(from, self.trace));
        }
        true
    }

    /// Returns whether the game should be saved.
    fn confirm_quit(&mut self, input: &Input, events: &mut Vec<Event>) -> bool {
        match input {
            Input::Confirm(true) => {
                self.over = true;
                events.push(Event::system(
                    Text::new("demo.quit.bye").with_str("name", self.name.clone()),
                ));
                true
            }
            Input::Confirm(false) | Input::Cancel => {
                self.flows.pop();
                false
            }
            _ => {
                invalid(events);
                false
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
        self.step(events, false)
    }

    fn handle(&mut self, input: Input) -> Step {
        if self.over {
            return self.step(Vec::new(), false);
        }
        if input == Input::Eof {
            self.over = true;
            return self.step(vec![Event::system(Text::new("ui.eof"))], false);
        }
        let mut events = Vec::new();
        let mut save = false;
        match self.flows.last().cloned() {
            None => save = self.command(&input, &mut events),
            Some(Flow::Intro) => self.intro(&input, &mut events),
            Some(Flow::AskName) => self.ask_name(&input, &mut events),
            Some(Flow::ConfirmName(name)) => self.confirm_name(name, &input, &mut events),
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
            Some(Flow::ConfirmName(name)) => Prompt::Confirm {
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
                Text::new("demo.objective.trace").with_int("limit", i64::from(TRACE_CRITICAL)),
            ],
        }
    }

    fn complete(&self, line: &str) -> Vec<String> {
        let prefix = line.to_ascii_lowercase();
        match self.prompt() {
            Prompt::Command if !line.contains(char::is_whitespace) => COMMANDS
                .iter()
                .filter(|command| command.starts_with(&prefix))
                .map(|command| (*command).to_owned())
                .collect(),
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
    Event::Screen(Table {
        title: Text::new("demo.help.title"),
        columns: vec![
            Text::new("demo.help.col_command"),
            Text::new("demo.help.col_effect"),
        ],
        rows: COMMANDS
            .iter()
            .map(|command| {
                vec![
                    Text::raw(*command),
                    Text::dynamic(format!("demo.help.{command}")),
                ]
            })
            .collect(),
    })
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
