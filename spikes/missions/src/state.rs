//! The game state the language reads, and the facts that feed it.
//!
//! [`State::apply`] only records facts in a *ledger* made of sets, maxima and a Pareto
//! frontier: it is idempotent and commutative. Everything that has consequences (statuses,
//! rewards, effects) happens in [`crate::engine::refresh`], which reads the ledger.

use std::collections::{BTreeMap, BTreeSet};

use crate::content::Content;
use crate::ids::{
    ChoiceId, CommandId, ContactId, DecisionId, FileId, FlagId, ItemId, QuestId, ReadableId,
    SiteId, TopicId,
};
use crate::schema::{ContactState, FlagKind, FlagValue, QuestStatus, SiteMark, Turn};

/// What the engine bus tells the mission language. Facts that can repeat (`Talked`, `Paid`,
/// `SiteCompromised`...) carry the turn they happened at, which makes them distinct values
/// and lets objectives count only what happened after a quest opened.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fact {
    SiteCompromised {
        site: SiteId,
        at: Turn,
    },
    FileExtracted {
        site: SiteId,
        file: FileId,
    },
    SiteMarked {
        site: SiteId,
        mark: SiteMark,
    },
    /// A fragment, mail or scene was read/seen.
    Read {
        id: ReadableId,
    },
    /// An encrypted document was decrypted.
    Decrypted {
        id: ReadableId,
    },
    ItemBought {
        item: ItemId,
    },
    Talked {
        contact: ContactId,
        at: Turn,
    },
    CommandUsed {
        command: CommandId,
    },
    LinkChanged {
        contact: ContactId,
        level: u8,
    },
    Paid {
        amount: u32,
        at: Turn,
    },
    HeatChanged {
        heat: u8,
        at: Turn,
    },
    DecisionMade {
        decision: DecisionId,
        choice: ChoiceId,
    },
    TopicChosen {
        contact: ContactId,
        topic: TopicId,
    },
    QuestAccepted {
        quest: QuestId,
    },
    QuestRestarted {
        quest: QuestId,
        at: Turn,
    },
    /// Time passes without anything else happening.
    Tick {
        at: Turn,
    },
}

impl Fact {
    /// The turn the fact happened at, when it has one.
    pub fn at(&self) -> Option<Turn> {
        match self {
            Fact::SiteCompromised { at, .. }
            | Fact::Talked { at, .. }
            | Fact::Paid { at, .. }
            | Fact::HeatChanged { at, .. }
            | Fact::QuestRestarted { at, .. }
            | Fact::Tick { at } => Some(*at),
            _ => None,
        }
    }
}

/// An objective path: `(index, 0)` is the objective, `(index, j + 1)` its j-th alternative.
pub type ObjKey = (u8, u8);

/// Progress of one quest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestRun {
    pub status: QuestStatus,
    /// Turn at which the quest became active; event objectives count after it.
    pub opened_at: Turn,
    /// Objectives achieved (latched; condition objectives are never latched).
    pub done: BTreeSet<ObjKey>,
}

/// Everything the language knows about the game. Deterministic collections only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub clock: Turn,
    pub tier: u8,
    pub reputation: i32,
    /// Credits earned so far (start + claimed grants); spending is derived.
    pub earned: i64,
    pub flags: BTreeMap<FlagId, FlagValue>,
    pub contacts: BTreeMap<ContactId, ContactState>,
    pub bonus_trust: BTreeMap<ContactId, i32>,
    /// Reward keys already paid (`quest-m05`, `breach-corp-server-01`...): the anti-farm ledger.
    pub claimed: BTreeSet<String>,
    pub owned: BTreeSet<ItemId>,
    /// Last turn each site was compromised.
    pub compromised: BTreeMap<SiteId, Turn>,
    pub extracted: BTreeSet<(SiteId, FileId)>,
    pub marks: BTreeSet<(SiteId, SiteMark)>,
    /// Read fragments/mails/scenes and decrypted documents.
    pub opened: BTreeSet<ReadableId>,
    /// Readables handed out by effects.
    pub granted: BTreeSet<ReadableId>,
    pub used: BTreeSet<CommandId>,
    pub links: BTreeMap<ContactId, u8>,
    pub talks: BTreeSet<(ContactId, Turn)>,
    pub payments: BTreeSet<(Turn, u32)>,
    /// Latest heat reading `(turn, heat)`.
    pub heat_now: (Turn, u8),
    /// Pareto frontier of the heat readings: turns increase, heats strictly decrease, so the
    /// peak since turn `t` is the heat of the first entry after `t`.
    pub heat_peaks: BTreeMap<Turn, u8>,
    pub decisions: BTreeMap<DecisionId, ChoiceId>,
    pub topics: BTreeSet<(ContactId, TopicId)>,
    pub accepted: BTreeSet<QuestId>,
    pub restarts: BTreeMap<QuestId, Turn>,
    pub quests: BTreeMap<QuestId, QuestRun>,
}

impl State {
    /// The state of a new game.
    pub fn new(c: &Content) -> Self {
        Self {
            clock: 0,
            tier: 1,
            reputation: 0,
            earned: i64::from(c.rules.start_credits),
            flags: BTreeMap::new(),
            contacts: c.contacts.iter().map(|d| (d.id.clone(), d.state)).collect(),
            bonus_trust: BTreeMap::new(),
            claimed: BTreeSet::new(),
            owned: BTreeSet::new(),
            compromised: BTreeMap::new(),
            extracted: BTreeSet::new(),
            marks: BTreeSet::new(),
            opened: BTreeSet::new(),
            granted: BTreeSet::new(),
            used: BTreeSet::new(),
            links: BTreeMap::new(),
            talks: BTreeSet::new(),
            payments: BTreeSet::new(),
            heat_now: (0, 0),
            heat_peaks: BTreeMap::new(),
            decisions: BTreeMap::new(),
            topics: BTreeSet::new(),
            accepted: BTreeSet::new(),
            restarts: BTreeMap::new(),
            quests: BTreeMap::new(),
        }
    }

    /// Status of a quest (`Unavailable` until the engine says otherwise).
    pub fn status(&self, q: &QuestId) -> QuestStatus {
        self.quests
            .get(q)
            .map_or(QuestStatus::Unavailable, |r| r.status)
    }

    /// Current value of a flag, with the declared default.
    pub fn flag(&self, c: &Content, id: &FlagId) -> Option<FlagValue> {
        if let Some(v) = self.flags.get(id) {
            return Some(v.clone());
        }
        let def = c.flag(id)?;
        Some(def.default.clone().unwrap_or_else(|| match def.kind {
            FlagKind::Bool => FlagValue::Bool(false),
            FlagKind::Enum => FlagValue::Str(def.values.first().cloned().unwrap_or_default()),
            FlagKind::Counter => FlagValue::Int(0),
            FlagKind::Bitset => FlagValue::Bits(BTreeSet::new()),
        }))
    }

    /// State of a contact.
    pub fn contact(&self, id: &ContactId) -> ContactState {
        self.contacts
            .get(id)
            .copied()
            .unwrap_or(ContactState::Offline)
    }

    /// Trust: starting value, `per_talk` per conversation, plus quest and choice bonuses.
    pub fn trust(&self, c: &Content, id: &ContactId) -> i32 {
        let base = c.contact(id).map_or(0, |d| d.trust);
        let talks = self.talks.iter().filter(|(who, _)| who == id).count();
        let talks = i32::try_from(talks).unwrap_or(i32::MAX);
        base.saturating_add(talks.saturating_mul(c.rules.per_talk))
            .saturating_add(self.bonus_trust.get(id).copied().unwrap_or(0))
    }

    /// Credits in hand: earned minus the price of owned items minus payments.
    pub fn credits(&self, c: &Content) -> i64 {
        let spent: i64 = self
            .owned
            .iter()
            .filter_map(|i| c.item(i))
            .map(|i| i64::from(i.price))
            .sum();
        let paid: i64 = self.payments.iter().map(|(_, a)| i64::from(*a)).sum();
        self.earned - spent - paid
    }

    /// Current heat.
    pub fn heat(&self) -> u8 {
        self.heat_now.1
    }

    /// Highest heat reading strictly after turn `t`.
    pub fn peak_since(&self, t: Turn) -> u8 {
        self.heat_peaks
            .range(t.saturating_add(1)..)
            .next()
            .map_or(0, |(_, h)| *h)
    }

    /// Whether a readable is in the player's hands: known from the start, handed out by an
    /// effect, or yielded by an extracted file or a decrypted document.
    pub fn obtained(&self, c: &Content, id: &ReadableId) -> bool {
        let Some(def) = c.readable(id) else {
            return false;
        };
        if (def.start && self.unlocked(&def.unlock)) || self.granted.contains(id) {
            return true;
        }
        let from_file = c.sites.iter().any(|s| {
            s.file.iter().any(|f| {
                f.yields.as_ref() == Some(id)
                    && self.extracted.contains(&(s.id.clone(), f.id.clone()))
            })
        });
        let from_doc = c
            .readables
            .iter()
            .any(|d| d.yields.as_ref() == Some(id) && self.opened.contains(&d.id));
        from_file || from_doc
    }

    /// Whether an `unlock` gate is satisfied.
    pub fn unlocked(&self, u: &crate::schema::Unlock) -> bool {
        u.tier.is_none_or(|t| self.tier >= t)
            && u.quest
                .as_ref()
                .is_none_or(|q| self.status(q) == QuestStatus::Completed)
    }

    /// Records a fact in the ledger. Unknown ids are ignored. Returns whether anything changed.
    /// Idempotent and commutative: sets, maxima and a Pareto frontier only.
    pub fn apply(&mut self, c: &Content, fact: &Fact) -> bool {
        let before = self.clock;
        if let Some(at) = fact.at() {
            self.clock = self.clock.max(at);
        }
        let changed = match fact {
            Fact::SiteCompromised { site, at } if c.site(site).is_some() => {
                raise(&mut self.compromised, site, *at)
            }
            Fact::FileExtracted { site, file }
                if c.site(site)
                    .is_some_and(|s| s.file.iter().any(|f| f.id == *file)) =>
            {
                self.extracted.insert((site.clone(), file.clone()))
            }
            Fact::SiteMarked { site, mark } if c.site(site).is_some() => {
                self.marks.insert((site.clone(), *mark))
            }
            Fact::Read { id } | Fact::Decrypted { id } if c.readable(id).is_some() => {
                self.opened.insert(id.clone())
            }
            Fact::ItemBought { item } if c.item(item).is_some() => self.owned.insert(item.clone()),
            Fact::Talked { contact, at } if c.contact(contact).is_some() => {
                self.talks.insert((contact.clone(), *at))
            }
            Fact::CommandUsed { command } if c.has_command(command.as_str()) => {
                self.used.insert(command.clone())
            }
            Fact::LinkChanged { contact, level } if c.contact(contact).is_some() => {
                raise(&mut self.links, contact, *level)
            }
            Fact::Paid { amount, at } => self.payments.insert((*at, *amount)),
            Fact::HeatChanged { heat, at } => self.record_heat(*at, *heat),
            Fact::DecisionMade { decision, choice } => self.record_decision(c, decision, choice),
            Fact::TopicChosen { contact, topic }
                if c.topics
                    .iter()
                    .any(|t| t.contact == *contact && t.id == *topic) =>
            {
                self.topics.insert((contact.clone(), topic.clone()))
            }
            Fact::QuestAccepted { quest } if c.quest(quest).is_some() => {
                self.accepted.insert(quest.clone())
            }
            Fact::QuestRestarted { quest, at } if c.quest(quest).is_some() => {
                raise(&mut self.restarts, quest, *at)
            }
            _ => false,
        };
        changed || before != self.clock
    }

    fn record_heat(&mut self, at: Turn, heat: u8) -> bool {
        let now = self.heat_now.max((at, heat));
        let changed = now != self.heat_now;
        self.heat_now = now;
        if self
            .heat_peaks
            .range(at..)
            .next()
            .is_some_and(|(_, h)| *h >= heat)
        {
            return changed;
        }
        let dominated: Vec<Turn> = self
            .heat_peaks
            .range(..=at)
            .filter(|(_, h)| **h <= heat)
            .map(|(t, _)| *t)
            .collect();
        for t in dominated {
            self.heat_peaks.remove(&t);
        }
        self.heat_peaks.insert(at, heat);
        true
    }

    fn record_decision(&mut self, c: &Content, decision: &DecisionId, choice: &ChoiceId) -> bool {
        let valid = c
            .decision(decision)
            .is_some_and(|d| d.choice.iter().any(|ch| ch.id == *choice));
        if !valid || self.claimed.contains(&format!("decision-{decision}")) {
            return false;
        }
        match self.decisions.get(decision) {
            Some(old) if old <= choice => false,
            _ => {
                self.decisions.insert(decision.clone(), choice.clone());
                true
            }
        }
    }
}

/// Raises `m[k]` to at least `v`; returns whether the map changed.
fn raise<K: Ord + Clone, V: Ord + Copy>(m: &mut BTreeMap<K, V>, k: &K, v: V) -> bool {
    match m.get_mut(k) {
        Some(e) if *e >= v => false,
        Some(e) => {
            *e = v;
            true
        }
        None => {
            m.insert(k.clone(), v);
            true
        }
    }
}
