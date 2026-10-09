//! The guided tutorial: ECHO-7 walks a new player through the commands that are open at the
//! start, in the order the hub offers them.
//!
//! The tutorial is a small state machine in the saved state ([`Tutorial`]), not a script:
//!
//! * a step is a command to learn ([`STEPS`]); it is **done** when the player runs that
//!   command, whenever they do (a step done ahead of its turn is simply skipped over);
//! * the **current** step is the first one not done; its hint is said once, at the command
//!   prompt (never in the middle of a menu), as soon as its command is open, so that the
//!   shop step waits for R4Z0R by the very rule that closes `shop` (`unlocks.toml`);
//! * the last step is a **closing** one: saying it ends the tutorial ([`Mode::Finished`]);
//! * the player can end it at any time (`tutorial skip`, or by declining the offer that
//!   opens it) and restart it (`tutorial restart`). It never blocks anything: it only talks.
//!
//! A save without the tutorial (every save that predates it) reads as [`Mode::Off`].

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::command::{ArgRef, Availability};
use crate::event::Event;
use crate::prompt::Input;
use crate::text::Text;

use super::CampaignGame;
use super::commands::SPECS;
use super::dispatch::invalid;
use super::keys;

/// What completes a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Completion {
    /// The player ran the command: it was open and its arguments were understood.
    Command,
    /// The game says so, when the command did what the step teaches (an intrusion that took
    /// place, not one that was declined).
    Outcome,
    /// The step teaches and nothing is asked: saying it ends the tutorial.
    Closing,
}

/// One lesson of the tutorial.
#[derive(Debug, Clone, Copy)]
pub(super) struct Step {
    /// Stable id, saved with the progress; also the middle of its text keys
    /// (`tutorial.step.<id>.say` and `.do`).
    pub(super) id: &'static str,
    /// The command it teaches: the step waits until it is open.
    pub(super) command: &'static str,
    pub(super) completion: Completion,
}

const fn step(id: &'static str, command: &'static str, completion: Completion) -> Step {
    Step {
        id,
        command,
        completion,
    }
}

/// The lessons, in teaching order. The first seven are what the first quest asks for; the
/// stall opens with its end, and the last two are what a player needs to keep a game.
pub(super) const STEPS: &[Step] = &[
    step("help", "help", Completion::Command),
    step("status", "status", Completion::Command),
    step("quests", "quests", Completion::Command),
    step("talk", "talk", Completion::Command),
    step("net", "net", Completion::Command),
    step("hack", "hack", Completion::Outcome),
    step("laylow", "laylow", Completion::Command),
    step("shop", "shop", Completion::Command),
    step("buy", "buy", Completion::Command),
    step("save", "save", Completion::Command),
    step("quit", "quit", Completion::Closing),
];

/// Where the tutorial stands.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Mode {
    /// Not running: declined, skipped, or never offered (a save that predates it).
    #[default]
    Off,
    /// Running.
    Active,
    /// Every lesson was given.
    Finished,
}

/// The saved state of the tutorial.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Tutorial {
    #[serde(default)]
    mode: Mode,
    /// The ids of the steps done, in no particular order (a set).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    done: BTreeSet<String>,
    /// The step whose hint was said last, so that it is never said twice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    shown: Option<String>,
}

impl Tutorial {
    /// Nothing to save: the default state, which is also that of a game that never had one.
    pub(super) fn is_off(&self) -> bool {
        *self == Self::default()
    }

    pub(super) fn mode(&self) -> Mode {
        self.mode
    }

    /// The ids of the steps done.
    #[cfg(test)]
    pub(super) fn done(&self) -> &BTreeSet<String> {
        &self.done
    }

    /// Starts (or restarts) from the first step.
    pub(super) fn begin(&mut self) {
        *self = Self {
            mode: Mode::Active,
            ..Self::default()
        };
    }

    /// Ends it and forgets the progress.
    pub(super) fn stop(&mut self) {
        *self = Self::default();
    }

    /// The first step not done, with its position. The closing step is never done, so there is
    /// always one.
    fn current(&self) -> Option<(usize, &'static Step)> {
        STEPS
            .iter()
            .enumerate()
            .find(|(_, step)| !self.done.contains(step.id))
    }

    /// The player did `command`: marks the steps it teaches, if the tutorial runs.
    pub(super) fn observe(&mut self, command: &str, how: Completion) {
        if self.mode != Mode::Active {
            return;
        }
        for step in STEPS {
            if step.command == command && step.completion == how {
                self.done.insert(step.id.to_owned());
            }
        }
    }

    /// Rejects a state that no play can reach.
    pub(super) fn validate(&self) -> Result<(), String> {
        let lesson = |id: &str| STEPS.iter().find(|step| step.id == id);
        for id in &self.done {
            if lesson(id).is_none_or(|step| step.completion == Completion::Closing) {
                return Err(format!("`{id}` is not a tutorial step that can be done"));
            }
        }
        if let Some(id) = &self.shown
            && lesson(id).is_none()
        {
            return Err(format!("`{id}` is not a tutorial step"));
        }
        let given = STEPS
            .iter()
            .filter(|step| step.completion != Completion::Closing)
            .all(|step| self.done.contains(step.id));
        let closing = STEPS
            .iter()
            .find(|step| step.completion == Completion::Closing)
            .map(|step| step.id);
        match self.mode {
            Mode::Off if !self.done.is_empty() || self.shown.is_some() => {
                Err("an ended tutorial keeps no progress".to_owned())
            }
            Mode::Finished if !given || self.shown.as_deref() != closing => {
                Err("the tutorial is finished before its steps are done".to_owned())
            }
            _ => Ok(()),
        }
    }
}

/// The hint of a step: ECHO-7 in character, then the instruction in neutral words.
fn step_events(index: usize, step: &Step) -> Vec<Event> {
    let part = |what: &str| Text::dynamic(format!("tutorial.step.{}.{what}", step.id));
    let line = if step.completion == Completion::Closing {
        Text::new("tutorial.step.end").with_text("text", part("do"))
    } else {
        Text::new("tutorial.step.line")
            .with_int("number", count(index + 1))
            .with_int("total", count(STEPS.len() - 1))
            .with_text("text", part("do"))
    };
    vec![Event::say(keys::echo7(), part("say")), Event::system(line)]
}

fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

impl CampaignGame {
    /// Whether the command of the table is open now.
    fn command_open(&self, name: &str) -> bool {
        SPECS
            .iter()
            .find(|spec| spec.name == name)
            .is_some_and(|spec| self.availability(spec) == Availability::Open)
    }

    /// `tutorial [skip|restart]`.
    pub(super) fn cmd_tutorial(&mut self, args: &[ArgRef], events: &mut Vec<Event>) {
        match args.first().and_then(ArgRef::as_word) {
            Some("skip") if self.state.tutorial.mode() == Mode::Active => {
                self.state.tutorial.stop();
                events.push(Event::system(Text::new("tutorial.skipped")));
            }
            None | Some("skip") => self.tutorial_state(events),
            Some("restart") => {
                self.state.tutorial.begin();
                events.push(Event::system(Text::new("tutorial.restarted")));
            }
            Some(_) => invalid(events),
        }
    }

    /// Where the tutorial stands, in words: the current hint again when it runs.
    fn tutorial_state(&mut self, events: &mut Vec<Event>) {
        let key = match self.state.tutorial.mode() {
            Mode::Off => "tutorial.state.off",
            Mode::Finished => "tutorial.state.finished",
            Mode::Active => {
                let open = self
                    .state
                    .tutorial
                    .current()
                    .filter(|(_, step)| self.command_open(step.command));
                match open {
                    Some((index, step)) => {
                        events.extend(step_events(index, step));
                        // Said now: the hook after the command must not say it again.
                        self.state.tutorial.shown = Some(step.id.to_owned());
                        return;
                    }
                    None => "tutorial.state.waiting",
                }
            }
        };
        events.push(Event::system(Text::new(key)));
    }

    /// After a command, at the command prompt: says the hint of the current step, once, when
    /// its command is open. Saying the closing step ends the tutorial.
    pub(super) fn teach(&mut self, events: &mut Vec<Event>) {
        if self.over || !self.state.flows.is_empty() || self.state.tutorial.mode() != Mode::Active {
            return;
        }
        let Some((index, step)) = self.state.tutorial.current() else {
            return;
        };
        if self.state.tutorial.shown.as_deref() == Some(step.id) || !self.command_open(step.command)
        {
            return;
        }
        events.extend(step_events(index, step));
        self.state.tutorial.shown = Some(step.id.to_owned());
        if step.completion == Completion::Closing {
            self.state.tutorial.mode = Mode::Finished;
        }
    }

    /// For a game taken up again: the instruction of the step it was on, if it was said. Never
    /// changes the state, so taking up a game any number of times says the same thing.
    pub(super) fn tutorial_resume(&self) -> Vec<Event> {
        let tutorial = &self.state.tutorial;
        if tutorial.mode() != Mode::Active {
            return Vec::new();
        }
        match tutorial.current() {
            Some((index, step)) if tutorial.shown.as_deref() == Some(step.id) => {
                step_events(index, step).into_iter().skip(1).collect()
            }
            _ => Vec::new(),
        }
    }

    /// The answer to the offer that closes the opening.
    pub(super) fn offer_step(&mut self, input: &Input, events: &mut Vec<Event>) {
        let accepted = match input {
            Input::Confirm(answer) => *answer,
            // Backing out of a question is declining it.
            Input::Cancel => false,
            _ => {
                invalid(events);
                return;
            }
        };
        self.state.flows.pop();
        if accepted {
            self.state.tutorial.begin();
        }
        self.finish_opening(accepted, events);
    }
}
