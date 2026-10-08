//! What the hub commands read: the quest log, the contact list, the ending reached.
//!
//! These are pure reads of `(Content, State)`, in file order, that carry ids and numbers only:
//! the campaign game turns them into [`Text`](crate::text::Text)s with the derived keys of
//! [`texts`](crate::content::texts) (`quest.<id>.title`, `quest.<id>.obj.<n>`...).

use crate::content::Content;
use crate::content::engine::{ENDING_FLAG, available_topics};
use crate::content::eval::{ObjectiveView, journal};
use crate::content::ids::FlagId;
use crate::content::schema::{
    ContactDef, ContactState, EndingDef, FlagValue, QuestDef, QuestStatus,
};
use crate::content::state::State;

/// A quest of the log with its objectives.
#[derive(Debug, Clone)]
pub struct QuestEntry<'a> {
    /// The definition (id, kind, chapter, giver, tier).
    pub def: &'a QuestDef,
    /// Where the quest is.
    pub status: QuestStatus,
    /// One line per objective, `Pending`/`Done`/`Hidden`/`NotApplicable`, with the progress of
    /// the counted ones. Empty while the quest is only offered (or not available).
    pub objectives: Vec<ObjectiveView>,
}

/// The quests that have the given status, in file order, with their journal.
#[must_use]
pub fn quests_with_status<'a>(
    c: &'a Content,
    s: &State,
    status: QuestStatus,
) -> Vec<QuestEntry<'a>> {
    c.quests
        .iter()
        .filter(|q| s.status(&q.id) == status)
        .map(|def| QuestEntry {
            def,
            status,
            objectives: match status {
                QuestStatus::Active | QuestStatus::Completed | QuestStatus::Failed => {
                    journal(c, s, &def.id)
                }
                QuestStatus::Unavailable | QuestStatus::Available => Vec::new(),
            },
        })
        .collect()
}

/// A contact of the contact list.
#[derive(Debug, Clone)]
pub struct ContactEntry<'a> {
    /// The definition (id, starting state and trust).
    pub def: &'a ContactDef,
    /// The state of the contact.
    pub state: ContactState,
    /// Whether the player can talk to the contact now.
    pub reachable: bool,
    /// Trust in the contact.
    pub trust: i32,
    /// The quests this contact offers and the player has not accepted yet.
    pub offers: Vec<&'a QuestDef>,
    /// How many dialogue topics are open to the player right now.
    pub topics: usize,
}

/// Every contact the player knows or could know, in file order. A contact that is `offline`
/// because it is not unlocked yet is part of the list: it is up to the command to hide it.
#[must_use]
pub fn contact_list<'a>(c: &'a Content, s: &State) -> Vec<ContactEntry<'a>> {
    c.contacts
        .iter()
        .map(|def| {
            let state = s.contact(&def.id);
            ContactEntry {
                def,
                state,
                reachable: state.reachable(),
                trust: s.trust(c, &def.id),
                offers: c
                    .quests
                    .iter()
                    .filter(|q| q.giver == def.id && s.status(&q.id) == QuestStatus::Available)
                    .collect(),
                topics: available_topics(c, s, &def.id).len(),
            }
        })
        .collect()
}

/// The ending of the campaign once `settle_ending` fixed it (in chapter 6), `None` before.
#[must_use]
pub fn ending<'a>(c: &'a Content, s: &State) -> Option<&'a EndingDef> {
    let flag = c.flag_named(ENDING_FLAG)?;
    match s.flag(c, &flag.id)? {
        FlagValue::Str(id) => c.endings.iter().find(|e| e.id.as_str() == id),
        _ => None,
    }
}

/// The chapter of the story the player is in, 1 to 6.
#[must_use]
pub fn chapter(c: &Content, s: &State) -> u32 {
    c.flag_named("chapter")
        .and_then(|flag| s.flag(c, &flag.id))
        .map_or(1, |value| match value {
            FlagValue::Int(n) => n.max(1),
            _ => 1,
        })
}

/// A flag of the content by its name, for the commands that show one (`hub`).
#[must_use]
pub fn flag_value(c: &Content, s: &State, name: &str) -> Option<FlagValue> {
    let id: FlagId = c.flag_named(name)?.id.clone();
    s.flag(c, &id)
}
