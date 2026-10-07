//! What the engine waits for, what the frontend sends back, and one engine answer.
//!
//! Menus, shops, conversations and the prologue never read anything themselves: the
//! engine says which [`Prompt`] it is waiting for and the frontend replies with an
//! [`Input`]. That keeps the engine pure, cloneable and savable in the middle of a menu.

use crate::event::Event;
use crate::text::Text;

/// What the engine is waiting for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prompt {
    /// The main command line.
    Command,
    /// A menu: answered by number or by stable id.
    Choice(Choice),
    /// A line of free text.
    Text {
        /// What is asked.
        label: Text,
        /// Longest accepted answer, in characters.
        max_chars: usize,
        /// Value used when the player answers nothing.
        default: Option<String>,
    },
    /// A yes or no question.
    Confirm {
        /// What is asked.
        question: Text,
        /// Answer used when the player answers nothing.
        default: bool,
    },
    /// "Press Enter to continue": replaces every timed pause.
    Continue,
    /// The game is over: nothing more will be accepted.
    End,
}

/// A menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// Title of the menu.
    pub title: Text,
    /// The entries, numbered from 1 for the player.
    pub options: Vec<ChoiceOption>,
    /// Label of the way out (`0`, an empty line or Escape), if the menu has one.
    pub cancel: Option<Text>,
}

/// One entry of a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceOption {
    /// Stable identifier: the player can type it, whatever the language.
    pub id: String,
    /// What the player reads.
    pub label: Text,
    /// `Err` carries the reason, said in words, why the entry cannot be taken now.
    pub available: Result<(), Text>,
}

/// What the frontend sends back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// A typed line (command, menu answer, free text).
    Line(String),
    /// A menu entry picked directly, 0-based (a click or a shortcut).
    Choice(usize),
    /// An answer to a yes or no question.
    Confirm(bool),
    /// "Continue" was acknowledged.
    Continue,
    /// The player backed out (Escape).
    Cancel,
    /// The input is closed: the engine ends the game, at any depth.
    Eof,
}

/// How a menu answer was understood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// The entry at this 0-based index.
    Pick(usize),
    /// The player left the menu.
    Cancel,
    /// The answer means nothing here: say so, change nothing, ask again.
    Invalid,
}

impl Choice {
    /// Understands an answer, with the same rules for every menu: a number from 1, a
    /// stable id (case-insensitive), `0` or an empty line to leave (when the menu has a
    /// way out), a direct 0-based index from the TUI, [`Input::Cancel`] to back out.
    #[must_use]
    pub fn resolve(&self, input: &Input) -> Resolution {
        match input {
            Input::Choice(index) if *index < self.options.len() => Resolution::Pick(*index),
            Input::Cancel => self.leave(),
            Input::Line(line) => self.resolve_line(line.trim()),
            Input::Choice(_) | Input::Confirm(_) | Input::Continue | Input::Eof => {
                Resolution::Invalid
            }
        }
    }

    fn resolve_line(&self, line: &str) -> Resolution {
        if line.is_empty() || line == "0" {
            return self.leave();
        }
        if let Ok(number) = line.parse::<usize>() {
            return match number.checked_sub(1) {
                Some(index) if index < self.options.len() => Resolution::Pick(index),
                _ => Resolution::Invalid,
            };
        }
        self.options
            .iter()
            .position(|option| option.id.eq_ignore_ascii_case(line))
            .map_or(Resolution::Invalid, Resolution::Pick)
    }

    fn leave(&self) -> Resolution {
        if self.cancel.is_some() {
            Resolution::Cancel
        } else {
            Resolution::Invalid
        }
    }
}

/// One answer of the engine to one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// What happened, in order.
    pub events: Vec<Event>,
    /// What the engine waits for next. After a load it is derived from the state.
    pub prompt: Prompt,
    /// The engine asks the frontend to persist the game now (and to say whether it worked).
    pub save_requested: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu(cancel: bool) -> Choice {
        let option = |id: &str| ChoiceOption {
            id: id.to_owned(),
            label: Text::raw(id),
            available: Ok(()),
        };
        Choice {
            title: Text::raw("menu"),
            options: vec![option("wares"), option("Info"), option("leave")],
            cancel: cancel.then(|| Text::raw("back")),
        }
    }

    fn line(text: &str) -> Input {
        Input::Line(text.to_owned())
    }

    #[test]
    fn numbers_start_at_one() {
        let menu = menu(true);
        assert_eq!(menu.resolve(&line("1")), Resolution::Pick(0));
        assert_eq!(menu.resolve(&line(" 3 ")), Resolution::Pick(2));
        assert_eq!(menu.resolve(&line("4")), Resolution::Invalid);
    }

    #[test]
    fn ids_work_whatever_the_case() {
        let menu = menu(true);
        assert_eq!(menu.resolve(&line("WARES")), Resolution::Pick(0));
        assert_eq!(menu.resolve(&line("info")), Resolution::Pick(1));
        assert_eq!(menu.resolve(&line("nothing")), Resolution::Invalid);
    }

    #[test]
    fn zero_empty_line_and_cancel_leave_only_when_there_is_a_way_out() {
        let with_exit = menu(true);
        for input in [line("0"), line(""), line("  "), Input::Cancel] {
            assert_eq!(with_exit.resolve(&input), Resolution::Cancel, "{input:?}");
        }
        let without_exit = menu(false);
        for input in [line("0"), line(""), Input::Cancel] {
            assert_eq!(
                without_exit.resolve(&input),
                Resolution::Invalid,
                "{input:?}"
            );
        }
    }

    #[test]
    fn a_direct_index_picks_and_out_of_range_is_invalid() {
        let menu = menu(true);
        assert_eq!(menu.resolve(&Input::Choice(0)), Resolution::Pick(0));
        assert_eq!(menu.resolve(&Input::Choice(2)), Resolution::Pick(2));
        assert_eq!(menu.resolve(&Input::Choice(3)), Resolution::Invalid);
    }

    #[test]
    fn other_inputs_mean_nothing_to_a_menu() {
        let menu = menu(true);
        for input in [Input::Confirm(true), Input::Continue, Input::Eof] {
            assert_eq!(menu.resolve(&input), Resolution::Invalid, "{input:?}");
        }
    }
}
