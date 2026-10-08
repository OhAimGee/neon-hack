//! Evaluation of conditions and objectives. Pure functions of `(Content, State)`.

use crate::content::Content;
use crate::ids::QuestId;
use crate::schema::{Cond, ExtractWhat, FlagValue, Goal, Objective, QuestDef, Turn};
use crate::state::{ObjKey, State};

impl Cond {
    /// Evaluates the condition. It is a finite tree of reads: no loop, no write.
    pub fn eval(&self, c: &Content, s: &State) -> bool {
        match self {
            Cond::All { of } => of.iter().all(|x| x.eval(c, s)),
            Cond::Any { of } => of.iter().any(|x| x.eval(c, s)),
            Cond::Not { of } => !of.eval(c, s),
            Cond::Flag {
                name,
                is,
                at_least,
                has,
            } => {
                let Some(value) = s.flag(c, name) else {
                    return false;
                };
                match (is, at_least, has) {
                    (Some(expected), None, None) => value == *expected,
                    (None, Some(n), None) => matches!(value, FlagValue::Int(v) if v >= *n),
                    (None, None, Some(bit)) => {
                        matches!(&value, FlagValue::Bits(b) if b.contains(bit))
                    }
                    _ => false,
                }
            }
            Cond::Quest { id, is } => s.status(id) == *is,
            Cond::Contact { id, is } => is.contains(&s.contact(id)),
            Cond::Trust { contact, at_least } => s.trust(c, contact) >= *at_least,
            Cond::Opened { id } => s.opened.contains(id),
            Cond::Objective { quest, id } => c
                .quest(quest)
                .and_then(|q| find_objective(q, id))
                .is_some_and(|key| s.quests.get(quest).is_some_and(|r| r.done.contains(&key))),
        }
    }
}

/// Path of the objective (or alternative) with this id.
pub fn find_objective(q: &QuestDef, id: &str) -> Option<ObjKey> {
    for (i, o) in q.objective.iter().enumerate() {
        let i = u8::try_from(i).ok()?;
        if o.id.as_deref() == Some(id) {
            return Some((i, 0));
        }
        for (j, ch) in o.children().iter().enumerate() {
            if ch.id.as_deref() == Some(id) {
                return Some((i, u8::try_from(j + 1).ok()?));
            }
        }
    }
    None
}

/// Whether the objective applies in the current state (its `when` guard).
pub fn applicable(c: &Content, s: &State, o: &Objective) -> bool {
    o.when.as_ref().is_none_or(|w| w.eval(c, s))
}

fn count(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

/// Whether the goal is met right now. Event goals look only at what happened after
/// `opened_at`; stock goals read the state, whenever it was reached.
pub fn holds(c: &Content, s: &State, q: &QuestDef, opened_at: Turn, o: &Objective) -> bool {
    match &o.goal {
        Goal::Talk { contact, count: n } => {
            let from = (contact.clone(), opened_at.saturating_add(1));
            let to = (contact.clone(), Turn::MAX);
            s.talks.range(from..=to).count() >= count(*n)
        }
        Goal::Buy { item } => s.owned.contains(item),
        Goal::Compromise {
            site: Some(site), ..
        } => s.compromised.get(site).is_some_and(|t| *t > opened_at),
        Goal::Compromise {
            site: None,
            count: n,
        } => s.compromised.values().filter(|t| **t > opened_at).count() >= count(*n),
        Goal::Extract { site, what } => match (what, site) {
            (ExtractWhat::File(file), Some(site)) => {
                s.extracted.contains(&(site.clone(), file.clone()))
            }
            (ExtractWhat::All, Some(site)) => c.site(site).is_some_and(|d| {
                !d.file.is_empty()
                    && d.file
                        .iter()
                        .all(|f| s.extracted.contains(&(site.clone(), f.id.clone())))
            }),
            (ExtractWhat::Count(n), Some(site)) => {
                s.extracted.iter().filter(|(st, _)| st == site).count() >= count(*n)
            }
            (ExtractWhat::Count(n), None) => s.extracted.len() >= count(*n),
            _ => false,
        },
        Goal::Open { readable } => s.opened.contains(readable),
        Goal::Link { contact, level } => s.links.get(contact).is_some_and(|l| l >= level),
        Goal::Reputation { min } => i64::from(s.reputation) >= i64::from(*min),
        Goal::Use {
            command: Some(command),
            ..
        } => s.used.contains(command),
        Goal::Use {
            group: Some(group), ..
        } => c
            .commands
            .iter()
            .any(|d| d.group.as_deref() == Some(group) && s.used.contains(&d.id)),
        Goal::Use { .. } | Goal::Invalid(_) => false,
        Goal::SiteState { site, mark } => s.marks.contains(&(site.clone(), *mark)),
        Goal::Pay { amount } => {
            let paid: u64 = s
                .payments
                .iter()
                .filter(|(t, _)| *t > opened_at)
                .map(|(_, a)| u64::from(*a))
                .sum();
            paid >= u64::from(c.resolve(*amount, q.tier).0)
        }
        Goal::Choice { decision } => s.decisions.contains_key(decision),
        Goal::HeatEndBelow { max } => u32::from(s.heat()) < *max,
        Goal::HeatPeakBelow { max } => u32::from(s.peak_since(opened_at)) < *max,
        Goal::AnyOf(of) => of
            .iter()
            .any(|ch| applicable(c, s, ch) && holds(c, s, q, opened_at, ch)),
    }
}

/// State of an objective in the journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveState {
    Pending,
    Done,
    /// A secret objective not achieved yet: shown as `???`.
    Hidden,
    /// Its `when` guard is false: not shown, not required.
    NotApplicable,
}

/// One line of the quest journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectiveView {
    pub index: usize,
    pub state: ObjectiveState,
    pub optional: bool,
}

/// The journal of a quest, as a frontend would show it.
pub fn journal(c: &Content, s: &State, quest: &QuestId) -> Vec<ObjectiveView> {
    let (Some(q), Some(run)) = (c.quest(quest), s.quests.get(quest)) else {
        return Vec::new();
    };
    q.objective
        .iter()
        .enumerate()
        .map(|(index, o)| {
            let done = if o.goal.is_condition() {
                holds(c, s, q, run.opened_at, o)
            } else {
                u8::try_from(index).is_ok_and(|i| run.done.contains(&(i, 0)))
            };
            let state = match (applicable(c, s, o), done, o.secret) {
                (false, ..) => ObjectiveState::NotApplicable,
                (true, true, _) => ObjectiveState::Done,
                (true, false, true) => ObjectiveState::Hidden,
                (true, false, false) => ObjectiveState::Pending,
            };
            ObjectiveView {
                index,
                state,
                optional: o.optional,
            }
        })
        .collect()
}
