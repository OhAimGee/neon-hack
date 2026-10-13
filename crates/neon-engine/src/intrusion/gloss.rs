//! The glossary terms every text of an intrusion may cite without being given them, so that a
//! template writes `{patrol}` and never the word (same mechanism as the campaign's).

use crate::event::{Event, Table};
use crate::prompt::{Choice, Prompt};
use crate::text::{Arg, Text};

const TERMS: &[&str] = &[
    "run", "turn", "cycle", "node", "patrol", "program", "trace", "forecast", "loot", "jackout",
    "ice", "deck", "undo", "burned", "noise", "site",
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
            options.iter_mut().for_each(|option| gloss(&mut option.label));
            cancel.iter_mut().for_each(gloss);
        }
        Prompt::Command | Prompt::Continue | Prompt::End => {}
    }
}
