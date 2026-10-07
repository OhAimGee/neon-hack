//! Events and prompts as lines of text.
//!
//! One function per kind of event, shared by the plain frontend, the log of the TUI and
//! the transcripts: what the TUI shows in widgets, the plain output says in words.
//! Nothing here relies on colour: a role is always readable in the text itself.

use neon_engine::event::{Event, Gauge, Role, Severity, Table};
use neon_engine::prompt::{Choice, Prompt};
use neon_engine::text::{Catalog, RenderMode, Text, render, to_ascii};

/// How much of the atmosphere the player wants to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) enum Verbosity {
    /// Only what matters: alerts, rewards, errors, answers.
    Brief,
    /// Everything except pure atmosphere.
    #[default]
    Normal,
    /// Everything.
    Full,
}

impl Verbosity {
    fn shows(self, importance: neon_engine::event::Importance) -> bool {
        use neon_engine::event::Importance;
        match importance {
            Importance::Essential => true,
            Importance::Normal => self >= Self::Normal,
            Importance::Flavor => self == Self::Full,
        }
    }
}

/// What a line is about, so a frontend can style it. The text never depends on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineKind {
    Narration,
    Dialogue,
    System,
    Alert(Severity),
    Reward,
    Error,
    Decor,
    Table,
    Gauge,
    Blank,
}

/// One line of output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
    pub(crate) kind: LineKind,
    pub(crate) text: String,
}

impl Line {
    fn new(kind: LineKind, text: String) -> Self {
        Self { kind, text }
    }
}

/// What to show for a prompt: the lines above the input, and the marker before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptView {
    pub(crate) header: Vec<Line>,
    pub(crate) marker: String,
}

/// Turns texts, events and prompts into lines, in one language and one mode.
pub(crate) struct Renderer<'a> {
    pub(crate) catalog: &'a Catalog,
    pub(crate) mode: RenderMode,
    pub(crate) verbosity: Verbosity,
}

impl Renderer<'_> {
    pub(crate) fn text(&self, text: &Text) -> String {
        render(text, self.catalog, self.mode)
    }

    /// Text that is not in the catalogs, such as what the player typed. In ASCII mode it is
    /// transliterated like everything else, so the output stays 7-bit.
    pub(crate) fn verbatim(&self, text: &str) -> String {
        if self.mode.ascii {
            to_ascii(text)
        } else {
            text.to_owned()
        }
    }

    /// The lines of an event; none when the verbosity hides it.
    pub(crate) fn event(&self, event: &Event) -> Vec<Line> {
        if !self.verbosity.shows(event.importance()) {
            return Vec::new();
        }
        match event {
            Event::Message { role, text, .. } => vec![self.message(role, text)],
            Event::Decor { art, alt } => self.decor(art, alt),
            Event::Screen(table) => self.table(table),
            Event::Changed {
                gauge,
                from,
                to,
                band,
            } => self.changed(*gauge, *from, *to, band).into_iter().collect(),
            Event::Break if self.verbosity == Verbosity::Brief => Vec::new(),
            Event::Break => vec![Line::new(LineKind::Blank, String::new())],
        }
    }

    fn message(&self, role: &Role, text: &Text) -> Line {
        match role {
            Role::Narration => Line::new(LineKind::Narration, self.text(text)),
            Role::System => Line::new(LineKind::System, self.text(text)),
            Role::Dialogue { speaker } => {
                let say = Text::new("ui.say")
                    .with_text("speaker", Text::dynamic(format!("contact.{speaker}.name")))
                    .with_text("text", text.clone());
                Line::new(LineKind::Dialogue, self.text(&say))
            }
            Role::Alert(severity) => Line::new(
                LineKind::Alert(*severity),
                self.tagged("ui.tag.alert", text),
            ),
            Role::Reward => Line::new(LineKind::Reward, self.tagged("ui.tag.reward", text)),
            Role::Error => Line::new(LineKind::Error, self.tagged("ui.tag.error", text)),
        }
    }

    fn tagged(&self, tag: &'static str, text: &Text) -> String {
        let tagged = Text::new("ui.tagged")
            .with_text("tag", Text::new(tag))
            .with_text("text", text.clone());
        self.text(&tagged)
    }

    fn decor(&self, art: &[&str], alt: &Text) -> Vec<Line> {
        let short = || vec![Line::new(LineKind::Decor, self.text(alt))];
        if self.mode.screen_reader {
            // Pure decoration is dropped for screen readers; its short text only at full verbosity.
            if self.verbosity == Verbosity::Full {
                short()
            } else {
                Vec::new()
            }
        } else if self.mode.ascii {
            short()
        } else {
            art.iter()
                .map(|row| Line::new(LineKind::Decor, (*row).to_owned()))
                .collect()
        }
    }

    fn table(&self, table: &Table) -> Vec<Line> {
        let mut lines = vec![Line::new(LineKind::Table, self.text(&table.title))];
        for (index, row) in table.rows.iter().enumerate() {
            let number = index + 1;
            let text = if self.mode.screen_reader {
                let cells: Vec<String> = table
                    .columns
                    .iter()
                    .zip(row)
                    .map(|(column, value)| {
                        let cell = Text::new("ui.sr.cell")
                            .with_text("column", column.clone())
                            .with_text("value", value.clone());
                        self.text(&cell)
                    })
                    .collect();
                format!("{number}. {}", cells.join("; "))
            } else {
                let cells: Vec<String> = row.iter().map(|cell| self.text(cell)).collect();
                format!("[{number}] {}", cells.join(" - "))
            };
            lines.push(Line::new(LineKind::Table, text));
        }
        lines
    }

    /// A gauge that did not move has nothing to say, whatever the engine sent.
    fn changed(&self, gauge: Gauge, from: i32, to: i32, band: &Text) -> Option<Line> {
        if from == to {
            return None;
        }
        let key = if to > from {
            "ui.gauge.up"
        } else {
            "ui.gauge.down"
        };
        let text = Text::new(key)
            .with_text("gauge", Text::new(gauge.name_key()))
            .with_int("delta", i64::from(to.abs_diff(from)))
            .with_int("from", i64::from(from))
            .with_int("to", i64::from(to))
            .with_text("band", band.clone());
        Some(Line::new(LineKind::Gauge, self.text(&text)))
    }

    /// The lines above the input and the marker before it, for the prompt the engine waits on.
    pub(crate) fn prompt(&self, prompt: &Prompt) -> PromptView {
        match prompt {
            Prompt::Command => PromptView {
                header: Vec::new(),
                marker: self.text(&Text::new("ui.prompt.command")),
            },
            Prompt::Choice(choice) => PromptView {
                header: self.choice_lines(choice),
                marker: self.text(&Text::new("ui.prompt.choice")),
            },
            Prompt::Text {
                label,
                default: None,
                ..
            } => PromptView {
                header: Vec::new(),
                marker: self.text(&Text::new("ui.prompt.text").with_text("label", label.clone())),
            },
            Prompt::Text {
                label,
                default: Some(default),
                ..
            } => PromptView {
                header: Vec::new(),
                marker: self.text(
                    &Text::new("ui.prompt.text_default")
                        .with_text("label", label.clone())
                        .with_str("default", default.clone()),
                ),
            },
            Prompt::Confirm { question, default } => {
                let hint = if *default {
                    "ui.hint.yes_default"
                } else {
                    "ui.hint.no_default"
                };
                PromptView {
                    header: Vec::new(),
                    marker: self.text(
                        &Text::new("ui.prompt.confirm")
                            .with_text("question", question.clone())
                            .with_text("hint", Text::new(hint)),
                    ),
                }
            }
            Prompt::Continue => PromptView {
                header: Vec::new(),
                marker: self.text(&Text::new("ui.prompt.continue")),
            },
            Prompt::End => PromptView {
                header: Vec::new(),
                marker: String::new(),
            },
        }
    }

    fn choice_lines(&self, choice: &Choice) -> Vec<Line> {
        let numbered = |number: usize, label: &str| {
            if self.mode.screen_reader {
                format!("{number}. {label}")
            } else {
                format!("[{number}] {label}")
            }
        };
        let mut lines = vec![Line::new(LineKind::Table, self.text(&choice.title))];
        for (index, option) in choice.options.iter().enumerate() {
            let label = match &option.available {
                Ok(()) => self.text(&option.label),
                Err(reason) => self.text(
                    &Text::new("ui.choice.unavailable")
                        .with_text("label", option.label.clone())
                        .with_text("reason", reason.clone()),
                ),
            };
            lines.push(Line::new(LineKind::Table, numbered(index + 1, &label)));
        }
        if let Some(cancel) = &choice.cancel {
            lines.push(Line::new(LineKind::Table, numbered(0, &self.text(cancel))));
        }
        lines
    }
}

#[cfg(test)]
mod tests;
