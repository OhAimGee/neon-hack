//! From what the player typed to what the engine understands.
//!
//! The plain frontend and the TUI share this: whatever the interface, the same line
//! becomes the same [`Input`] for the same [`Prompt`].

use neon_engine::{Input, Prompt};

/// A line that cannot be turned into an input for the current prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputError {
    /// A yes or no question was answered with something else.
    NotYesOrNo,
}

/// Turns a typed line into the input the engine waits for.
pub(crate) fn to_input(prompt: &Prompt, line: &str) -> Result<Input, InputError> {
    match prompt {
        Prompt::Confirm { default, .. } => parse_confirm(line, *default)
            .map(Input::Confirm)
            .ok_or(InputError::NotYesOrNo),
        Prompt::Continue => Ok(Input::Continue),
        Prompt::Command | Prompt::Choice(_) | Prompt::Text { .. } | Prompt::End => {
            Ok(Input::Line(line.to_owned()))
        }
    }
}

/// Understands a yes or no, in English or French. An empty answer is the default.
fn parse_confirm(line: &str, default: bool) -> Option<bool> {
    match line.trim().to_lowercase().as_str() {
        "" => Some(default),
        "y" | "yes" | "o" | "oui" => Some(true),
        "n" | "no" | "non" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neon_engine::prompt::{Choice, ChoiceOption};
    use neon_engine::text::Text;

    fn confirm(default: bool) -> Prompt {
        Prompt::Confirm {
            question: Text::raw("?"),
            default,
        }
    }

    #[test]
    fn yes_and_no_are_understood_in_both_languages() {
        for yes in ["y", "Y", "yes", "YES", "o", "oui", " Oui "] {
            assert_eq!(
                to_input(&confirm(false), yes),
                Ok(Input::Confirm(true)),
                "{yes:?}"
            );
        }
        for no in ["n", "no", "NON", "non"] {
            assert_eq!(
                to_input(&confirm(true), no),
                Ok(Input::Confirm(false)),
                "{no:?}"
            );
        }
    }

    #[test]
    fn an_empty_answer_is_the_default_and_anything_else_is_refused() {
        assert_eq!(to_input(&confirm(true), ""), Ok(Input::Confirm(true)));
        assert_eq!(to_input(&confirm(false), "  "), Ok(Input::Confirm(false)));
        assert_eq!(
            to_input(&confirm(true), "maybe"),
            Err(InputError::NotYesOrNo)
        );
    }

    #[test]
    fn other_prompts_get_the_line_as_typed_or_a_continue() {
        let choice = Prompt::Choice(Choice {
            title: Text::raw("t"),
            options: vec![ChoiceOption {
                id: "a".to_owned(),
                label: Text::raw("a"),
                available: Ok(()),
            }],
            cancel: None,
        });
        for prompt in [Prompt::Command, choice] {
            assert_eq!(
                to_input(&prompt, " scan "),
                Ok(Input::Line(" scan ".to_owned()))
            );
        }
        assert_eq!(to_input(&Prompt::Continue, "whatever"), Ok(Input::Continue));
    }
}
