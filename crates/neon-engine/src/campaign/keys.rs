//! The texts of the content, by their derived keys (`content::texts`): the campaign never
//! writes a title or a dialogue line, it points at the key the ids give.

use crate::content::ids::{
    ChoiceId, ContactId, DecisionId, EndingId, ItemId, QuestId, ReadableId, ServiceId, TopicId,
};
use crate::event::{Event, Table};
use crate::prompt::{Choice, Prompt};
use crate::text::{Arg, Text};

/// An id as it is written in a key: `-` becomes `_`.
fn k(id: &impl std::fmt::Display) -> String {
    id.to_string().replace('-', "_")
}

/// ECHO-7, who gives the hints.
pub(super) fn echo7() -> ContactId {
    match ContactId::new("echo7") {
        Ok(id) => id,
        Err(error) => unreachable!("the static id `echo7` is valid: {error}"),
    }
}

pub(super) fn quest_title(id: &QuestId) -> Text {
    Text::dynamic(format!("quest.{}.title", k(id)))
}

pub(super) fn quest_desc(id: &QuestId) -> Text {
    Text::dynamic(format!("quest.{}.desc", k(id)))
}

pub(super) fn quest_title_short(id: &QuestId) -> Text {
    Text::dynamic(format!("quest.{}.title_short", k(id)))
}

/// Objective `number`, counted from one.
pub(super) fn quest_objective(id: &QuestId, number: usize) -> Text {
    Text::dynamic(format!("quest.{}.obj.{number}", k(id)))
}

/// Hint `level` (1 a lead, 2 the solution) of objective `number`, counted from one.
pub(super) fn quest_hint_key(id: &QuestId, number: usize, level: u8) -> String {
    format!("quest.{}.hint.{number}.{level}", k(id))
}

pub(super) fn contact_name(id: &ContactId) -> Text {
    Text::dynamic(format!("contact.{}.name", k(id)))
}

pub(super) fn topic_question(contact: &ContactId, topic: &TopicId) -> Text {
    Text::dynamic(format!("contact.{}.topic.{}.q", k(contact), k(topic)))
}

pub(super) fn topic_answer(contact: &ContactId, topic: &TopicId) -> Text {
    Text::dynamic(format!("contact.{}.topic.{}.a", k(contact), k(topic)))
}

pub(super) fn item_name(id: &ItemId) -> Text {
    Text::dynamic(format!("item.{}.name", k(id)))
}

pub(super) fn mail_subject(id: &ReadableId) -> Text {
    Text::dynamic(format!("mail.{}.subject", k(id)))
}

pub(super) fn mail_body(id: &ReadableId) -> Text {
    Text::dynamic(format!("mail.{}.body", k(id)))
}

pub(super) fn fragment_title(id: &ReadableId) -> Text {
    Text::dynamic(format!("frag.{}.title", k(id)))
}

pub(super) fn fragment_body(id: &ReadableId) -> Text {
    Text::dynamic(format!("frag.{}.body", k(id)))
}

pub(super) fn document_title(id: &ReadableId) -> Text {
    Text::dynamic(format!("doc.{}.title", k(id)))
}

/// Paragraph `number` (from one) of a cutscene.
pub(super) fn cutscene_paragraph(id: &ReadableId, number: u8) -> Text {
    Text::dynamic(format!("cutscene.{}.p{number:02}", k(id)))
}

pub(super) fn decision_prompt(id: &DecisionId) -> Text {
    Text::dynamic(format!("decision.{}.prompt", k(id)))
}

pub(super) fn decision_choice(decision: &DecisionId, choice: &ChoiceId) -> Text {
    Text::dynamic(format!(
        "decision.{}.choice.{}.label",
        k(decision),
        k(choice)
    ))
}

pub(super) fn ending_title(id: &EndingId) -> Text {
    Text::dynamic(format!("ending.{}.title", k(id)))
}

/// Paragraph `number` (from one) of an ending.
pub(super) fn ending_paragraph(id: &EndingId, number: u8) -> Text {
    Text::dynamic(format!("ending.{}.p{number:02}", k(id)))
}

pub(super) fn epilogue_line(id: &impl std::fmt::Display) -> Text {
    Text::dynamic(format!("epilogue.{}", k(id)))
}

pub(super) fn service_name(id: &ServiceId) -> Text {
    Text::dynamic(format!("service.{}.name", k(id)))
}

/// The glossary terms every text of the campaign may cite without being given them: the game
/// adds them to a text that lacks them, once, when it hands the text out. A text then never
/// types a term (`{quest}`, `{net^}`), whoever built it, and the lists that carry a text for
/// each row (help lines, columns) need no plumbing. `handle` is not here: the word and the
/// player's own handle are both called `handle`, so each text says which one it means.
const TERMS: &[&str] = &[
    "net",
    "quest",
    "contract",
    "contact",
    "message",
    "inbox",
    "document",
    "fragment",
    "archives",
    "site",
    "shop",
    "credit",
    "notoriety",
    "journal",
    "slot",
    "hint",
    "service",
    "level",
    "item",
    "decision",
    "checkpoint",
    "objective",
    "chapter",
    "run",
    "panel",
    "loot",
    "reputation",
    "safehouse",
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

/// [`gloss`] for every text of an event.
pub(super) fn gloss_event(event: &mut Event) {
    match event {
        Event::Message { text, .. } => gloss(text),
        Event::Decor { alt, .. } => gloss(alt),
        Event::Changed { band, .. } => gloss(band),
        Event::Screen(Table {
            title,
            columns,
            rows,
        }) => {
            gloss(title);
            columns.iter_mut().for_each(gloss);
            rows.iter_mut().flatten().for_each(gloss);
        }
        Event::Break => {}
    }
}

/// [`gloss`] for every text of a prompt.
pub(super) fn gloss_prompt(prompt: &mut Prompt) {
    match prompt {
        Prompt::Text { label, .. } => gloss(label),
        Prompt::Confirm { question, .. } => gloss(question),
        Prompt::Choice(Choice {
            title,
            options,
            cancel,
        }) => {
            gloss(title);
            for option in options {
                gloss(&mut option.label);
                if let Err(reason) = &mut option.available {
                    gloss(reason);
                }
            }
            cancel.iter_mut().for_each(gloss);
        }
        Prompt::Command | Prompt::Continue | Prompt::End => {}
    }
}
