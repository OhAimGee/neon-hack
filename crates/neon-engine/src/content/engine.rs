//! `refresh`: the one place where statuses move and rewards are paid.
//!
//! It is a deterministic function of the state, run to a fixed point by a bounded loop (never
//! by recursion). Effects mutate the state and append [`Outcome`]s for the campaign game;
//! nothing is called back.

use std::collections::BTreeSet;

use crate::content::Content;
use crate::content::eval::{applicable, holds};
use crate::content::ids::{ChoiceId, ContactId, DecisionId, EndingId, FlagId, QuestId, ReadableId};
use crate::content::money::{Credits, Reputation};
use crate::content::schema::{
    Block, ChoiceDef, ContactState, Effect, EndingDef, EpilogueDef, FlagKind, FlagValue, Goal,
    Grant, QuestDef, QuestKind, QuestStatus, TopicDef,
};
use crate::content::state::{ObjKey, QuestRun, State};

/// What the language tells the rest of the engine. The campaign game turns these into
/// [`Event`](crate::Event)s (texts, roles, gauges) and into facts for the other systems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// A quest became available: its giver offers it.
    QuestOffered(QuestId),
    /// A quest became active.
    QuestOpened(QuestId),
    /// A quest was completed.
    QuestCompleted(QuestId),
    /// A failable quest was lost.
    QuestFailed(QuestId),
    /// An offered quest whose availability condition became false.
    QuestWithdrawn(QuestId),
    /// A quest that cannot fail was reopened after a lost peak.
    QuestRestarted(QuestId),
    /// An objective (or an alternative of an `any_of`) was achieved.
    ObjectiveDone {
        /// The quest.
        quest: QuestId,
        /// The objective path.
        key: ObjKey,
    },
    /// A payment. `key` is unique: the source's key is in `State::claimed` (`quest-m05`), a
    /// `grant` effect adds `:<block>.<effect>` to it.
    Reward {
        /// The claim key.
        key: String,
        /// Credits paid.
        credits: Credits,
        /// Reputation paid (negative for a penalty).
        reputation: Reputation,
    },
    /// A flag changed value.
    FlagChanged {
        /// The flag.
        flag: FlagId,
        /// Its new value.
        value: FlagValue,
    },
    /// A contact changed state.
    ContactChanged {
        /// The contact.
        contact: ContactId,
        /// The new state.
        state: ContactState,
    },
    /// Trust was won or lost.
    TrustChanged {
        /// The contact.
        contact: ContactId,
        /// Points gained (or lost when negative).
        delta: i32,
    },
    /// The player reached a new tier (shown as a level).
    TierGranted(u8),
    /// A mail to deliver, a scene to play, a fragment or document to reveal.
    Unlocked(ReadableId),
    /// The heat must be forced to this value (the game answers with a heat reading).
    HeatForced(u8),
    /// The heat gets a floor until a quest opens.
    HeatFloor {
        /// The floor added.
        delta: i8,
        /// The quest whose opening lifts it.
        until: QuestId,
    },
    /// The ending of the campaign was fixed.
    EndingSettled(EndingId),
    /// A decision whose option was not offered was ignored and forgotten.
    DecisionRejected {
        /// The decision.
        decision: DecisionId,
        /// The option that was refused.
        choice: ChoiceId,
    },
    /// The fixed-point loop hit its bound (a content bug: the validator should prevent it).
    BudgetExceeded,
}

/// A new game: initial state, refreshed once (quests with no prerequisite open).
#[must_use]
pub fn new_game(c: &Content) -> (State, Vec<Outcome>) {
    let mut s = State::new(c);
    let out = refresh(c, &mut s);
    (s, out)
}

/// Moves the state to its fixed point and returns what happened. Idempotent: a second call
/// returns nothing and changes nothing.
pub fn refresh(c: &Content, s: &mut State) -> Vec<Outcome> {
    let mut pass = Pass {
        c,
        s,
        out: Vec::new(),
    };
    let bound = c.quests.len() * 4 + c.decisions.len() + c.topics.len() + c.sites.len() + 16;
    for _ in 0..bound {
        if !pass.run() {
            return pass.out;
        }
    }
    pass.out.push(Outcome::BudgetExceeded);
    pass.out
}

struct Pass<'a> {
    c: &'a Content,
    s: &'a mut State,
    out: Vec<Outcome>,
}

impl Pass<'_> {
    /// One pass over breaches, decisions, topics, then quests, all in file order.
    fn run(&mut self) -> bool {
        let c = self.c;
        let mut changed = self.breaches();
        changed |= self.decisions();
        changed |= self.topics();
        for q in &c.quests {
            changed |= self.step(q);
        }
        changed
    }

    fn claim(&mut self, key: &str) -> bool {
        self.s.claimed.insert(key.to_owned())
    }

    fn breaches(&mut self) -> bool {
        let c = self.c;
        let mut changed = false;
        for site in &c.sites {
            let Some(size) = site.first_breach else {
                continue;
            };
            if self.s.compromised.contains_key(&site.id)
                && self.claim(&format!("breach-{}", site.id))
            {
                let grant = Grant {
                    credits: Some(size),
                    reputation: Some(size),
                    penalty: false,
                };
                self.pay(
                    &grant,
                    site.unlock.tier.unwrap_or(1),
                    &format!("breach-{}", site.id),
                );
                changed = true;
            }
        }
        changed
    }

    fn decisions(&mut self) -> bool {
        let c = self.c;
        let mut changed = false;
        for d in &c.decisions {
            let Some(choice_id) = self.s.decisions.get(&d.id).cloned() else {
                continue;
            };
            let key = format!("decision-{}", d.id);
            if self.s.claimed.contains(&key) {
                continue;
            }
            changed = true;
            let choice = d.choice.iter().find(|ch| ch.id == choice_id);
            let Some(choice) =
                choice.filter(|ch| ch.requires.as_ref().is_none_or(|r| r.eval(c, self.s)))
            else {
                self.s.decisions.remove(&d.id);
                self.out.push(Outcome::DecisionRejected {
                    decision: d.id.clone(),
                    choice: choice_id,
                });
                continue;
            };
            self.claim(&key);
            let tier = c.quest(&d.quest).map_or(1, |q| q.tier);
            self.set_flag(&d.flag, &FlagValue::Str(choice_id.to_string()));
            self.blocks(&choice.then, tier, &key);
        }
        changed
    }

    fn topics(&mut self) -> bool {
        let c = self.c;
        let mut changed = false;
        for t in &c.topics {
            let chosen = (t.contact.clone(), t.id.clone());
            let key = format!("topic-{}-{}", t.contact, t.id);
            if !self.s.topics.contains(&chosen) || self.s.claimed.contains(&key) {
                continue;
            }
            changed = true;
            if topic_available(c, self.s, t) {
                self.claim(&key);
                self.blocks(&t.then, 1, &key);
            } else {
                self.s.topics.remove(&chosen);
            }
        }
        changed
    }

    fn offerable(&self, q: &QuestDef) -> bool {
        q.prereq
            .iter()
            .all(|p| self.s.status(p) == QuestStatus::Completed)
            && self.s.unlocked(&q.unlock)
            && self.s.contact(&q.giver).can_give()
            && q.available_if
                .as_ref()
                .is_none_or(|a| a.eval(self.c, self.s))
    }

    fn set_status(&mut self, q: &QuestDef, status: QuestStatus) {
        let clock = self.s.clock;
        let run = self
            .s
            .quests
            .entry(q.id.clone())
            .or_insert_with(|| QuestRun {
                status,
                opened_at: clock,
                done: BTreeSet::default(),
            });
        run.status = status;
    }

    fn step(&mut self, q: &QuestDef) -> bool {
        match self.s.status(&q.id) {
            QuestStatus::Completed | QuestStatus::Failed => false,
            QuestStatus::Unavailable => {
                if !self.offerable(q) {
                    return false;
                }
                self.set_status(q, QuestStatus::Available);
                self.out.push(Outcome::QuestOffered(q.id.clone()));
                true
            }
            QuestStatus::Available => {
                if !self.offerable(q) {
                    self.set_status(q, QuestStatus::Unavailable);
                    self.s.accepted.remove(&q.id);
                    self.out.push(Outcome::QuestWithdrawn(q.id.clone()));
                } else if q.kind == QuestKind::Main || self.s.accepted.contains(&q.id) {
                    self.set_status(q, QuestStatus::Active);
                    let clock = self.s.clock;
                    if let Some(run) = self.s.quests.get_mut(&q.id) {
                        run.opened_at = clock;
                        run.done.clear();
                    }
                    self.out.push(Outcome::QuestOpened(q.id.clone()));
                    let key = format!("open-{}", q.id);
                    if self.claim(&key) {
                        self.blocks(&q.on_open, q.tier, &key);
                    }
                } else {
                    return false;
                }
                true
            }
            QuestStatus::Active => self.progress(q),
        }
    }

    fn progress(&mut self, q: &QuestDef) -> bool {
        let Some(run) = self.s.quests.get(&q.id) else {
            return false;
        };
        let opened_at = run.opened_at;
        let restart = self
            .s
            .restarts
            .get(&q.id)
            .copied()
            .filter(|t| *t > opened_at);
        if let (false, Some(at)) = (q.failable, restart) {
            // Reopened at the turn of the restart, not at the end of the batch: a fact of a
            // later turn in the same batch counts.
            if let Some(run) = self.s.quests.get_mut(&q.id) {
                run.opened_at = at;
                run.done.clear();
            }
            self.out.push(Outcome::QuestRestarted(q.id.clone()));
            return true;
        }
        let mut changed = self.latch(q, opened_at);
        // A quest never concludes in the refresh that opened it: it stays visible one turn.
        if self.s.clock > opened_at && self.concludable(q, opened_at) {
            self.conclude(q, QuestStatus::Completed);
            return true;
        }
        let lost = q.objective.iter().any(|o| {
            !o.optional
                && applicable(self.c, self.s, o)
                && matches!(o.goal, Goal::HeatPeakBelow { .. })
                && !holds(self.c, self.s, q, opened_at, o)
        });
        if q.failable && (lost || q.fail_if.as_ref().is_some_and(|f| f.eval(self.c, self.s))) {
            self.conclude(q, QuestStatus::Failed);
            changed = true;
        }
        changed
    }

    /// Records the objectives (and alternatives) that are met, once and for all.
    fn latch(&mut self, q: &QuestDef, opened_at: u32) -> bool {
        let mut newly: Vec<ObjKey> = Vec::new();
        for (i, o) in q.objective.iter().enumerate() {
            let Ok(i) = u8::try_from(i) else { continue };
            if o.goal.is_condition() || !applicable(self.c, self.s, o) {
                continue;
            }
            for (j, ch) in o.children().iter().enumerate() {
                if let Ok(j) = u8::try_from(j + 1)
                    && applicable(self.c, self.s, ch)
                    && holds(self.c, self.s, q, opened_at, ch)
                {
                    newly.push((i, j));
                }
            }
            if holds(self.c, self.s, q, opened_at, o) {
                newly.push((i, 0));
            }
        }
        let mut changed = false;
        if let Some(run) = self.s.quests.get_mut(&q.id) {
            for key in newly {
                if run.done.insert(key) {
                    self.out.push(Outcome::ObjectiveDone {
                        quest: q.id.clone(),
                        key,
                    });
                    changed = true;
                }
            }
        }
        changed
    }

    /// Every required, applicable objective is achieved; conditions hold right now.
    fn concludable(&self, q: &QuestDef, opened_at: u32) -> bool {
        let Some(run) = self.s.quests.get(&q.id) else {
            return false;
        };
        q.objective.iter().enumerate().all(|(i, o)| {
            o.optional
                || !applicable(self.c, self.s, o)
                || if o.goal.is_condition() {
                    holds(self.c, self.s, q, opened_at, o)
                } else {
                    u8::try_from(i).is_ok_and(|i| run.done.contains(&(i, 0)))
                }
        })
    }

    /// Sets the final status first, then pays: a payment can never be repeated.
    fn conclude(&mut self, q: &QuestDef, status: QuestStatus) {
        self.set_status(q, status);
        if status == QuestStatus::Completed {
            self.out.push(Outcome::QuestCompleted(q.id.clone()));
            let key = format!("quest-{}", q.id);
            if self.claim(&key) {
                if let Some(grant) = &q.reward {
                    self.pay(grant, q.tier, &key);
                }
                let delta = self.c.rules.quest_completed;
                self.s.add_trust(&q.giver, delta);
                self.out.push(Outcome::TrustChanged {
                    contact: q.giver.clone(),
                    delta,
                });
                self.blocks(&q.on_complete, q.tier, &key);
            }
        } else {
            self.out.push(Outcome::QuestFailed(q.id.clone()));
            let key = format!("quest-{}-failed", q.id);
            if self.claim(&key) {
                self.blocks(&q.on_fail, q.tier, &key);
            }
        }
    }

    fn pay(&mut self, g: &Grant, tier: u8, key: &str) {
        let (mut credits, mut reputation) = (Credits::ZERO, Reputation::ZERO);
        if let Some(size) = g.credits {
            credits = self.c.resolve(size, tier).0;
        }
        if let Some(size) = g.reputation {
            let r = self.c.resolve(size, tier).1;
            reputation = if g.penalty { r.negated() } else { r };
        }
        self.s.earned = self.s.earned.saturating_add(credits);
        self.s.reputation = self.s.reputation.saturating_add(reputation);
        self.out.push(Outcome::Reward {
            key: key.to_owned(),
            credits,
            reputation,
        });
    }

    /// Runs the blocks of a source. A `grant` inside gets its own stable key
    /// (`<source>:<block>.<effect>`) so that every `Outcome::Reward` key is unique.
    fn blocks(&mut self, blocks: &[Block], tier: u8, key: &str) {
        for (bi, b) in blocks.iter().enumerate() {
            if b.when.as_ref().is_none_or(|w| w.eval(self.c, self.s)) {
                for (ei, e) in b.then.iter().enumerate() {
                    self.effect(e, tier, &format!("{key}:{bi}.{ei}"));
                }
            }
        }
    }

    fn effect(&mut self, e: &Effect, tier: u8, key: &str) {
        match e {
            Effect::SetFlag { flag, value } => self.set_flag(flag, value),
            Effect::SetContactState { contact, state } => {
                if self.s.contacts.insert(contact.clone(), *state) != Some(*state) {
                    self.out.push(Outcome::ContactChanged {
                        contact: contact.clone(),
                        state: *state,
                    });
                }
            }
            Effect::Trust { contact, delta } => {
                self.s.add_trust(contact, *delta);
                self.out.push(Outcome::TrustChanged {
                    contact: contact.clone(),
                    delta: *delta,
                });
            }
            Effect::Grant(g) => self.pay(g, tier, key),
            Effect::GrantTier { tier } => {
                if *tier > self.s.tier {
                    self.s.tier = *tier;
                    self.out.push(Outcome::TierGranted(*tier));
                }
            }
            Effect::Unlock { readable } => {
                if self.s.granted.insert(readable.clone()) {
                    self.out.push(Outcome::Unlocked(readable.clone()));
                }
            }
            Effect::HeatForce { value } => self.out.push(Outcome::HeatForced(*value)),
            Effect::HeatFloor { delta, until } => {
                self.out.push(Outcome::HeatFloor {
                    delta: *delta,
                    until: until.clone(),
                });
            }
            Effect::SettleEnding => {
                let c = self.c;
                if let (Some(ending), Some(flag)) =
                    (settled_ending(c, self.s), c.flag_named(ENDING_FLAG))
                {
                    let id = ending.id.clone();
                    self.set_flag(&flag.id, &FlagValue::Str(id.to_string()));
                    self.out.push(Outcome::EndingSettled(id));
                }
            }
        }
    }

    fn set_flag(&mut self, flag: &FlagId, value: &FlagValue) {
        let Some(def) = self.c.flag(flag) else { return };
        let new = match (def.kind, value, self.s.flag(self.c, flag)) {
            (FlagKind::Bitset, FlagValue::Str(bit), Some(FlagValue::Bits(mut bits))) => {
                bits.insert(bit.clone());
                FlagValue::Bits(bits)
            }
            _ => value.clone(),
        };
        if self.s.flag(self.c, flag).as_ref() != Some(&new) {
            self.s.flags.insert(flag.clone(), new.clone());
            self.out.push(Outcome::FlagChanged {
                flag: flag.clone(),
                value: new,
            });
        }
    }
}

/// Name of the enum flag that holds the ending once `settle_ending` ran.
pub const ENDING_FLAG: &str = "ending";

/// The first ending (file order) whose condition holds.
#[must_use]
pub fn settled_ending<'a>(c: &'a Content, s: &State) -> Option<&'a EndingDef> {
    c.endings.iter().find(|e| e.when.eval(c, s))
}

/// The epilogue lines whose condition holds, in file order.
#[must_use]
pub fn montage<'a>(c: &'a Content, s: &State) -> Vec<&'a EpilogueDef> {
    c.epilogue.iter().filter(|l| l.when.eval(c, s)).collect()
}

/// The options of a decision the player may pick right now.
#[must_use]
pub fn offered_choices<'a>(c: &'a Content, s: &State, decision: &DecisionId) -> Vec<&'a ChoiceDef> {
    c.decision(decision).map_or_else(Vec::new, |d| {
        d.choice
            .iter()
            .filter(|ch| ch.requires.as_ref().is_none_or(|r| r.eval(c, s)))
            .collect()
    })
}

/// Whether a topic can be chosen: the contact is reachable and the topic's condition holds.
#[must_use]
pub fn topic_available(c: &Content, s: &State, t: &TopicDef) -> bool {
    s.contact(&t.contact).reachable() && t.when.as_ref().is_none_or(|w| w.eval(c, s))
}

/// The topics of a contact the player may open now and has not opened yet.
#[must_use]
pub fn available_topics<'a>(c: &'a Content, s: &State, contact: &ContactId) -> Vec<&'a TopicDef> {
    c.topics
        .iter()
        .filter(|t| t.contact == *contact && !s.topics.contains(&(t.contact.clone(), t.id.clone())))
        .filter(|t| topic_available(c, s, t))
        .collect()
}
