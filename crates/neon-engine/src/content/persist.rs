//! Saving the mission state.
//!
//! [`State`] is written as an ordered table with text keys, the shape the versioned TOML
//! envelope of [`crate::save`] wants: every id is a key or a string, a set is a sorted array,
//! a map keyed by turn uses the turn as text. The in-memory layout of [`State`] is free to
//! change; this layout is the one that is saved, so it only changes with a migration.
//!
//! ```toml
//! clock = 12
//! tier = 3
//! reputation = 45
//! earned = 705
//! claimed = ["breach-localhost", "quest-m01"]
//! owned = ["stealth_module"]
//!
//! [heat]            # latest reading
//! at = 9
//! heat = 20
//!
//! [heat_peaks]      # Pareto frontier: turn = heat
//! 9 = 20
//!
//! [contacts]
//! echo7 = "available"
//!
//! [extracted]       # site = [files]
//! corp-server-01 = ["employee_records"]
//!
//! [quests.m01]
//! status = "completed"
//! opened_at = 0
//! done = [[0, 0], [1, 0]]
//! ```
//!
//! Reading is two steps. Deserializing checks the shape (typed ids, caps of [`Credits`] and
//! [`Reputation`], numeric turn keys); [`State::validate`] then checks the state against the
//! [`Content`], so an edited or damaged file cannot put the engine in a situation it could
//! never have reached. Unknown keys are ignored and the collections default to empty, as
//! the save format asks for new optional fields.

use std::collections::{BTreeMap, BTreeSet};

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::content::Content;
use crate::content::ids::{
    ChoiceId, CommandId, ContactId, DecisionId, FileId, FlagId, ItemId, QuestId, ReadableId,
    SiteId, TopicId,
};
use crate::content::money::{Credits, Reputation};
use crate::content::schema::{
    ContactState, FlagKind, FlagValue, MAX_TIER, QuestStatus, SiteMark, Turn,
};
use crate::content::state::{HEAT_MAX, ObjKey, QuestRun, State, TRUST_CAP};
use crate::content::validate::flag_accepts;

/// The latest heat reading.
#[derive(Serialize, Deserialize)]
struct HeatRepr {
    at: Turn,
    heat: u8,
}

/// The progress of one quest.
#[derive(Serialize, Deserialize)]
struct QuestRunRepr {
    status: QuestStatus,
    opened_at: Turn,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    done: Vec<ObjKey>,
}

/// The saved layout of a [`State`].
#[derive(Serialize, Deserialize)]
struct StateRepr {
    clock: Turn,
    tier: u8,
    reputation: Reputation,
    earned: Credits,
    heat: HeatRepr,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    claimed: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    owned: BTreeSet<ItemId>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    opened: BTreeSet<ReadableId>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    granted: BTreeSet<ReadableId>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    used: BTreeSet<CommandId>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    accepted: BTreeSet<QuestId>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    flags: BTreeMap<FlagId, FlagValue>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    contacts: BTreeMap<ContactId, ContactState>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    bonus_trust: BTreeMap<ContactId, i32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    links: BTreeMap<ContactId, u8>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    compromised: BTreeMap<SiteId, Turn>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    extracted: BTreeMap<SiteId, BTreeSet<FileId>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    marks: BTreeMap<SiteId, BTreeSet<SiteMark>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    talks: BTreeMap<ContactId, BTreeSet<Turn>>,
    /// Turn (as text) to the amounts paid at that turn.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    payments: BTreeMap<String, Vec<Credits>>,
    /// Turn (as text) to heat.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    heat_peaks: BTreeMap<String, u8>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    decisions: BTreeMap<DecisionId, ChoiceId>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    topics: BTreeMap<ContactId, BTreeSet<TopicId>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    restarts: BTreeMap<QuestId, Turn>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    quests: BTreeMap<QuestId, QuestRunRepr>,
}

impl From<&State> for StateRepr {
    fn from(s: &State) -> Self {
        let mut extracted: BTreeMap<SiteId, BTreeSet<FileId>> = BTreeMap::new();
        for (site, file) in &s.extracted {
            extracted
                .entry(site.clone())
                .or_default()
                .insert(file.clone());
        }
        let mut marks: BTreeMap<SiteId, BTreeSet<SiteMark>> = BTreeMap::new();
        for (site, mark) in &s.marks {
            marks.entry(site.clone()).or_default().insert(*mark);
        }
        let mut talks: BTreeMap<ContactId, BTreeSet<Turn>> = BTreeMap::new();
        for (contact, at) in &s.talks {
            talks.entry(contact.clone()).or_default().insert(*at);
        }
        let mut payments: BTreeMap<String, Vec<Credits>> = BTreeMap::new();
        for (at, amount) in &s.payments {
            payments.entry(at.to_string()).or_default().push(*amount);
        }
        let mut topics: BTreeMap<ContactId, BTreeSet<TopicId>> = BTreeMap::new();
        for (contact, topic) in &s.topics {
            topics
                .entry(contact.clone())
                .or_default()
                .insert(topic.clone());
        }
        Self {
            clock: s.clock,
            tier: s.tier,
            reputation: s.reputation,
            earned: s.earned,
            heat: HeatRepr {
                at: s.heat_now.0,
                heat: s.heat_now.1,
            },
            claimed: s.claimed.clone(),
            owned: s.owned.clone(),
            opened: s.opened.clone(),
            granted: s.granted.clone(),
            used: s.used.clone(),
            accepted: s.accepted.clone(),
            flags: s.flags.clone(),
            contacts: s.contacts.clone(),
            bonus_trust: s.bonus_trust.clone(),
            links: s.links.clone(),
            compromised: s.compromised.clone(),
            extracted,
            marks,
            talks,
            payments,
            heat_peaks: s
                .heat_peaks
                .iter()
                .map(|(at, heat)| (at.to_string(), *heat))
                .collect(),
            decisions: s.decisions.clone(),
            topics,
            restarts: s.restarts.clone(),
            quests: s
                .quests
                .iter()
                .map(|(id, run)| {
                    (
                        id.clone(),
                        QuestRunRepr {
                            status: run.status,
                            opened_at: run.opened_at,
                            done: run.done.iter().copied().collect(),
                        },
                    )
                })
                .collect(),
        }
    }
}

/// A turn written as a key.
fn parse_turn(key: &str) -> Result<Turn, String> {
    key.parse()
        .map_err(|_| format!("`{key}` is not a turn number"))
}

impl TryFrom<StateRepr> for State {
    type Error = String;

    fn try_from(r: StateRepr) -> Result<Self, String> {
        let mut payments = BTreeSet::new();
        for (key, amounts) in r.payments {
            let at = parse_turn(&key)?;
            payments.extend(amounts.into_iter().map(|amount| (at, amount)));
        }
        let mut heat_peaks = BTreeMap::new();
        for (key, heat) in r.heat_peaks {
            heat_peaks.insert(parse_turn(&key)?, heat);
        }
        Ok(Self {
            clock: r.clock,
            tier: r.tier,
            reputation: r.reputation,
            earned: r.earned,
            flags: r.flags,
            contacts: r.contacts,
            bonus_trust: r.bonus_trust,
            claimed: r.claimed,
            owned: r.owned,
            compromised: r.compromised,
            extracted: r
                .extracted
                .into_iter()
                .flat_map(|(site, files)| files.into_iter().map(move |f| (site.clone(), f)))
                .collect(),
            marks: r
                .marks
                .into_iter()
                .flat_map(|(site, marks)| marks.into_iter().map(move |m| (site.clone(), m)))
                .collect(),
            opened: r.opened,
            granted: r.granted,
            used: r.used,
            links: r.links,
            talks: r
                .talks
                .into_iter()
                .flat_map(|(who, turns)| turns.into_iter().map(move |at| (who.clone(), at)))
                .collect(),
            payments,
            heat_now: (r.heat.at, r.heat.heat),
            heat_peaks,
            decisions: r.decisions,
            topics: r
                .topics
                .into_iter()
                .flat_map(|(who, topics)| topics.into_iter().map(move |t| (who.clone(), t)))
                .collect(),
            accepted: r.accepted,
            restarts: r.restarts,
            quests: r
                .quests
                .into_iter()
                .map(|(id, run)| {
                    (
                        id,
                        QuestRun {
                            status: run.status,
                            opened_at: run.opened_at,
                            done: run.done.into_iter().collect(),
                        },
                    )
                })
                .collect(),
        })
    }
}

impl Serialize for State {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        StateRepr::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for State {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let repr = StateRepr::deserialize(deserializer)?;
        Self::try_from(repr).map_err(D::Error::custom)
    }
}

/// `Ok` when `ok`, otherwise the message.
fn ensure(ok: bool, message: impl FnOnce() -> String) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message()) }
}

impl State {
    /// Checks the state against the content: the check a save goes through when it is loaded
    /// (see [`crate::save::SaveState`]). A state built by [`State::new`], [`State::apply`] and
    /// `refresh` always passes.
    ///
    /// It rejects what the engine could never have produced: an id the content does not
    /// know, a value outside its type (tier, flag, link level, trust), a turn later than the
    /// clock, a heat frontier that is not strictly decreasing, an objective that does not
    /// exist, a reward claimed for something that did not happen, a quest that left
    /// `Unavailable` without its claims. The amounts and the reputation are range-checked
    /// when they are read. It does not replay the rules (prerequisites, gates): content may
    /// be rebalanced without invalidating the saves of players who went further.
    ///
    /// # Errors
    ///
    /// The first problem found, in English, for developers.
    pub fn validate(&self, c: &Content) -> Result<(), String> {
        self.validate_scalars()?;
        self.validate_ids(c)?;
        self.validate_turns()?;
        self.validate_flags(c)?;
        self.validate_quests(c)?;
        self.validate_claims(c)
    }

    fn validate_scalars(&self) -> Result<(), String> {
        ensure((1..=MAX_TIER).contains(&self.tier), || {
            format!("tier {} is outside 1..={MAX_TIER}", self.tier)
        })?;
        ensure(self.heat_now.1 <= HEAT_MAX, || {
            format!("heat {} is above {HEAT_MAX}", self.heat_now.1)
        })?;
        for (contact, trust) in &self.bonus_trust {
            ensure(trust.unsigned_abs() <= TRUST_CAP.unsigned_abs(), || {
                format!("trust {trust} for `{contact}` is outside +-{TRUST_CAP}")
            })?;
        }
        for (contact, level) in &self.links {
            ensure((1..=3).contains(level), || {
                format!("link level {level} for `{contact}` is outside 1..=3")
            })?;
        }
        Ok(())
    }

    /// Every id the state names exists in the content.
    fn validate_ids(&self, c: &Content) -> Result<(), String> {
        for contact in self
            .contacts
            .keys()
            .chain(self.bonus_trust.keys())
            .chain(self.links.keys())
            .chain(self.talks.iter().map(|(who, _)| who))
            .chain(self.topics.iter().map(|(who, _)| who))
        {
            ensure(c.contact(contact).is_some(), || {
                format!("unknown contact `{contact}`")
            })?;
        }
        for item in &self.owned {
            ensure(c.item(item).is_some(), || format!("unknown item `{item}`"))?;
        }
        for site in self.compromised.keys() {
            ensure(c.site(site).is_some(), || format!("unknown site `{site}`"))?;
        }
        for (site, file) in &self.extracted {
            ensure(
                c.site(site)
                    .is_some_and(|d| d.file.iter().any(|f| f.id == *file)),
                || format!("unknown file `{file}` of site `{site}`"),
            )?;
        }
        for (site, _) in &self.marks {
            ensure(c.site(site).is_some(), || format!("unknown site `{site}`"))?;
        }
        for readable in self.opened.iter().chain(&self.granted) {
            ensure(c.readable(readable).is_some(), || {
                format!("unknown readable `{readable}`")
            })?;
        }
        for command in &self.used {
            ensure(c.has_command(command.as_str()), || {
                format!("unknown command `{command}`")
            })?;
        }
        for (decision, choice) in &self.decisions {
            ensure(
                c.decision(decision)
                    .is_some_and(|d| d.choice.iter().any(|ch| ch.id == *choice)),
                || format!("unknown choice `{choice}` of decision `{decision}`"),
            )?;
        }
        for (contact, topic) in &self.topics {
            ensure(
                c.topics
                    .iter()
                    .any(|t| t.contact == *contact && t.id == *topic),
                || format!("unknown topic `{topic}` of `{contact}`"),
            )?;
        }
        for quest in self.accepted.iter().chain(self.restarts.keys()) {
            ensure(c.quest(quest).is_some(), || {
                format!("unknown quest `{quest}`")
            })?;
        }
        Ok(())
    }

    /// No turn is later than the clock, and the heat frontier is a frontier.
    fn validate_turns(&self) -> Result<(), String> {
        let clock = self.clock;
        let later = |what: &str, at: Turn| {
            ensure(at <= clock, || {
                format!("{what} at turn {at} is later than the clock ({clock})")
            })
        };
        later("heat reading", self.heat_now.0)?;
        for (site, at) in &self.compromised {
            later(&format!("breach of `{site}`"), *at)?;
        }
        for (contact, at) in &self.talks {
            later(&format!("conversation with `{contact}`"), *at)?;
        }
        for (at, _) in &self.payments {
            later("payment", *at)?;
        }
        for (quest, at) in &self.restarts {
            later(&format!("restart of `{quest}`"), *at)?;
        }
        for (quest, run) in &self.quests {
            later(&format!("opening of `{quest}`"), run.opened_at)?;
        }
        let mut lower_bound: Option<u8> = None;
        for (at, heat) in &self.heat_peaks {
            later("heat peak", *at)?;
            ensure(*heat <= HEAT_MAX, || {
                format!("heat peak {heat} is above {HEAT_MAX}")
            })?;
            ensure(lower_bound.is_none_or(|previous| *heat < previous), || {
                format!("heat peaks must strictly decrease with the turns (turn {at})")
            })?;
            lower_bound = Some(*heat);
        }
        Ok(())
    }

    fn validate_flags(&self, c: &Content) -> Result<(), String> {
        for (id, value) in &self.flags {
            let def = c.flag(id).ok_or_else(|| format!("unknown flag `{id}`"))?;
            let fits = match (def.kind, value) {
                (FlagKind::Bitset, FlagValue::Bits(bits)) => {
                    bits.iter().all(|bit| def.values.contains(bit))
                }
                (FlagKind::Bitset, _) => false,
                _ => flag_accepts(def, value),
            };
            ensure(fits, || {
                format!("value {value:?} does not fit flag `{id}` ({:?})", def.kind)
            })?;
        }
        Ok(())
    }

    fn validate_quests(&self, c: &Content) -> Result<(), String> {
        for (id, run) in &self.quests {
            let def = c.quest(id).ok_or_else(|| format!("unknown quest `{id}`"))?;
            for &(i, j) in &run.done {
                let objective = def.objective.get(usize::from(i));
                let exists = objective.is_some_and(|o| {
                    j == 0 || o.children().get(usize::from(j).saturating_sub(1)).is_some()
                });
                ensure(exists, || {
                    format!("quest `{id}` has no objective ({i}, {j})")
                })?;
            }
            let started = matches!(
                run.status,
                QuestStatus::Active | QuestStatus::Completed | QuestStatus::Failed
            );
            ensure(started || run.done.is_empty(), || {
                format!(
                    "quest `{id}` is {:?} but has achieved objectives",
                    run.status
                )
            })?;
            ensure(run.status != QuestStatus::Failed || def.failable, || {
                format!("quest `{id}` failed but cannot fail")
            })?;
            // A status is only ever set together with its claim: the two are one step.
            let claimed = |key: String| self.claimed.contains(&key);
            ensure(!started || claimed(format!("open-{id}")), || {
                format!("quest `{id}` was opened without its claim")
            })?;
            ensure(
                run.status != QuestStatus::Completed || claimed(format!("quest-{id}")),
                || format!("quest `{id}` is completed without its claim"),
            )?;
            ensure(
                run.status != QuestStatus::Failed || claimed(format!("quest-{id}-failed")),
                || format!("quest `{id}` failed without its claim"),
            )?;
        }
        Ok(())
    }

    /// The anti-farm ledger holds keys the content can produce, and what a claim says happened
    /// happened.
    fn validate_claims(&self, c: &Content) -> Result<(), String> {
        // Every key the content can produce, with whether its source did happen.
        let mut happened: BTreeMap<String, bool> = BTreeMap::new();
        let mut note = |key: String, did: bool| {
            happened
                .entry(key)
                .and_modify(|known| *known |= did)
                .or_insert(did);
        };
        for site in c.sites.iter().filter(|s| s.first_breach.is_some()) {
            note(
                format!("breach-{}", site.id),
                self.compromised.contains_key(&site.id),
            );
        }
        for q in &c.quests {
            let status = self.status(&q.id);
            note(format!("quest-{}", q.id), status == QuestStatus::Completed);
            note(
                format!("open-{}", q.id),
                !matches!(status, QuestStatus::Unavailable | QuestStatus::Available),
            );
            if q.failable {
                note(
                    format!("quest-{}-failed", q.id),
                    status == QuestStatus::Failed,
                );
            }
        }
        for d in &c.decisions {
            note(
                format!("decision-{}", d.id),
                self.decisions.contains_key(&d.id),
            );
        }
        for t in &c.topics {
            note(
                format!("topic-{}-{}", t.contact, t.id),
                self.topics.contains(&(t.contact.clone(), t.id.clone())),
            );
        }
        for key in &self.claimed {
            match happened.get(key) {
                None => {
                    return Err(format!(
                        "claim `{key}` does not belong to anything in the content"
                    ));
                }
                Some(false) => {
                    return Err(format!(
                        "claim `{key}` is recorded but what it pays for did not happen"
                    ));
                }
                Some(true) => {}
            }
        }
        Ok(())
    }
}
