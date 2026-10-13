//! The glossary terms every text of an intrusion may cite without being given them, so that a
//! template writes `{patrol}` and never the word (same mechanism as the campaign's).

use crate::event::{Event, Table};
use crate::game::PanelSection;
use crate::prompt::{Choice, Prompt};
use crate::text::{Arg, Text};

use super::view::RunView;

const TERMS: &[&str] = &[
    "run", "turn", "cycle", "node", "patrol", "program", "trace", "forecast", "loot", "jackout",
    "ice", "deck", "undo", "burned", "noise", "site", "intel", "difficulty_story",
];

/// Gives a text, and the texts nested in its arguments, the terms it was not given.
pub(super) fn gloss(text: &mut Text) {
    for term in TERMS {
        if !text.args.iter().any(|(name, _)| name == term) {
            text.args.push((term, Arg::Term(term)));
        }
    }
    for (_, arg) in &mut text.args {
        if let Arg::Text(inner) = arg {
            gloss(inner);
        }
    }
}

fn gloss_table(table: &mut Table) {
    gloss(&mut table.title);
    table.columns.iter_mut().for_each(gloss);
    table.rows.iter_mut().flatten().for_each(gloss);
}

/// [`gloss`] for every text of an event.
pub(super) fn gloss_event(event: &mut Event) {
    match event {
        Event::Message { text, .. } => gloss(text),
        Event::Decor { alt, .. } => gloss(alt),
        Event::Changed { band, .. } => gloss(band),
        Event::Screen(table) => gloss_table(table),
        Event::Effect(_) | Event::Break => {}
    }
}

/// [`gloss`] for every text of a view, so a panel renders it as the terminal would.
pub(super) fn gloss_view(view: &mut RunView) {
    gloss(&mut view.site);
    gloss(&mut view.trace.band);
    view.nodes.iter_mut().for_each(|node| gloss(&mut node.name));
    view.deck
        .iter_mut()
        .for_each(|program| gloss(&mut program.name));
    view.forecast.iter_mut().for_each(gloss);
}

/// [`gloss`] for every text of the panel sections and the status bar.
pub(super) fn gloss_sections(sections: &mut [PanelSection], status: &mut [Text]) {
    for section in sections {
        gloss(&mut section.title);
        for row in &mut section.rows {
            gloss(&mut row.text);
        }
    }
    status.iter_mut().for_each(gloss);
}

/// [`gloss`] for every text of a prompt.
pub(super) fn gloss_prompt(prompt: &mut Prompt) {
    match prompt {
        Prompt::Confirm { question, .. } => gloss(question),
        Prompt::Text { label, .. } => gloss(label),
        Prompt::Choice(Choice {
            title,
            options,
            cancel,
        }) => {
            gloss(title);
            for option in options {
                gloss(&mut option.label);
            }
            cancel.iter_mut().for_each(gloss);
        }
        Prompt::Command | Prompt::Continue | Prompt::End => {}
    }
}
