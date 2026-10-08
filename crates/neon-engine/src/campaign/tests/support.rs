//! A driver that plays a campaign with typed lines, the way a frontend does, and renders
//! what the game says.

use crate::campaign::{CampaignGame, Difficulty};
use crate::event::Event;
use crate::game::Game;
use crate::prompt::{Input, Prompt, Step};
use crate::save::SaveRequest;
use crate::text::{Catalog, Lang, RenderMode, Text, render};

/// The embedded catalog of a language.
pub(super) fn catalog(lang: Lang) -> Catalog {
    Catalog::embedded(lang).unwrap()
}

/// Renders a text in full mode.
pub(super) fn say(text: &Text, lang: Lang) -> String {
    render(text, &catalog(lang), RenderMode::FULL)
}

/// A campaign being played.
pub(super) struct Driver {
    pub(super) game: CampaignGame,
    /// Everything said so far, in order.
    pub(super) events: Vec<Event>,
    /// The last step.
    pub(super) last: Step,
    /// The saves the game asked for, in order.
    pub(super) saves: Vec<SaveRequest>,
}

impl Driver {
    /// A new campaign at the handle prompt.
    pub(super) fn raw() -> Self {
        let mut game = CampaignGame::new(Difficulty::Normal, 7).unwrap();
        let last = game.start();
        Self {
            events: last.events.clone(),
            game,
            last,
            saves: Vec::new(),
        }
    }

    /// A new campaign past the handle prompt (the handle is `Neon`), at the command prompt.
    pub(super) fn new() -> Self {
        let mut driver = Self::raw();
        driver.line("Neon");
        driver
    }

    /// Sends an input.
    pub(super) fn send(&mut self, input: Input) -> &Step {
        self.last = self.game.handle(input);
        self.events.extend(self.last.events.clone());
        self.saves.extend(self.last.save);
        &self.last
    }

    /// Sends a typed line and returns the events it caused, rendered in English.
    pub(super) fn line(&mut self, line: &str) -> String {
        // Confirmations are answered as the frontends do.
        let input = match &self.game.prompt() {
            Prompt::Confirm { .. } => Input::Confirm(matches!(line, "y" | "yes" | "o" | "oui")),
            _ => Input::Line(line.to_owned()),
        };
        self.send(input);
        self.rendered(Lang::En)
    }

    /// The events of the last step, rendered.
    pub(super) fn rendered(&self, lang: Lang) -> String {
        let cat = catalog(lang);
        let mut out = String::new();
        for event in &self.last.events {
            out.push_str(&render_event(event, &cat));
        }
        out
    }

    /// Plays lines in order, returning a transcript with the prompts and echoes.
    pub(super) fn script(&mut self, lines: &[&str]) -> String {
        let cat = catalog(Lang::En);
        let mut out = String::new();
        for line in lines {
            out.push_str(&format!("> {line}\n"));
            self.line(line);
            out.push_str(&self.rendered(Lang::En));
            out.push_str(&render_prompt(&self.last.prompt, &cat));
        }
        out
    }

    /// Whether the last step's text contains `needle` once rendered in English.
    pub(super) fn said(&self, needle: &str) -> bool {
        self.rendered(Lang::En).contains(needle)
    }
}

/// An event as lines, in full mode (a tiny stand-in for the renderer of the CLI).
pub(super) fn render_event(event: &Event, cat: &Catalog) -> String {
    let mode = RenderMode::FULL;
    match event {
        Event::Message { role, text, .. } => {
            let tag = match role {
                crate::event::Role::Alert(_) => "[ALERT] ",
                crate::event::Role::Reward => "[Reward] ",
                crate::event::Role::Error => "[Error] ",
                crate::event::Role::Dialogue { speaker } => {
                    return format!("{speaker} > {}\n", render(text, cat, mode));
                }
                _ => "",
            };
            format!("{tag}{}\n", render(text, cat, mode))
        }
        Event::Screen(table) => {
            let mut out = format!("{}\n", render(&table.title, cat, mode));
            for (index, row) in table.rows.iter().enumerate() {
                let cells: Vec<String> = row.iter().map(|cell| render(cell, cat, mode)).collect();
                out.push_str(&format!("[{}] {}\n", index + 1, cells.join(" - ")));
            }
            out
        }
        Event::Changed {
            gauge,
            from,
            to,
            band,
        } => format!(
            "{} {from} -> {to} ({})\n",
            render(&Text::new(gauge.name_key()), cat, mode),
            render(band, cat, mode)
        ),
        Event::Decor { alt, .. } => format!("{}\n", render(alt, cat, mode)),
        Event::Break => "\n".to_owned(),
    }
}

/// A prompt as the lines a frontend shows above the input.
pub(super) fn render_prompt(prompt: &Prompt, cat: &Catalog) -> String {
    let mode = RenderMode::FULL;
    match prompt {
        Prompt::Command => String::new(),
        Prompt::Text { label, .. } => format!("{}:\n", render(label, cat, mode)),
        Prompt::Confirm { question, .. } => format!("{} [y/n]\n", render(question, cat, mode)),
        Prompt::Continue => "[Enter]\n".to_owned(),
        Prompt::End => "(end)\n".to_owned(),
        Prompt::Choice(choice) => {
            let mut out = format!("{}\n", render(&choice.title, cat, mode));
            for (index, option) in choice.options.iter().enumerate() {
                let label = render(&option.label, cat, mode);
                match &option.available {
                    Ok(()) => out.push_str(&format!("  {}. {label}\n", index + 1)),
                    Err(reason) => out.push_str(&format!(
                        "  {}. {label} ({})\n",
                        index + 1,
                        render(reason, cat, mode)
                    )),
                }
            }
            if let Some(cancel) = &choice.cancel {
                out.push_str(&format!("  0. {}\n", render(cancel, cat, mode)));
            }
            out
        }
    }
}
