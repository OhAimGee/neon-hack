//! A driver that plays a campaign with typed lines, the way a frontend does, and renders
//! what the game says.

#![allow(
    clippy::format_push_string,
    reason = "a test renderer that builds small strings, where `write!` would only add noise"
)]

use std::collections::BTreeSet;

use crate::campaign::{CampaignGame, Difficulty};
use crate::content::Fact;
use crate::content::schema::{ContactState, QuestStatus};
use crate::content::state::QuestRun;
use crate::event::{Event, Role};
use crate::game::Game;
use crate::prompt::{Input, Prompt, Step};
use crate::save::SaveRequest;
use crate::text::{Catalog, Lang, RenderMode, Text, render};

/// The embedded catalog of a language.
pub(super) fn catalog(lang: Lang) -> Catalog {
    Catalog::embedded(lang).unwrap()
}

/// A campaign being played.
pub(super) struct Driver {
    pub(super) game: CampaignGame,
    /// The last step.
    pub(super) last: Step,
    /// The saves the game asked for, in order.
    pub(super) saves: Vec<SaveRequest>,
    /// Every event said so far.
    pub(super) history: Vec<Event>,
}

impl Driver {
    /// A new campaign at the handle prompt.
    pub(super) fn raw() -> Self {
        Self::raw_with(Difficulty::Normal)
    }

    pub(super) fn raw_with(difficulty: Difficulty) -> Self {
        let mut game = CampaignGame::new(difficulty, 7).unwrap();
        let last = game.start();
        Self {
            history: last.events.clone(),
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
        self.history.extend(self.last.events.clone());
        self.saves.extend(self.last.save);
        &self.last
    }

    /// Sends a typed line, answering a confirmation by `y` or not, as the frontends do, and
    /// returns what it caused, rendered in English.
    pub(super) fn line(&mut self, line: &str) -> String {
        let input = match &self.game.prompt() {
            Prompt::Confirm { .. } => Input::Confirm(matches!(line, "y" | "yes" | "o" | "oui")),
            _ => Input::Line(line.to_owned()),
        };
        self.send(input);
        self.text()
    }

    /// Sends several lines; returns what the last one caused.
    pub(super) fn lines(&mut self, lines: &[&str]) -> String {
        let mut out = String::new();
        for line in lines {
            out = self.line(line);
        }
        out
    }

    /// The events of the last step, rendered in English.
    pub(super) fn text(&self) -> String {
        self.rendered(Lang::En)
    }

    pub(super) fn rendered(&self, lang: Lang) -> String {
        let cat = catalog(lang);
        self.last
            .events
            .iter()
            .map(|event| render_event(event, &cat))
            .collect()
    }

    /// The prompt as a frontend shows it, in English.
    pub(super) fn prompt_text(&self) -> String {
        render_prompt(&self.last.prompt, &catalog(Lang::En))
    }

    /// Whether the last step holds an error.
    pub(super) fn errored(&self) -> bool {
        self.last.events.iter().any(|event| {
            matches!(
                event,
                Event::Message {
                    role: Role::Error,
                    ..
                }
            )
        })
    }

    // ------------------------------------------------------------------ shortcuts

    pub(super) fn status(&self, quest: &str) -> QuestStatus {
        self.game.state.missions.status(&quest.parse().unwrap())
    }

    pub(super) fn credits(&self) -> u32 {
        self.game.credits()
    }

    /// Plays the first quest through its commands: one of each thing it asks for.
    pub(super) fn finish_m01(&mut self) {
        self.lines(&[
            "quests",
            "help",
            "net",
            "status",
            "talk echo7",
            "0",
            "hack localhost",
            "y",
            "laylow",
        ]);
        assert_eq!(self.status("m01"), QuestStatus::Completed);
    }

    /// Plays the second quest: one intrusion on the first big site.
    pub(super) fn finish_m02(&mut self) {
        self.finish_m01();
        self.lines(&["hack corp-server-01", "y"]);
        assert_eq!(self.status("m02"), QuestStatus::Completed);
    }

    /// Applies facts at the next turn, as a command would, and returns what was said.
    pub(super) fn feed(&mut self, mut facts: Vec<Fact>) -> String {
        let at = self.game.state.next_turn();
        facts.push(Fact::Tick { at });
        let mut events = Vec::new();
        self.game.apply(facts, at, &mut events);
        events
            .iter_mut()
            .for_each(crate::campaign::keys::gloss_event);
        let cat = catalog(Lang::En);
        events
            .iter()
            .map(|event| render_event(event, &cat))
            .collect()
    }

    /// Jumps to the start of chapter 4 without playing it: tier 5, M01 to M08 done, the
    /// contacts met. The lists are brought up to date like after any command.
    pub(super) fn jump_to_chapter4(&mut self) {
        self.jump_to(&[
            "m01", "m02", "m03", "m04", "m05", "s06", "m06", "m07", "m08",
        ]);
    }

    /// Jumps to a state where exactly these quests are done, at tier 5, the contacts met.
    pub(super) fn jump_to(&mut self, done: &[&str]) {
        let c = self.game.content;
        let state = &mut self.game.state;
        state.missions.tier = 5;
        for done in done {
            state.missions.claimed.insert(format!("open-{done}"));
            state.missions.claimed.insert(format!("quest-{done}"));
            state.missions.quests.insert(
                done.parse().unwrap(),
                QuestRun {
                    status: QuestStatus::Completed,
                    opened_at: 0,
                    done: BTreeSet::default(),
                },
            );
        }
        for contact in [
            "r4z0r", "phoenix", "miner", "angel", "ghost", "aura", "insider",
        ] {
            state
                .missions
                .contacts
                .insert(contact.parse().unwrap(), ContactState::Available);
        }
        // The next `feed` settles the content and tells what the jump opened.
        state.sync_lists(c);
    }
}

/// An event as lines, in full mode (a stand-in for the renderer of the CLI).
pub(super) fn render_event(event: &Event, cat: &Catalog) -> String {
    let mode = RenderMode::FULL;
    match event {
        Event::Message { role, text, .. } => {
            let tag = match role {
                Role::Alert(_) => "[ALERT] ",
                Role::Reward => "[Reward] ",
                Role::Error => "[Error] ",
                Role::Dialogue { speaker } => {
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
