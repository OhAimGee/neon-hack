//! What the optimistic player types next.
//!
//! The content-level optimistic player (`tests/content/optimist.rs`) walks the language by
//! giving it facts. This one has to find, for each thing the quests ask, a command the game
//! really offers. It decides like a player who reads the journal: the objectives of the
//! active quests (the public mission state and the content) say what to do, and the game says
//! what it offers (`Game::complete`, the same list the TAB key shows): a move whose command
//! is not offered right now is not made. A thing that is wanted but never offered is a dead
//! end, and the walk reports it.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use neon_engine::Game;
use neon_engine::campaign::CampaignGame;
use neon_engine::content::engine::{available_topics, offered_choices};
use neon_engine::content::eval::{applicable, holds};
use neon_engine::content::ids::{
    ChoiceId, ContactId, DecisionId, ItemId, QuestId, ReadableId, ServiceId, SiteId,
};
use neon_engine::content::schema::{
    ExtractWhat, Goal, Objective, QuestDef, QuestKind, QuestStatus, ReadableKind, Turn,
};
use neon_engine::content::{Content, State};

/// How the optimistic player behaves: the option of each decision, and whether it also does
/// the optional objectives.
#[derive(Debug, Clone, Default)]
pub(crate) struct Plan {
    pub(crate) choices: BTreeMap<DecisionId, ChoiceId>,
    pub(crate) optional: bool,
}

impl Plan {
    /// A short name for a failure message.
    pub(crate) fn name(&self) -> String {
        let choices: Vec<String> = self
            .choices
            .iter()
            .map(|(decision, choice)| format!("{decision}={choice}"))
            .collect();
        format!(
            "{}{}",
            choices.join(" "),
            if self.optional { " +optional" } else { "" }
        )
    }
}

/// Every combination of the options of every decision, with and without the optional
/// objectives: the 216 plans of the content-level player.
pub(crate) fn all_plans(c: &Content) -> Vec<Plan> {
    let mut plans: Vec<BTreeMap<DecisionId, ChoiceId>> = vec![BTreeMap::new()];
    for decision in &c.decisions {
        plans = plans
            .into_iter()
            .flat_map(|plan| {
                decision.choice.iter().map(move |choice| {
                    let mut next = plan.clone();
                    next.insert(decision.id.clone(), choice.id.clone());
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
            })
        })
        .collect()
}

/// One thing the player does at the command prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Move {
    Accept(QuestId),
    /// A conversation, and what to do in its menu besides taking the topics offered: pay for
    /// a quest, or decide.
    Talk {
        contact: ContactId,
        pay: Option<QuestId>,
        decide: Option<(DecisionId, ChoiceId)>,
    },
    Buy(ItemId),
    Hack(SiteId),
    /// A message (`read`) or a fragment (`archives`).
    Read(ReadableId),
    Decrypt(ReadableId),
    Link(ContactId),
    /// A command typed bare (`help`, `net`...).
    Use(&'static str),
    Cool(ServiceId),
    /// Any command that makes a turn pass, so that a quest opened this turn can conclude.
    Tick,
}

pub(crate) struct Brain<'a> {
    game: &'a CampaignGame,
    c: &'a Content,
    s: &'a State,
    plan: &'a Plan,
}

/// The notoriety at which the stall closes (the band "hunted").
const MARKET_CLOSES: u32 = 80;

/// The game command that stands for a command of the content language.
fn game_command(content_command: &str) -> Option<&'static str> {
    Some(match content_command {
        "quests" => "quests",
        "help" => "help",
        "status" => "status",
        "scan" => "net",
        "laylow" => "laylow",
        _ => return None,
    })
}

impl<'a> Brain<'a> {
    pub(crate) fn new(game: &'a CampaignGame, plan: &'a Plan) -> Self {
        Self {
            game,
            c: game.content(),
            s: game.state().missions(),
            plan,
        }
    }

    /// Whether the game offers `command` with this argument now: what TAB would complete.
    fn offers(&self, command: &str, argument: &str) -> bool {
        self.game
            .complete(&format!("{command} "))
            .iter()
            .any(|offered| offered == argument)
    }

    /// Whether the game offers the command itself now.
    fn offers_command(&self, command: &str) -> bool {
        self.game
            .complete(command)
            .iter()
            .any(|offered| offered == command)
    }

    /// The next thing to do, or `None` when nothing is left.
    pub(crate) fn next_move(&self) -> Option<Move> {
        self.free()
            .or_else(|| self.work(|q| q.kind != QuestKind::Main))
            .or_else(|| self.work(|q| q.kind == QuestKind::Main))
            .or_else(|| self.cool())
            .or_else(|| self.tick())
    }

    // ------------------------------------------------------------------ free moves

    /// What costs nothing and can only help: accepting offers, reading what arrived, hearing
    /// every topic of every contact.
    fn free(&self) -> Option<Move> {
        for q in &self.c.quests {
            if self.s.status(&q.id) == QuestStatus::Available
                && self.offers("accept", q.id.as_str())
            {
                return Some(Move::Accept(q.id.clone()));
            }
        }
        for r in &self.c.readables {
            if !self.s.opened.contains(&r.id) && self.s.obtained(self.c, &r.id) {
                let command = match r.kind {
                    ReadableKind::Mail => "read",
                    ReadableKind::Fragment => "archives",
                    ReadableKind::Document | ReadableKind::Scene => continue,
                };
                if self.offers(command, r.id.as_str()) {
                    return Some(Move::Read(r.id.clone()));
                }
            }
        }
        for contact in &self.c.contacts {
            if !available_topics(self.c, self.s, &contact.id).is_empty()
                && self.offers("talk", contact.id.as_str())
            {
                return Some(Move::Talk {
                    contact: contact.id.clone(),
                    pay: None,
                    decide: None,
                });
            }
        }
        None
    }

    // -------------------------------------------------------------------- the quests

    /// The first thing that moves a quest of the kind forward.
    fn work(&self, kind: impl Fn(&QuestDef) -> bool) -> Option<Move> {
        self.c
            .quests
            .iter()
            .filter(|q| kind(q) && self.s.status(&q.id) == QuestStatus::Active)
            .find_map(|q| self.quest(q))
    }

    fn quest(&self, q: &QuestDef) -> Option<Move> {
        let run = self.s.quests.get(&q.id)?;
        for (i, o) in q.objective.iter().enumerate() {
            let at = u8::try_from(i).ok()?;
            let latched = run.done.contains(&(at, 0));
            let wants_alternative = o.children().iter().enumerate().any(|(j, ch)| {
                self.prefers(ch.id.as_deref())
                    && applicable(self.c, self.s, ch)
                    && u8::try_from(j + 1).is_ok_and(|j| !run.done.contains(&(at, j)))
            });
            if latched && !o.goal.is_condition() && !wants_alternative {
                continue;
            }
            if (o.optional && !self.plan.optional) || !applicable(self.c, self.s, o) {
                continue;
            }
            if let Some(found) = self.pursue(q, run.opened_at, o) {
                return Some(found);
            }
        }
        None
    }

    /// Whether an alternative is the one the plan's decisions lead to (`pay`, `betray`...).
    fn prefers(&self, id: Option<&str>) -> bool {
        id.is_some_and(|id| {
            self.plan
                .choices
                .values()
                .any(|choice| id.starts_with(choice.as_str()))
        })
    }

    /// What the objective asks, as a move, when the game offers it now.
    fn pursue(&self, q: &QuestDef, opened_at: Turn, o: &Objective) -> Option<Move> {
        let is_any = matches!(o.goal, Goal::AnyOf(_));
        if holds(self.c, self.s, q, opened_at, o) && !o.goal.is_condition() && !is_any {
            return None;
        }
        match &o.goal {
            Goal::Talk { contact, count } => {
                let have = self
                    .s
                    .talks
                    .iter()
                    .filter(|(who, t)| who == contact && *t > opened_at)
                    .count();
                let need = usize::try_from(*count).unwrap_or(0);
                (have < need && self.offers("talk", contact.as_str())).then(|| Move::Talk {
                    contact: contact.clone(),
                    pay: None,
                    decide: None,
                })
            }
            Goal::Buy { item } => self.buy(item),
            Goal::Compromise {
                site: Some(site), ..
            }
            | Goal::SiteState { site, .. } => self.hack(site),
            Goal::Compromise { site: None, count } => {
                let have: Vec<&SiteId> = self
                    .s
                    .compromised
                    .iter()
                    .filter(|(_, t)| **t > opened_at)
                    .map(|(site, _)| site)
                    .collect();
                let need = usize::try_from(*count)
                    .unwrap_or(0)
                    .saturating_sub(have.len());
                self.c
                    .sites
                    .iter()
                    .filter(|d| !have.contains(&&d.id) && self.s.unlocked(&d.unlock))
                    .take(need)
                    .find_map(|d| self.hack(&d.id))
            }
            Goal::Extract { site, what } => self.extract(site.as_ref(), what),
            Goal::Open { readable } => self.open(readable, 0),
            Goal::Link { contact, level } => {
                let have = self.s.links.get(contact).copied().unwrap_or(0);
                (have < *level && self.offers("link", contact.as_str()))
                    .then(|| Move::Link(contact.clone()))
            }
            Goal::Use { command, group } => {
                let wanted = command
                    .as_ref()
                    .map(neon_engine::content::ids::CommandId::as_str);
                self.c
                    .commands
                    .iter()
                    .filter(|d| {
                        wanted.is_some_and(|w| d.id.as_str() == w)
                            || (wanted.is_none() && d.group.as_ref() == group.as_ref())
                    })
                    .filter_map(|d| game_command(d.id.as_str()))
                    .find(|name| self.offers_command(name))
                    .map(Move::Use)
            }
            Goal::Pay { amount } => {
                let price = self.c.resolve(*amount, q.tier).0;
                (self.game.credits() >= price.get() && self.offers("talk", q.giver.as_str())).then(
                    || Move::Talk {
                        contact: q.giver.clone(),
                        pay: Some(q.id.clone()),
                        decide: None,
                    },
                )
            }
            Goal::Choice { decision } => self.decide(decision),
            Goal::AnyOf(of) => {
                let (mut first, rest): (Vec<_>, Vec<_>) =
                    of.iter().partition(|ch| self.prefers(ch.id.as_deref()));
                first.extend(rest);
                for ch in first {
                    if !applicable(self.c, self.s, ch) {
                        continue;
                    }
                    if holds(self.c, self.s, q, opened_at, ch) {
                        return None;
                    }
                    if let Some(found) = self.pursue(q, opened_at, ch) {
                        return Some(found);
                    }
                }
                None
            }
            // Cooling is a move of its own, made last; the others are not things to do.
            Goal::HeatEndBelow { .. }
            | Goal::HeatPeakBelow { .. }
            | Goal::Reputation { .. }
            | Goal::Invalid(_) => None,
        }
    }

    fn buy(&self, item: &ItemId) -> Option<Move> {
        if self.s.owned.contains(item) {
            return None;
        }
        if self.offers("buy", item.as_str()) {
            return Some(Move::Buy(item.clone()));
        }
        // The stall closes when the notoriety is too high: lying low reopens it.
        let def = self.c.item(item)?;
        let could = self.s.unlocked(&def.unlock) && self.game.credits() >= def.price.get();
        if could && u32::from(self.game.notoriety()) >= MARKET_CLOSES {
            return self.cool_below(MARKET_CLOSES);
        }
        None
    }

    /// An intrusion that brings `site` closer to being compromised: its relays first.
    fn hack(&self, site: &SiteId) -> Option<Move> {
        let mut chain = Vec::new();
        let mut cur = site.clone();
        while let Some(d) = self.c.site(&cur) {
            if !self.s.unlocked(&d.unlock) || chain.len() > self.c.sites.len() {
                return None;
            }
            chain.push(cur.clone());
            match &d.relay {
                Some(r) if !self.s.compromised.contains_key(r) => cur = r.clone(),
                _ => break,
            }
        }
        let target = chain.pop()?;
        self.offers("hack", target.as_str())
            .then_some(Move::Hack(target))
    }

    fn extract(&self, site: Option<&SiteId>, what: &ExtractWhat) -> Option<Move> {
        let c = self.c;
        let sites: Vec<SiteId> = match (site, what) {
            (Some(site), ExtractWhat::File(_) | ExtractWhat::All) => vec![site.clone()],
            (Some(site), ExtractWhat::Count(n)) => {
                let have = self.s.extracted.iter().filter(|(st, _)| st == site).count();
                if have < usize::try_from(*n).unwrap_or(0) {
                    vec![site.clone()]
                } else {
                    Vec::new()
                }
            }
            (None, ExtractWhat::Count(n)) => {
                let need = usize::try_from(*n)
                    .unwrap_or(0)
                    .saturating_sub(self.s.extracted.len());
                c.sites
                    .iter()
                    .filter(|d| self.s.unlocked(&d.unlock))
                    .flat_map(|d| d.file.iter().map(|f| (d.id.clone(), f.id.clone())))
                    .filter(|key| !self.s.extracted.contains(key))
                    .take(need)
                    .map(|(site, _)| site)
                    .collect()
            }
            (None, _) => Vec::new(),
        };
        sites.iter().find_map(|site| self.hack(site))
    }

    /// Reads a readable, going to fetch it, buying the key or decrypting it when needed.
    fn open(&self, id: &ReadableId, depth: usize) -> Option<Move> {
        let c = self.c;
        let def = c.readable(id)?;
        if self.s.opened.contains(id) || depth > c.readables.len() {
            return None;
        }
        if !self.s.obtained(c, id) {
            let site = c.sites.iter().find_map(|s| {
                s.file
                    .iter()
                    .any(|f| f.yields.as_ref() == Some(id))
                    .then(|| s.id.clone())
            });
            if let Some(site) = site {
                return self.hack(&site);
            }
            let doc = c.readables.iter().find(|d| d.yields.as_ref() == Some(id))?;
            return self.open(&doc.id, depth + 1);
        }
        match (def.kind, &def.requires_item) {
            (ReadableKind::Document, Some(item)) if !self.s.owned.contains(item) => self.buy(item),
            (ReadableKind::Document, _) => self
                .offers("decrypt", id.as_str())
                .then(|| Move::Decrypt(id.clone())),
            (ReadableKind::Mail, _) => self
                .offers("read", id.as_str())
                .then(|| Move::Read(id.clone())),
            (ReadableKind::Fragment, _) => self
                .offers("archives", id.as_str())
                .then(|| Move::Read(id.clone())),
            (ReadableKind::Scene, _) => None,
        }
    }

    /// A decision is put by the giver of its quest, in the menu of the conversation.
    fn decide(&self, decision: &DecisionId) -> Option<Move> {
        let def = self.c.decision(decision)?;
        let giver = self.c.quest(&def.quest)?.giver.clone();
        let wanted = self.plan.choices.get(decision)?;
        let offered = offered_choices(self.c, self.s, decision);
        if !offered.iter().any(|ch| ch.id == *wanted) {
            return None;
        }
        self.offers("talk", giver.as_str()).then(|| Move::Talk {
            contact: giver,
            pay: None,
            decide: Some((decision.clone(), wanted.clone())),
        })
    }

    // ------------------------------------------------------------ cooling and ticking

    /// A cover service, when a quest wants the notoriety under a limit it is not under.
    fn cool(&self) -> Option<Move> {
        let limit = self
            .c
            .quests
            .iter()
            .filter(|q| self.s.status(&q.id) == QuestStatus::Active)
            .flat_map(|q| {
                q.objective
                    .iter()
                    .filter(|o| !o.optional || self.plan.optional)
                    .filter(|o| applicable(self.c, self.s, o))
                    .filter_map(|o| match o.goal {
                        Goal::HeatEndBelow { max } => Some(max),
                        _ => None,
                    })
            })
            .min()?;
        self.cool_below(limit)
    }

    /// A cover service that brings the notoriety under `limit`, or at least lowers it.
    fn cool_below(&self, limit: u32) -> Option<Move> {
        let notoriety = u32::from(self.game.notoriety());
        if notoriety < limit {
            return None;
        }
        let need = notoriety - limit + 1;
        let open: Vec<_> = self
            .c
            .services
            .iter()
            .filter(|svc| self.offers("laylow", svc.id.as_str()))
            .collect();
        // The cheapest service that is enough, else the one that cools the most.
        open.iter()
            .filter(|svc| u32::from(svc.cooling) >= need)
            .min_by_key(|svc| svc.price)
            .or_else(|| open.iter().max_by_key(|svc| svc.cooling))
            .map(|svc| Move::Cool(svc.id.clone()))
    }

    /// When a quest was opened this very turn and nothing else is left to do, any command
    /// that changes the world lets the turn pass.
    fn tick(&self) -> Option<Move> {
        let waiting = self
            .s
            .quests
            .values()
            .any(|run| run.status == QuestStatus::Active && run.opened_at >= self.s.clock);
        (waiting && self.offers("talk", "echo7")).then_some(Move::Tick)
    }

    /// What the walk could not do, for the failure message: the quests still open and why.
    pub(crate) fn describe_pending(&self) -> String {
        let mut out = String::new();
        for q in &self.c.quests {
            let status = self.s.status(&q.id);
            if matches!(status, QuestStatus::Active | QuestStatus::Available) {
                let _ = writeln!(out, "  {} is {status:?}", q.id);
                if let Some(run) = self.s.quests.get(&q.id) {
                    for (i, o) in q.objective.iter().enumerate() {
                        let done = holds(self.c, self.s, q, run.opened_at, o);
                        let _ = writeln!(
                            out,
                            "    {} #{}: {:?}{}",
                            if done { "[x]" } else { "[ ]" },
                            i + 1,
                            o.goal,
                            if o.optional { " (optional)" } else { "" }
                        );
                    }
                }
            }
        }
        let _ = writeln!(
            out,
            "  credits {} notoriety {} clock {}",
            self.game.credits(),
            self.game.notoriety(),
            self.s.clock
        );
        out
    }
}
