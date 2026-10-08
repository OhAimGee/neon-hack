//! Evaluation of conditions and objectives. Pure functions of `(Content, State)`.

use crate::content::Content;
use crate::content::ids::QuestId;
use crate::content::schema::{Cond, ExtractWhat, FlagValue, Goal, Objective, QuestDef, Turn};
use crate::content::state::{ObjKey, State};

impl Cond {
    /// Evaluates the condition. It is a finite tree of reads: no loop, no write.
    #[must_use]
    pub fn eval(&self, c: &Content, s: &State) -> bool {
        match self {
            Self::All { of } => of.iter().all(|x| x.eval(c, s)),
            Self::Any { of } => of.iter().any(|x| x.eval(c, s)),
            Self::Not { of } => !of.eval(c, s),
            Self::Flag {
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
            Self::Quest { id, is } => s.status(id) == *is,
            Self::Contact { id, is } => is.contains(&s.contact(id)),
            Self::Trust { contact, at_least } => s.trust(c, contact) >= *at_least,
            Self::Opened { id } => s.opened.contains(id),
            Self::Objective { quest, id } => c
                .quest(quest)
                .and_then(|q| find_objective(q, id))
                .is_some_and(|key| s.quests.get(quest).is_some_and(|r| r.done.contains(&key))),
        }
    }
}

/// Path of the objective (or alternative) with this id.
#[must_use]
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
#[must_use]
pub fn applicable(c: &Content, s: &State, o: &Objective) -> bool {
    o.when.as_ref().is_none_or(|w| w.eval(c, s))
}

fn wanted(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

/// A count as a `u32` for display, stopping at the largest value.
fn shown(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Whether the goal is met right now. Event goals look only at what happened after
/// `opened_at`; stock goals read the state, whenever it was reached.
#[must_use]
pub fn holds(
    content: &Content,
    state: &State,
    quest: &QuestDef,
    opened_at: Turn,
    objective: &Objective,
) -> bool {
    match &objective.goal {
        Goal::Talk { contact, count: n } => talks_since(state, contact, opened_at) >= wanted(*n),
        Goal::Buy { item } => state.owned.contains(item),
        Goal::Compromise {
            site: Some(site), ..
        } => state.compromised.get(site).is_some_and(|t| *t > opened_at),
        Goal::Compromise {
            site: None,
            count: n,
        } => breaches_since(state, opened_at) >= wanted(*n),
        Goal::Extract { site, what } => match (what, site) {
            (ExtractWhat::File(file), Some(site)) => {
                state.extracted.contains(&(site.clone(), file.clone()))
            }
            (ExtractWhat::All, Some(site)) => content.site(site).is_some_and(|d| {
                !d.file.is_empty()
                    && d.file
                        .iter()
                        .all(|f| state.extracted.contains(&(site.clone(), f.id.clone())))
            }),
            (ExtractWhat::Count(n), Some(site)) => {
                state.extracted.iter().filter(|(st, _)| st == site).count() >= wanted(*n)
            }
            (ExtractWhat::Count(n), None) => state.extracted.len() >= wanted(*n),
            _ => false,
        },
        Goal::Open { readable } => state.opened.contains(readable),
        Goal::Link { contact, level } => state.links.get(contact).is_some_and(|l| l >= level),
        Goal::Reputation { min } => state.reputation >= *min,
        Goal::Use {
            command: Some(command),
            ..
        } => state.used.contains(command),
        Goal::Use {
            group: Some(group), ..
        } => content
            .commands
            .iter()
            .any(|d| d.group.as_deref() == Some(group) && state.used.contains(&d.id)),
        Goal::Use { .. } | Goal::Invalid(_) => false,
        Goal::SiteState { site, mark } => state.marks.contains(&(site.clone(), *mark)),
        Goal::Pay { amount } => {
            let paid: u64 = state
                .payments
                .iter()
                .filter(|(t, _)| *t > opened_at)
                .map(|(_, a)| u64::from(a.get()))
                .fold(0, u64::saturating_add);
            paid >= u64::from(content.resolve(*amount, quest.tier).0.get())
        }
        Goal::Choice { decision } => state.decisions.contains_key(decision),
        Goal::HeatEndBelow { max } => u32::from(state.heat()) < *max,
        Goal::HeatPeakBelow { max } => u32::from(state.peak_since(opened_at)) < *max,
        Goal::AnyOf(of) => of.iter().any(|ch| {
            applicable(content, state, ch) && holds(content, state, quest, opened_at, ch)
        }),
    }
}

/// Conversations with a contact at a turn after `opened_at`.
fn talks_since(s: &State, contact: &crate::content::ids::ContactId, opened_at: Turn) -> usize {
    let from = (contact.clone(), opened_at.saturating_add(1));
    let to = (contact.clone(), Turn::MAX);
    s.talks.range(from..=to).count()
}

/// Sites breached at a turn after `opened_at`.
fn breaches_since(s: &State, opened_at: Turn) -> usize {
    s.compromised.values().filter(|t| **t > opened_at).count()
}

/// How far a counted objective has got, as `(have, need)`: conversations, breaches or files
/// asked for several times, or every file of a site. `None` for an objective that is a single
/// yes or no.
#[must_use]
pub fn progress(c: &Content, s: &State, opened_at: Turn, o: &Objective) -> Option<(u32, u32)> {
    let (have, need) = match &o.goal {
        Goal::Talk { contact, count } => (talks_since(s, contact, opened_at), wanted(*count)),
        Goal::Compromise {
            site: None,
            count: n,
        } => (breaches_since(s, opened_at), wanted(*n)),
        Goal::Extract {
            site: Some(site),
            what: ExtractWhat::All,
        } => {
            let files = &c.site(site)?.file;
            let have = files
                .iter()
                .filter(|f| s.extracted.contains(&(site.clone(), f.id.clone())))
                .count();
            (have, files.len())
        }
        Goal::Extract {
            site,
            what: ExtractWhat::Count(n),
        } => {
            let have = match site {
                Some(site) => s.extracted.iter().filter(|(st, _)| st == site).count(),
                None => s.extracted.len(),
            };
            (have, wanted(*n))
        }
        _ => return None,
    };
    (need > 1).then(|| (shown(have.min(need)), shown(need)))
}

/// State of an objective in the journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveState {
    /// To do.
    Pending,
    /// Achieved.
    Done,
    /// A secret objective not achieved yet: shown as `???`.
    Hidden,
    /// Its `when` guard is false: not shown, not required.
    NotApplicable,
}

/// One line of the quest journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectiveView {
    /// Position of the objective in the quest (the `N` of the `quest.<id>.obj.N` text).
    pub index: usize,
    /// Where it stands.
    pub state: ObjectiveState,
    /// A bonus objective does not block the conclusion.
    pub optional: bool,
    /// `(have, need)` of a counted objective, see [`progress`].
    pub progress: Option<(u32, u32)>,
}

/// The journal of a quest, as a frontend would show it.
#[must_use]
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
                progress: (state == ObjectiveState::Pending)
                    .then(|| progress(c, s, run.opened_at, o))
                    .flatten(),
            }
        })
        .collect()
}
