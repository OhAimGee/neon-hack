//! The "optimistic player": walks the content to a fixed point by performing everything that
//! is enabled, to prove there is no dead end (architecture dossier, section 8).
//!
//! It reads only the content and the state: it accepts offered quests, does what each active
//! objective asks (buying, breaching, extracting, decrypting, talking, paying, cooling the
//! heat down), reads everything it obtains, opens every dialogue topic and, for each
//! decision, picks the option of its [`Plan`]. It plays the engine's part for the two facts
//! the language cannot produce itself: a forced heat becomes a heat reading.

use std::collections::{BTreeMap, BTreeSet};

use neon_engine::content::engine::{Outcome, available_topics, new_game, offered_choices, refresh};
use neon_engine::content::eval::{applicable, holds};
use neon_engine::content::ids::{
    ChoiceId, DecisionId, FileId, ItemId, QuestId, ReadableId, SiteId,
};
use neon_engine::content::schema::{
    ExtractWhat, Goal, Objective, QuestDef, QuestKind, QuestStatus, ReadableKind, Turn,
};
use neon_engine::content::{Content, Fact, State};

/// Safety net: no content of the campaign needs more rounds than this.
const MAX_ROUNDS: usize = 400;

/// How the optimistic player behaves.
#[derive(Debug, Clone, Default)]
pub(crate) struct Plan {
    /// The option to pick for each decision (the first offered one when absent).
    pub(crate) choices: BTreeMap<DecisionId, ChoiceId>,
    /// Also do the optional objectives.
    pub(crate) optional: bool,
    /// Offered quests that are never accepted.
    pub(crate) skip: BTreeSet<QuestId>,
}

/// What the walk ended with.
#[derive(Debug, Clone)]
pub(crate) struct Report {
    pub(crate) state: State,
    pub(crate) outcomes: Vec<Outcome>,
    pub(crate) rounds: usize,
    /// The facts of each round, in order: a legal history to replay or shuffle in tests.
    pub(crate) log: Vec<Vec<Fact>>,
    /// Quests still active or offered (and not skipped) when nothing more could be done.
    pub(crate) stuck: Vec<QuestId>,
    /// Whether, after any round, the purchases and payments cost more than the credits
    /// earned: the player overspent.
    pub(crate) overdrawn: bool,
}

/// Every combination of the options of every decision, with and without the optional
/// objectives: the plans the "no dead end" invariant must hold for.
pub(crate) fn all_plans(c: &Content) -> Vec<Plan> {
    let mut plans = vec![BTreeMap::new()];
    for d in &c.decisions {
        plans = plans
            .into_iter()
            .flat_map(|p: BTreeMap<DecisionId, ChoiceId>| {
                d.choice.iter().map(move |ch| {
                    let mut next = p.clone();
                    next.insert(d.id.clone(), ch.id.clone());
                    next
                })
            })
            .collect();
    }
    plans
        .into_iter()
        .flat_map(|choices| {
            [true, false].map(|optional| Plan {
                choices: choices.clone(),
                optional,
                skip: BTreeSet::new(),
            })
        })
        .collect()
}

/// Plays the whole content with the plan.
pub(crate) fn play(c: &Content, plan: &Plan) -> Report {
    let (s, outcomes) = new_game(c);
    play_from(c, plan, s, outcomes)
}

/// Plays from a given state; `fresh` is what the last refresh returned.
pub(crate) fn play_from(
    c: &Content,
    plan: &Plan,
    mut s: State,
    mut outcomes: Vec<Outcome>,
) -> Report {
    let mut now: Turn = s.clock;
    let mut rounds = 0;
    let mut fresh = outcomes.clone();
    let mut overdrawn = s.overdrawn(c);
    let mut log: Vec<Vec<Fact>> = Vec::new();
    while rounds < MAX_ROUNDS {
        rounds += 1;
        let mut w = Walker {
            c,
            plan,
            s: &s,
            now: &mut now,
            facts: Vec::new(),
            reserved: 0,
        };
        w.round(&fresh);
        let mut facts = w.facts;
        if facts.is_empty() {
            // A quest opened this very turn cannot conclude yet: let a turn pass once.
            if s.quests
                .values()
                .any(|r| r.status == QuestStatus::Active && r.opened_at >= s.clock)
            {
                now += 1;
                facts.push(Fact::Tick { at: now });
            } else {
                break;
            }
        }
        log.push(facts.clone());
        let mut before = s.clone();
        for f in &facts {
            s.apply(c, f);
        }
        fresh = refresh(c, &mut s);
        outcomes.extend(fresh.clone());
        overdrawn |= s.overdrawn(c);
        before.clock = s.clock;
        if s == before {
            break;
        }
    }
    let stuck = c
        .quests
        .iter()
        .filter(|q| match s.status(&q.id) {
            QuestStatus::Active => true,
            QuestStatus::Available => !plan.skip.contains(&q.id),
            _ => false,
        })
        .map(|q| q.id.clone())
        .collect();
    Report {
        state: s,
        outcomes,
        rounds,
        log,
        stuck,
        overdrawn,
    }
}

struct Walker<'a> {
    c: &'a Content,
    plan: &'a Plan,
    s: &'a State,
    now: &'a mut Turn,
    facts: Vec<Fact>,
    /// Credits already committed by the facts queued in this round.
    reserved: i64,
}

impl Walker<'_> {
    fn next(&mut self) -> Turn {
        *self.now += 1;
        *self.now
    }

    fn round(&mut self, fresh: &[Outcome]) {
        let c = self.c;
        for o in fresh {
            if let Outcome::HeatForced(v) = o {
                let at = self.next();
                self.facts.push(Fact::HeatChanged { heat: *v, at });
            }
        }
        // Contracts first: an optimistic player earns the advantages before the finale. The
        // main line only moves when no contract can progress.
        for q in c.quests.iter().filter(|q| q.kind != QuestKind::Main) {
            self.pursue_quest(q);
        }
        if self.facts.is_empty()
            || self
                .facts
                .iter()
                .all(|f| matches!(f, Fact::HeatChanged { .. }))
        {
            for q in c.quests.iter().filter(|q| q.kind == QuestKind::Main) {
                self.pursue_quest(q);
            }
        }
        for r in &c.readables {
            if r.kind != ReadableKind::Document
                && !self.s.opened.contains(&r.id)
                && self.s.obtained(c, &r.id)
            {
                self.facts.push(Fact::Read { id: r.id.clone() });
            }
        }
        for contact in &c.contacts {
            for t in available_topics(c, self.s, &contact.id) {
                self.facts.push(Fact::TopicChosen {
                    contact: contact.id.clone(),
                    topic: t.id.clone(),
                });
            }
        }
    }

    fn pursue_quest(&mut self, q: &QuestDef) {
        match self.s.status(&q.id) {
            QuestStatus::Available if !self.plan.skip.contains(&q.id) => {
                self.facts.push(Fact::QuestAccepted {
                    quest: q.id.clone(),
                });
            }
            QuestStatus::Active => self.quest(q),
            _ => {}
        }
    }

    fn quest(&mut self, q: &QuestDef) {
        let Some(run) = self.s.quests.get(&q.id) else {
            return;
        };
        for (i, o) in q.objective.iter().enumerate() {
            let latched = u8::try_from(i).is_ok_and(|i| run.done.contains(&(i, 0)));
            let wants_alternative = o.children().iter().enumerate().any(|(j, ch)| {
                self.prefers(ch.id.as_deref())
                    && applicable(self.c, self.s, ch)
                    && !u8::try_from(j + 1)
                        .is_ok_and(|j| u8::try_from(i).is_ok_and(|i| run.done.contains(&(i, j))))
            });
            if latched && !o.goal.is_condition() && !wants_alternative {
                continue;
            }
            if (o.optional && !self.plan.optional) || !applicable(self.c, self.s, o) {
                continue;
            }
            self.pursue(q, run.opened_at, o);
        }
    }

    /// Does what the objective asks, when it can be done now. Returns whether it did anything.
    fn pursue(&mut self, q: &QuestDef, opened_at: Turn, o: &Objective) -> bool {
        let before = self.facts.len();
        let is_any = matches!(o.goal, Goal::AnyOf(_));
        if holds(self.c, self.s, q, opened_at, o) && !o.goal.is_condition() && !is_any {
            return false;
        }
        match &o.goal {
            Goal::Talk { contact, count } => {
                let have = self
                    .s
                    .talks
                    .iter()
                    .filter(|(who, t)| who == contact && *t > opened_at)
                    .count();
                if self.s.contact(contact).reachable() {
                    for _ in have..usize::try_from(*count).unwrap_or(0) {
                        let at = self.next();
                        self.facts.push(Fact::Talked {
                            contact: contact.clone(),
                            at,
                        });
                    }
                }
            }
            Goal::Buy { item } => self.buy(item),
            Goal::Compromise {
                site: Some(site), ..
            } => self.breach(site),
            Goal::Compromise { site: None, count } => {
                let have: BTreeSet<&SiteId> = self
                    .s
                    .compromised
                    .iter()
                    .filter(|(_, t)| **t > opened_at)
                    .map(|(s, _)| s)
                    .collect();
                let need = usize::try_from(*count)
                    .unwrap_or(0)
                    .saturating_sub(have.len());
                let picks: Vec<SiteId> = self
                    .c
                    .sites
                    .iter()
                    .filter(|d| !have.contains(&d.id) && self.s.unlocked(&d.unlock))
                    .take(need)
                    .map(|d| d.id.clone())
                    .collect();
                for site in picks {
                    self.breach(&site);
                }
            }
            Goal::Extract { site, what } => self.extract(site.as_ref(), what),
            Goal::Open { readable } => self.open(readable, 0),
            Goal::Link { contact, level } => {
                if self.s.contact(contact).reachable() {
                    self.facts.push(Fact::LinkChanged {
                        contact: contact.clone(),
                        level: *level,
                    });
                }
            }
            Goal::Use { command, group } => {
                let cmd = command.clone().or_else(|| {
                    self.c
                        .commands
                        .iter()
                        .find(|d| d.group.as_ref() == group.as_ref())
                        .map(|d| d.id.clone())
                });
                if let Some(command) = cmd {
                    self.facts.push(Fact::CommandUsed { command });
                }
            }
            Goal::SiteState { site, mark } => {
                if self.ensure_breached(site) {
                    self.facts.push(Fact::SiteMarked {
                        site: site.clone(),
                        mark: *mark,
                    });
                }
            }
            Goal::Pay { amount } => {
                let price = self.c.resolve(*amount, q.tier).0;
                if self.spendable() >= i64::from(price.get()) {
                    self.reserved += i64::from(price.get());
                    let at = self.next();
                    self.facts.push(Fact::Paid { amount: price, at });
                }
            }
            Goal::Choice { decision } => self.choose(decision),
            Goal::HeatEndBelow { max } => {
                if u32::from(self.s.heat()) >= *max {
                    let at = self.next();
                    self.facts.push(Fact::HeatChanged { heat: 0, at });
                }
            }
            Goal::AnyOf(of) => {
                let (mut first, rest): (Vec<_>, Vec<_>) =
                    of.iter().partition(|ch| self.prefers(ch.id.as_deref()));
                first.extend(rest);
                for ch in first {
                    if !applicable(self.c, self.s, ch) {
                        continue;
                    }
                    if holds(self.c, self.s, q, opened_at, ch) || self.pursue(q, opened_at, ch) {
                        break;
                    }
                }
            }
            Goal::Reputation { .. } | Goal::HeatPeakBelow { .. } | Goal::Invalid(_) => {}
        }
        self.facts.len() > before
    }

    /// Whether an alternative is the one the plan's decisions lead to (`pay`, `betray`...).
    fn prefers(&self, id: Option<&str>) -> bool {
        id.is_some_and(|id| {
            self.plan
                .choices
                .values()
                .any(|ch| id.starts_with(ch.as_str()))
        })
    }

    fn buy(&mut self, item: &ItemId) {
        let Some(def) = self.c.item(item) else { return };
        let queued = self
            .facts
            .iter()
            .any(|f| matches!(f, Fact::ItemBought { item: i } if i == item));
        if !self.s.owned.contains(item)
            && !queued
            && self.s.unlocked(&def.unlock)
            && self.spendable() >= i64::from(def.price.get())
        {
            self.reserved += i64::from(def.price.get());
            self.facts.push(Fact::ItemBought { item: item.clone() });
        }
    }

    /// Credits left once the costs already queued in this round are counted.
    fn spendable(&self) -> i64 {
        i64::from(self.s.credits(self.c).get()) - self.reserved
    }

    /// Compromises a site, its relays first. Nothing happens when a gate is closed.
    fn breach(&mut self, site: &SiteId) {
        let mut chain = Vec::new();
        let mut cur = site.clone();
        while let Some(d) = self.c.site(&cur) {
            if !self.s.unlocked(&d.unlock) || chain.len() > self.c.sites.len() {
                return;
            }
            chain.push(cur.clone());
            match &d.relay {
                Some(r) if !self.s.compromised.contains_key(r) => cur = r.clone(),
                _ => break,
            }
        }
        for id in chain.into_iter().rev() {
            let at = self.next();
            self.facts.push(Fact::SiteCompromised { site: id, at });
        }
    }

    /// Whether the site has been compromised; otherwise starts breaching it.
    fn ensure_breached(&mut self, site: &SiteId) -> bool {
        if self.s.compromised.contains_key(site) {
            return true;
        }
        self.breach(site);
        false
    }

    fn extract(&mut self, site: Option<&SiteId>, what: &ExtractWhat) {
        let c = self.c;
        let todo: Vec<(SiteId, FileId)> = match (site, what) {
            (Some(site), ExtractWhat::File(f)) => vec![(site.clone(), f.clone())],
            (Some(site), ExtractWhat::All) => c
                .site(site)
                .map(|d| {
                    d.file
                        .iter()
                        .map(|f| (site.clone(), f.id.clone()))
                        .collect()
                })
                .unwrap_or_default(),
            (Some(site), ExtractWhat::Count(n)) => {
                let have = self.s.extracted.iter().filter(|(st, _)| st == site).count();
                let need = usize::try_from(*n).unwrap_or(0).saturating_sub(have);
                self.unextracted(Some(site), need)
            }
            (None, ExtractWhat::Count(n)) => {
                let need = usize::try_from(*n)
                    .unwrap_or(0)
                    .saturating_sub(self.s.extracted.len());
                self.unextracted(None, need)
            }
            (None, _) => Vec::new(),
        };
        for (site, file) in todo {
            if self.ensure_breached(&site) {
                self.facts.push(Fact::FileExtracted { site, file });
            }
        }
    }

    /// Files not extracted yet, of one site or of any site open to the player.
    fn unextracted(&self, only: Option<&SiteId>, n: usize) -> Vec<(SiteId, FileId)> {
        self.c
            .sites
            .iter()
            .filter(|d| only.is_none_or(|o| *o == d.id) && self.s.unlocked(&d.unlock))
            .flat_map(|d| d.file.iter().map(|f| (d.id.clone(), f.id.clone())))
            .filter(|key| !self.s.extracted.contains(key))
            .take(n)
            .collect()
    }

    /// Reads a readable, decrypting it (and buying the key) or going to fetch it when needed.
    fn open(&mut self, id: &ReadableId, depth: usize) {
        let c = self.c;
        let Some(def) = c.readable(id) else { return };
        if self.s.opened.contains(id) || depth > c.readables.len() {
            return;
        }
        if !self.s.obtained(c, id) {
            let file = c.sites.iter().find_map(|s| {
                s.file
                    .iter()
                    .find(|f| f.yields.as_ref() == Some(id))
                    .map(|f| (s.id.clone(), f.id.clone()))
            });
            if let Some((site, file)) = file {
                if self.ensure_breached(&site) {
                    self.facts.push(Fact::FileExtracted { site, file });
                }
            } else if let Some(doc) = c.readables.iter().find(|d| d.yields.as_ref() == Some(id)) {
                self.open(&doc.id.clone(), depth + 1);
            }
            return;
        }
        match (def.kind, &def.requires_item) {
            (ReadableKind::Document, Some(item)) if !self.s.owned.contains(item) => self.buy(item),
            (ReadableKind::Document, _) => self.facts.push(Fact::Decrypted { id: id.clone() }),
            _ => self.facts.push(Fact::Read { id: id.clone() }),
        }
    }

    fn choose(&mut self, decision: &DecisionId) {
        let offered = offered_choices(self.c, self.s, decision);
        let wanted = self.plan.choices.get(decision);
        let pick = match wanted {
            Some(w) => offered.iter().find(|ch| ch.id == *w),
            None => offered.first(),
        };
        if let Some(ch) = pick {
            self.facts.push(Fact::DecisionMade {
                decision: decision.clone(),
                choice: ch.id.clone(),
            });
        }
    }
}
