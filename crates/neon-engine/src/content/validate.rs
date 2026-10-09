//! Content validation: everything a writer can get wrong, reported with a file and a line.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Display;

use crate::content::engine::ENDING_FLAG;
use crate::content::eval::find_objective;
use crate::content::ids::{FlagId, QuestId, valid_id};
use crate::content::money::Credits;
use crate::content::schema::{
    Block, Cond, Effect, FlagDef, FlagKind, FlagValue, Goal, MAX_TIER, Objective, QuestDef,
    QuestKind, ReadableKind, Size,
};
use crate::content::state::HEAT_MAX;
use crate::content::texts::required_keys;
use crate::content::{Content, Diagnostic, File};

/// Most objectives in a quest. The bible has M01 with 7 and M12 with 6; the journal shows one
/// line each and the 64x20 layout leaves about 12 lines for it, so 8 is the ceiling. A quest
/// that needs more is split, or its steps are grouped with `any_of`.
pub const MAX_OBJECTIVES: usize = 8;
/// Alternatives in an `any_of`.
pub const MAX_ALTERNATIVES: usize = 4;
/// Nesting of conditions (`all` / `any` / `not`).
pub const MAX_COND_DEPTH: usize = 6;

pub(crate) fn validate(c: &Content, check_texts: bool) -> Vec<Diagnostic> {
    let mut v = V {
        c,
        errs: Vec::new(),
    };
    v.catalog();
    v.flags();
    v.rewards();
    v.unlocks();
    v.quests();
    v.decisions();
    v.topics();
    v.endings();
    v.conflicts();
    v.sources_of_readables();
    if check_texts {
        v.texts();
    }
    v.errs
}

struct V<'a> {
    c: &'a Content,
    errs: Vec<Diagnostic>,
}

/// Who writes a flag: used to find writes that could depend on the order of events.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Writer {
    Quest(QuestId),
    Decision(String, QuestId),
    Topic,
}

impl V<'_> {
    fn err(&mut self, file: File, at: usize, msg: impl Into<String>) {
        self.errs.push(self.c.diag(file, at, msg));
    }

    fn refer(
        &mut self,
        file: File,
        at: usize,
        ctx: &str,
        what: &str,
        exists: bool,
        id: &dyn Display,
    ) {
        if !exists {
            self.err(file, at, format!("{ctx}: unknown {what} `{id}`"));
        }
    }

    fn unique<'b>(&mut self, file: File, what: &str, ids: impl Iterator<Item = (&'b str, usize)>) {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for (id, at) in ids {
            if !valid_id(id) {
                self.err(
                    file,
                    at,
                    format!("{what} id `{id}` must match [a-z0-9][a-z0-9_-]{{0,47}}"),
                );
            } else if !seen.insert(id) {
                self.err(file, at, format!("duplicate {what} id `{id}`"));
            }
        }
    }

    // ------------------------------------------------------------- catalog

    fn catalog(&mut self) {
        let c = self.c;
        let f = File::Catalog;
        self.unique(f, "site", c.sites.iter().map(|x| (x.id.as_str(), x.at)));
        self.unique(f, "item", c.items.iter().map(|x| (x.id.as_str(), x.at)));
        self.unique(
            f,
            "contact",
            c.contacts.iter().map(|x| (x.id.as_str(), x.at)),
        );
        self.unique(
            f,
            "readable",
            c.readables.iter().map(|x| (x.id.as_str(), x.at)),
        );
        self.unique(
            f,
            "command",
            c.commands.iter().map(|x| (x.id.as_str(), x.at)),
        );
        for s in &c.sites {
            let ctx = format!("site `{}`", s.id);
            self.unique(
                f,
                &format!("file of {ctx}"),
                s.file.iter().map(|x| (x.id.as_str(), s.at)),
            );
            if let Some(r) = &s.relay {
                self.refer(f, s.at, &ctx, "relay site", c.site(r).is_some(), r);
                let tier_of = |id: &crate::content::ids::SiteId| {
                    c.site(id).and_then(|x| x.unlock.tier).unwrap_or(1)
                };
                if c.site(r).is_some() && tier_of(r) > s.unlock.tier.unwrap_or(1) {
                    self.err(
                        f,
                        s.at,
                        format!("{ctx}: its relay `{r}` needs a higher tier than the site itself"),
                    );
                }
                let mut hops = 0;
                let mut cur = Some(r.clone());
                while let Some(id) = cur {
                    hops += 1;
                    if id == s.id || hops > c.sites.len() {
                        self.err(f, s.at, format!("{ctx}: relay cycle"));
                        break;
                    }
                    cur = c.site(&id).and_then(|x| x.relay.clone());
                }
            }
            self.unlock(f, s.at, &ctx, &s.unlock);
            for file in &s.file {
                if let Some(y) = &file.yields {
                    self.refer(
                        f,
                        s.at,
                        &format!("{ctx}, file `{}`", file.id),
                        "readable",
                        c.readable(y).is_some(),
                        y,
                    );
                }
            }
        }
        for i in &c.items {
            self.unlock(f, i.at, &format!("item `{}`", i.id), &i.unlock);
        }
        for r in &c.readables {
            let ctx = format!("readable `{}`", r.id);
            self.unlock(f, r.at, &ctx, &r.unlock);
            let doc = r.kind == ReadableKind::Document;
            if !doc && (r.requires_item.is_some() || r.yields.is_some()) {
                self.err(
                    f,
                    r.at,
                    format!("{ctx}: only documents can have `requires_item` or `yields`"),
                );
            }
            if let Some(i) = &r.requires_item {
                self.refer(f, r.at, &ctx, "item", c.item(i).is_some(), i);
            }
            if let Some(y) = &r.yields {
                self.refer(f, r.at, &ctx, "readable", c.readable(y).is_some(), y);
                if *y == r.id {
                    self.err(f, r.at, format!("{ctx}: yields itself"));
                }
            }
            if (r.kind == ReadableKind::Scene) != (r.paragraphs > 0) {
                self.err(
                    f,
                    r.at,
                    format!("{ctx}: `paragraphs` is required for scenes and only for them"),
                );
            }
        }
    }

    fn unlock(&mut self, file: File, at: usize, ctx: &str, u: &crate::content::schema::Unlock) {
        if u.tier.is_some_and(|t| !(1..=MAX_TIER).contains(&t)) {
            self.err(
                file,
                at,
                format!("{ctx}: unlock tier must be 1 to {MAX_TIER}"),
            );
        }
        if let Some(q) = &u.quest {
            self.refer(file, at, ctx, "quest", self.c.quest(q).is_some(), q);
        }
    }

    /// Every readable must come from somewhere (bible test T9).
    fn sources_of_readables(&mut self) {
        let c = self.c;
        let mut unlocked: BTreeSet<&crate::content::ids::ReadableId> = BTreeSet::new();
        for q in &c.quests {
            unlocks_in(&q.on_open, &mut unlocked);
            unlocks_in(&q.on_complete, &mut unlocked);
            unlocks_in(&q.on_fail, &mut unlocked);
        }
        for d in &c.decisions {
            for ch in &d.choice {
                unlocks_in(&ch.then, &mut unlocked);
            }
        }
        for t in &c.topics {
            unlocks_in(&t.then, &mut unlocked);
        }
        // What the player can really get: what is known at the start, handed out by an effect or
        // yielded by a file, then what the documents among those yield, and so on. Two
        // documents yielding each other, with no other source, are never obtained.
        let mut obtainable: BTreeSet<&crate::content::ids::ReadableId> = unlocked;
        obtainable.extend(c.readables.iter().filter(|r| r.start).map(|r| &r.id));
        obtainable.extend(
            c.sites
                .iter()
                .flat_map(|s| s.file.iter().filter_map(|f| f.yields.as_ref())),
        );
        loop {
            let more: Vec<&crate::content::ids::ReadableId> = c
                .readables
                .iter()
                .filter(|d| obtainable.contains(&d.id))
                .filter_map(|d| d.yields.as_ref())
                .filter(|y| !obtainable.contains(y))
                .collect();
            if more.is_empty() {
                break;
            }
            obtainable.extend(more);
        }
        for r in c.readables.iter().filter(|r| !obtainable.contains(&r.id)) {
            self.err(
                File::Catalog,
                r.at,
                format!("readable `{}` has no source: not `start`, not unlocked by an effect, and no file or obtainable document yields it", r.id),
            );
        }
    }

    // --------------------------------------------------------------- flags

    fn flags(&mut self) {
        let c = self.c;
        self.unique(
            File::Flags,
            "flag",
            c.flags.iter().map(|x| (x.id.as_str(), x.at)),
        );
        for fl in &c.flags {
            let ctx = format!("flag `{}`", fl.id);
            let at = fl.at;
            let values_ok = fl.values.iter().all(|v| valid_id(v))
                && fl.values.iter().collect::<BTreeSet<_>>().len() == fl.values.len();
            match fl.kind {
                FlagKind::Bool if !fl.values.is_empty() || fl.max.is_some() => {
                    self.err(
                        File::Flags,
                        at,
                        format!("{ctx}: a bool has neither `values` nor `max`"),
                    );
                }
                FlagKind::Enum | FlagKind::Bitset
                    if fl.values.is_empty() || fl.max.is_some() || !values_ok =>
                {
                    self.err(
                        File::Flags,
                        at,
                        format!("{ctx}: needs distinct valid `values` and no `max`"),
                    );
                }
                FlagKind::Counter if fl.max.is_none_or(|m| m == 0) || !fl.values.is_empty() => {
                    self.err(
                        File::Flags,
                        at,
                        format!("{ctx}: a counter needs `max` >= 1 and no `values`"),
                    );
                }
                _ => {}
            }
            if let Some(d) = &fl.default {
                let fits = match (fl.kind, d) {
                    (FlagKind::Bitset, FlagValue::Bits(b)) => {
                        b.iter().all(|x| fl.values.contains(x))
                    }
                    (FlagKind::Bitset, _) => false,
                    _ => flag_accepts(fl, d),
                };
                if !fits {
                    self.err(
                        File::Flags,
                        at,
                        format!("{ctx}: default does not fit the type"),
                    );
                }
            }
        }
    }

    fn rewards(&mut self) {
        let c = self.c;
        for size in Size::ALL {
            let n = c.sizes.iter().filter(|s| s.id == size).count();
            if n != 1 {
                self.err(
                    File::Rewards,
                    0,
                    format!("size {size:?} must be defined exactly once, found {n}"),
                );
            }
        }
        for t in 1..=MAX_TIER {
            let defs: Vec<_> = c.tiers.iter().filter(|x| x.id == t).collect();
            if defs.len() != 1 || defs.iter().any(|x| x.price == Credits::ZERO) {
                self.err(
                    File::Rewards,
                    defs.first().map_or(0, |x| x.at),
                    format!("tier {t} needs exactly one positive `price` (R(P))"),
                );
            }
        }
        if let Some(t) = c.tiers.iter().find(|t| !(1..=MAX_TIER).contains(&t.id)) {
            self.err(File::Rewards, t.at, format!("tier {} out of range", t.id));
        }
        self.game_rules();
    }

    /// The constants of the hub: services, automatic intrusions, hint budgets.
    fn game_rules(&mut self) {
        let c = self.c;
        let f = File::Rewards;
        self.unique(
            f,
            "service",
            c.services.iter().map(|x| (x.id.as_str(), x.at)),
        );
        if c.services.is_empty() {
            self.err(
                f,
                0,
                "at least one `[[service]]` is needed (what `laylow` sells)",
            );
        }
        for service in &c.services {
            if service.price == Credits::ZERO {
                self.err(
                    f,
                    service.at,
                    format!("service `{}` must have a positive price", service.id),
                );
            }
            if !(1..=HEAT_MAX).contains(&service.cooling) {
                self.err(
                    f,
                    service.at,
                    format!(
                        "service `{}`: cooling must be between 1 and {HEAT_MAX}",
                        service.id
                    ),
                );
            }
        }
        let heat = &c.auto_resolve.nominal_heat;
        if heat.len() != usize::from(MAX_TIER) || heat.iter().any(|h| *h > HEAT_MAX) {
            self.err(
                f,
                0,
                format!(
                    "`[auto_resolve] nominal_heat` needs {MAX_TIER} values (one per tier), each at most {HEAT_MAX}"
                ),
            );
        }
        let h = &c.hints;
        if !(h.story >= h.normal && h.normal >= h.expert && h.expert >= h.hardcore) {
            self.err(
                f,
                0,
                "`[hints]` must not grow with the difficulty: story >= normal >= expert >= hardcore",
            );
        }
    }

    // ------------------------------------------------------------- unlocks

    /// The rules of `unlocks.toml`: each names a command of the game once, with a condition of
    /// the mission language and a reason text key.
    fn unlocks(&mut self) {
        let c = self.c;
        let f = File::Unlocks;
        self.unique(
            f,
            "unlocked command",
            c.unlocks.iter().map(|x| (x.command.as_str(), x.at)),
        );
        let known = &c.sources.game_commands;
        for u in &c.unlocks {
            let ctx = format!("unlock of `{}`", u.command);
            if !known.is_empty() && !known.contains(&u.command) {
                self.err(
                    f,
                    u.at,
                    format!("{ctx}: `{}` is not a command of the game", u.command),
                );
            }
            let well_formed = u.reason.starts_with("unlock.")
                && u.reason.len() > "unlock.".len()
                && u.reason.bytes().all(|b| {
                    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.'
                });
            if !well_formed {
                self.err(
                    f,
                    u.at,
                    format!(
                        "{ctx}: the reason `{}` must be a text key `unlock.<name>`",
                        u.reason
                    ),
                );
            }
            self.cond(f, u.at, &ctx, &u.when, 1);
        }
    }

    // -------------------------------------------------------------- quests

    fn quests(&mut self) {
        let c = self.c;
        self.unique(
            File::Quests,
            "quest",
            c.quests.iter().map(|x| (x.id.as_str(), x.at)),
        );
        for (idx, q) in c.quests.iter().enumerate() {
            self.quest(idx, q);
        }
        self.prereq_cycles();
    }

    fn quest(&mut self, idx: usize, q: &QuestDef) {
        let c = self.c;
        let f = File::Quests;
        let ctx = format!("quest `{}`", q.id);
        let at = q.at;
        if q.code.is_empty() {
            self.err(f, at, format!("{ctx}: `code` is empty"));
        }
        if !(1..=6).contains(&q.chapter) {
            self.err(f, at, format!("{ctx}: chapter must be 1 to 6"));
        }
        if !(1..=MAX_TIER).contains(&q.tier) {
            self.err(f, at, format!("{ctx}: tier must be 1 to {MAX_TIER}"));
        }
        self.refer(
            f,
            at,
            &ctx,
            "giver contact",
            c.contact(&q.giver).is_some(),
            &q.giver,
        );
        self.unlock(f, at, &ctx, &q.unlock);
        if let Some(u) = &q.unlock.quest {
            if *u == q.id {
                self.err(f, at, format!("{ctx}: is locked by itself"));
            } else if c.quest_index(u).is_some_and(|i| i > idx) {
                self.err(
                    f,
                    at,
                    format!("{ctx}: its unlock quest `{u}` is written after it"),
                );
            }
        }
        for p in &q.prereq {
            self.refer(f, at, &ctx, "prerequisite", c.quest(p).is_some(), p);
            if *p == q.id {
                self.err(f, at, format!("{ctx}: is its own prerequisite"));
            } else if c.quest_index(p).is_some_and(|i| i > idx) {
                self.err(
                    f,
                    at,
                    format!(
                        "{ctx}: prerequisite `{p}` is written after it (prerequisites come first)"
                    ),
                );
            }
        }
        for cond in [&q.available_if, &q.fail_if].into_iter().flatten() {
            self.cond(f, at, &ctx, cond, 1);
        }
        if (q.fail_if.is_some() || !q.on_fail.is_empty()) && !q.failable {
            self.err(
                f,
                at,
                format!("{ctx}: `fail_if` / `on_fail` need `failable = true`"),
            );
        }
        if q.failable && q.kind == QuestKind::Main {
            self.err(
                f,
                at,
                format!("{ctx}: a main quest cannot fail (no dead end)"),
            );
        }
        if q.kind == QuestKind::Branch && q.available_if.is_none() {
            self.err(
                f,
                at,
                format!("{ctx}: a branch quest needs `available_if` (the decision that opens it)"),
            );
        }
        if q.objective.is_empty() || q.objective.len() > MAX_OBJECTIVES {
            self.err(
                f,
                at,
                format!(
                    "{ctx}: {} objectives, the limit is 1 to {MAX_OBJECTIVES}",
                    q.objective.len()
                ),
            );
        }
        if let Some(g) = &q.reward
            && g.credits.is_none()
            && g.reputation.is_none()
        {
            self.err(f, at, format!("{ctx}: empty reward"));
        }
        let mut ids = BTreeSet::new();
        for (i, o) in q.objective.iter().enumerate() {
            let octx = format!("{ctx}, objective {}", i + 1);
            self.objective(q, o, &octx, true, &mut ids);
        }
        self.blocks(f, at, &ctx, &q.on_open);
        self.blocks(f, at, &ctx, &q.on_complete);
        self.blocks(f, at, &ctx, &q.on_fail);
    }

    fn objective(
        &mut self,
        q: &QuestDef,
        o: &Objective,
        ctx: &str,
        top: bool,
        ids: &mut BTreeSet<String>,
    ) {
        let c = self.c;
        let f = File::Quests;
        let at = o.at;
        if let Some(id) = &o.id
            && (!valid_id(id) || !ids.insert(id.clone()))
        {
            self.err(
                f,
                at,
                format!("{ctx}: objective id `{id}` is invalid or duplicated in the quest"),
            );
        }
        if let Some(w) = &o.when {
            self.cond(f, at, ctx, w, 1);
        }
        match &o.goal {
            Goal::Invalid(msg) => self.err(f, at, format!("{ctx}: {msg}")),
            Goal::Talk { contact, count } => {
                self.refer(f, at, ctx, "contact", c.contact(contact).is_some(), contact);
                self.positive(f, at, ctx, *count);
            }
            Goal::Buy { item } => self.refer(f, at, ctx, "item", c.item(item).is_some(), item),
            Goal::Compromise { site, count } => {
                if let Some(s) = site {
                    self.refer(f, at, ctx, "site", c.site(s).is_some(), s);
                }
                self.positive(f, at, ctx, *count);
            }
            Goal::Extract { site, what } => {
                if let Some(s) = site {
                    self.refer(f, at, ctx, "site", c.site(s).is_some(), s);
                    if let crate::content::schema::ExtractWhat::File(file) = what {
                        let known = c
                            .site(s)
                            .is_some_and(|d| d.file.iter().any(|x| x.id == *file));
                        self.refer(
                            f,
                            at,
                            ctx,
                            &format!("file of `{s}`"),
                            known || c.site(s).is_none(),
                            file,
                        );
                    }
                    if matches!(what, crate::content::schema::ExtractWhat::All)
                        && c.site(s).is_some_and(|d| d.file.is_empty())
                    {
                        self.err(f, at, format!("{ctx}: site `{s}` has no file to extract"));
                    }
                }
                if let crate::content::schema::ExtractWhat::Count(n) = what {
                    self.positive(f, at, ctx, *n);
                }
            }
            Goal::Open { readable } => self.refer(
                f,
                at,
                ctx,
                "readable",
                c.readable(readable).is_some(),
                readable,
            ),
            Goal::Link { contact, level } => {
                self.refer(f, at, ctx, "contact", c.contact(contact).is_some(), contact);
                if !(1..=crate::content::state::MAX_LINK_LEVEL).contains(level) {
                    self.err(f, at, format!("{ctx}: link level must be 1 to 3"));
                }
            }
            Goal::Use { command, group } => {
                if let Some(cmd) = command {
                    self.refer(f, at, ctx, "command", c.has_command(cmd.as_str()), cmd);
                }
                if let Some(g) = group {
                    let known = c
                        .commands
                        .iter()
                        .any(|d| d.group.as_deref() == Some(g.as_str()));
                    self.refer(f, at, ctx, "command group", known, g);
                }
            }
            Goal::SiteState { site, .. } => {
                self.refer(f, at, ctx, "site", c.site(site).is_some(), site);
            }
            Goal::Choice { decision } => {
                let owner = c.decision(decision).map(|d| d.quest.clone());
                self.refer(f, at, ctx, "decision", owner.is_some(), decision);
                if owner.is_some_and(|quest| quest != q.id) {
                    self.err(
                        f,
                        at,
                        format!("{ctx}: decision `{decision}` belongs to another quest"),
                    );
                }
            }
            Goal::HeatEndBelow { max } | Goal::HeatPeakBelow { max }
                if *max == 0 || *max > u32::from(HEAT_MAX) + 1 =>
            {
                self.err(
                    f,
                    at,
                    format!("{ctx}: heat bound must be 1 to {}", u32::from(HEAT_MAX) + 1),
                );
            }
            Goal::Reputation { .. }
            | Goal::Pay { .. }
            | Goal::HeatEndBelow { .. }
            | Goal::HeatPeakBelow { .. } => {}
            Goal::AnyOf(of) => {
                if !top {
                    self.err(f, at, format!("{ctx}: `any_of` cannot be nested"));
                }
                if !(2..=MAX_ALTERNATIVES).contains(&of.len()) {
                    self.err(
                        f,
                        at,
                        format!("{ctx}: `any_of` takes 2 to {MAX_ALTERNATIVES} alternatives"),
                    );
                }
                for (j, ch) in of.iter().enumerate() {
                    if ch.goal.is_condition() {
                        self.err(
                            f,
                            ch.at,
                            format!("{ctx}: a heat objective cannot be an alternative"),
                        );
                    }
                    let cctx = format!("{ctx}, alternative {}", j + 1);
                    self.objective(q, ch, &cctx, false, ids);
                }
            }
        }
        if o.secret && o.goal.is_condition() {
            self.err(
                f,
                at,
                format!("{ctx}: a condition objective cannot be secret"),
            );
        }
    }

    fn positive(&mut self, file: File, at: usize, ctx: &str, n: u32) {
        if n == 0 {
            self.err(file, at, format!("{ctx}: `count` must be at least 1"));
        }
    }

    fn prereq_cycles(&mut self) {
        let c = self.c;
        let mut indeg: BTreeMap<&QuestId, usize> = c.quests.iter().map(|q| (&q.id, 0)).collect();
        // A quest waits for its prerequisites and for its `unlock.quest`.
        let deps = |q: &QuestDef| -> Vec<QuestId> {
            q.prereq
                .iter()
                .chain(q.unlock.quest.iter())
                .filter(|p| **p != q.id && c.quest(p).is_some())
                .cloned()
                .collect()
        };
        for q in &c.quests {
            *indeg.entry(&q.id).or_insert(0) += deps(q).len();
        }
        let mut ready: Vec<&QuestId> = indeg
            .iter()
            .filter(|(_, n)| **n == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut removed = BTreeSet::new();
        while let Some(id) = ready.pop() {
            removed.insert(id);
            for q in c
                .quests
                .iter()
                .filter(|q| q.prereq.contains(id) && q.id != *id)
            {
                if let Some(n) = indeg.get_mut(&q.id) {
                    *n = n.saturating_sub(1);
                    if *n == 0 {
                        ready.push(&q.id);
                    }
                }
            }
        }
        for q in c.quests.iter().filter(|q| !removed.contains(&q.id)) {
            self.err(
                File::Quests,
                q.at,
                format!("quest `{}` is in a prerequisite cycle", q.id),
            );
        }
    }

    // ---------------------------------------------------- conditions, effects

    fn cond(&mut self, file: File, at: usize, ctx: &str, cond: &Cond, depth: usize) {
        let c = self.c;
        if depth > MAX_COND_DEPTH {
            self.err(
                file,
                at,
                format!("{ctx}: condition nested deeper than {MAX_COND_DEPTH}"),
            );
            return;
        }
        match cond {
            Cond::All { of } | Cond::Any { of } => {
                if of.is_empty() {
                    self.err(file, at, format!("{ctx}: empty `all`/`any`"));
                }
                for x in of {
                    self.cond(file, at, ctx, x, depth + 1);
                }
            }
            Cond::Not { of } => self.cond(file, at, ctx, of, depth + 1),
            Cond::Flag {
                name,
                is,
                at_least,
                has,
            } => match c.flag(name) {
                None => self.err(file, at, format!("{ctx}: unknown flag `{name}`")),
                Some(def) => {
                    let ok = match (def.kind, is, at_least, has) {
                        (FlagKind::Bool | FlagKind::Enum, Some(v), None, None)
                        | (FlagKind::Counter, Some(v @ FlagValue::Int(_)), None, None) => {
                            flag_accepts(def, v)
                        }
                        (FlagKind::Counter, None, Some(n), None) => {
                            def.max.is_some_and(|m| *n <= m)
                        }
                        (FlagKind::Bitset, None, None, Some(bit)) => def.values.contains(bit),
                        _ => false,
                    };
                    if !ok {
                        self.err(file, at, format!("{ctx}: flag `{name}` ({:?}) is tested with a wrong or undeclared value", def.kind));
                    }
                }
            },
            Cond::Quest { id, .. } => self.refer(file, at, ctx, "quest", c.quest(id).is_some(), id),
            Cond::Contact { id, is } => {
                self.refer(file, at, ctx, "contact", c.contact(id).is_some(), id);
                if is.is_empty() {
                    self.err(
                        file,
                        at,
                        format!("{ctx}: `contact` condition without states"),
                    );
                }
            }
            Cond::Trust { contact, .. } => self.refer(
                file,
                at,
                ctx,
                "contact",
                c.contact(contact).is_some(),
                contact,
            ),
            Cond::Opened { id } => {
                self.refer(file, at, ctx, "readable", c.readable(id).is_some(), id);
            }
            Cond::Objective { quest, id } => {
                let goal = c.quest(quest).and_then(|q| {
                    let (i, j) = find_objective(q, id)?;
                    let top = q.objective.get(usize::from(i))?;
                    if j == 0 {
                        Some(top)
                    } else {
                        top.children().get(usize::from(j) - 1)
                    }
                });
                match goal {
                    None => self.err(
                        file,
                        at,
                        format!("{ctx}: quest `{quest}` has no objective with id `{id}`"),
                    ),
                    Some(o) if o.goal.is_condition() => {
                        self.err(
                            file,
                            at,
                            format!("{ctx}: objective `{id}` is a condition and is never recorded"),
                        );
                    }
                    Some(_) => {}
                }
            }
        }
    }

    fn blocks(&mut self, file: File, at: usize, ctx: &str, blocks: &[Block]) {
        for b in blocks {
            if let Some(w) = &b.when {
                self.cond(file, at, ctx, w, 1);
            }
            for e in &b.then {
                self.effect(file, at, ctx, e);
            }
        }
    }

    fn effect(&mut self, file: File, at: usize, ctx: &str, e: &Effect) {
        let c = self.c;
        match e {
            Effect::SetFlag { flag, value } => match c.flag(flag) {
                None => self.err(file, at, format!("{ctx}: unknown flag `{flag}`")),
                Some(def) if !flag_accepts(def, value) => {
                    self.err(
                        file,
                        at,
                        format!(
                            "{ctx}: value {value:?} does not fit flag `{flag}` ({:?})",
                            def.kind
                        ),
                    );
                }
                Some(_) => {}
            },
            Effect::SetContactState { contact, .. } | Effect::Trust { contact, .. } => {
                self.refer(
                    file,
                    at,
                    ctx,
                    "contact",
                    c.contact(contact).is_some(),
                    contact,
                );
            }
            Effect::Grant(g) if g.credits.is_none() && g.reputation.is_none() => {
                self.err(file, at, format!("{ctx}: empty `grant`"));
            }
            Effect::GrantTier { tier } if !(1..=MAX_TIER).contains(tier) => {
                self.err(file, at, format!("{ctx}: tier must be 1 to {MAX_TIER}"));
            }
            Effect::Unlock { readable } => self.refer(
                file,
                at,
                ctx,
                "readable",
                c.readable(readable).is_some(),
                readable,
            ),
            Effect::HeatForce { value } if *value > HEAT_MAX => {
                self.err(file, at, format!("{ctx}: heat is 0 to {HEAT_MAX}"));
            }
            Effect::HeatFloor { until, .. } => {
                self.refer(file, at, ctx, "quest", c.quest(until).is_some(), until);
            }
            Effect::SettleEnding => {
                let ok = c.flag_named(ENDING_FLAG).is_some_and(|d| {
                    d.kind == FlagKind::Enum
                        && c.endings
                            .iter()
                            .all(|e| d.values.iter().any(|v| v == e.id.as_str()))
                });
                if !ok || c.endings.is_empty() {
                    self.err(file, at, format!("{ctx}: `settle_ending` needs endings and an enum flag `ending` listing them"));
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------- decisions, topics, endings

    fn decisions(&mut self) {
        let c = self.c;
        let f = File::Decisions;
        self.unique(
            f,
            "decision",
            c.decisions.iter().map(|x| (x.id.as_str(), x.at)),
        );
        for d in &c.decisions {
            let ctx = format!("decision `{}`", d.id);
            let at = d.at;
            self.unique(
                f,
                &format!("choice of {ctx}"),
                d.choice.iter().map(|x| (x.id.as_str(), at)),
            );
            let flag = c.flag(&d.flag);
            match flag {
                Some(def) if def.kind == FlagKind::Enum => {
                    for ch in &d.choice {
                        if !def.values.iter().any(|v| v == ch.id.as_str()) {
                            self.err(
                                f,
                                at,
                                format!(
                                    "{ctx}: choice `{}` is not a declared value of flag `{}`",
                                    ch.id, d.flag
                                ),
                            );
                        }
                    }
                }
                _ => self.err(
                    f,
                    at,
                    format!("{ctx}: flag `{}` must be a declared enum", d.flag),
                ),
            }
            let carrier = c.quest(&d.quest);
            self.refer(f, at, &ctx, "quest", carrier.is_some(), &d.quest);
            if let Some(q) = carrier
                && !q
                    .objective
                    .iter()
                    .any(|o| matches!(&o.goal, Goal::Choice { decision } if *decision == d.id))
            {
                self.err(
                    f,
                    at,
                    format!("{ctx}: quest `{}` has no `choice` objective for it", q.id),
                );
            }
            if d.choice.len() < 2 {
                self.err(
                    f,
                    at,
                    format!("{ctx}: a decision needs at least two choices"),
                );
            }
            for ch in &d.choice {
                let cctx = format!("{ctx}, choice `{}`", ch.id);
                if let Some(r) = &ch.requires {
                    self.cond(f, at, &cctx, r, 1);
                }
                self.blocks(f, at, &cctx, &ch.then);
            }
        }
    }

    fn topics(&mut self) {
        let c = self.c;
        let mut seen = BTreeSet::new();
        for t in &c.topics {
            let ctx = format!("topic `{}` of `{}`", t.id, t.contact);
            self.refer(
                File::Topics,
                t.at,
                &ctx,
                "contact",
                c.contact(&t.contact).is_some(),
                &t.contact,
            );
            if !valid_id(t.id.as_str()) || !seen.insert((&t.contact, &t.id)) {
                self.err(
                    File::Topics,
                    t.at,
                    format!("{ctx}: invalid or duplicate topic id"),
                );
            }
            if let Some(w) = &t.when {
                self.cond(File::Topics, t.at, &ctx, w, 1);
            }
            self.blocks(File::Topics, t.at, &ctx, &t.then);
        }
    }

    fn endings(&mut self) {
        let c = self.c;
        let f = File::Decisions;
        self.unique(f, "ending", c.endings.iter().map(|x| (x.id.as_str(), x.at)));
        self.unique(
            f,
            "epilogue line",
            c.epilogue.iter().map(|x| (x.id.as_str(), x.at)),
        );
        for e in &c.endings {
            self.cond(f, e.at, &format!("ending `{}`", e.id), &e.when, 1);
        }
        for l in &c.epilogue {
            let ctx = format!("epilogue line `{}`", l.id);
            self.refer(
                f,
                l.at,
                &ctx,
                "contact",
                c.contact(&l.contact).is_some(),
                &l.contact,
            );
            self.cond(f, l.at, &ctx, &l.when, 1);
        }
    }

    /// Two writers of the same flag must not depend on which one runs first: they have to be
    /// ordered by prerequisites, be exclusive choices of one decision, or write the same value.
    fn conflicts(&mut self) {
        let c = self.c;
        let mut writes: Writes<'_> = BTreeMap::new();
        let ending = c.flag_named(ENDING_FLAG).map(|d| &d.id);
        for q in &c.quests {
            let w = Writer::Quest(q.id.clone());
            collect_writes(&q.on_open, (&w, q.at, File::Quests), ending, &mut writes);
            collect_writes(
                &q.on_complete,
                (&w, q.at, File::Quests),
                ending,
                &mut writes,
            );
            collect_writes(&q.on_fail, (&w, q.at, File::Quests), ending, &mut writes);
        }
        for d in &c.decisions {
            let w = Writer::Decision(d.id.to_string(), d.quest.clone());
            for ch in &d.choice {
                collect_writes(&ch.then, (&w, d.at, File::Decisions), ending, &mut writes);
            }
            if let Some(def) = c.flag(&d.flag) {
                writes.entry(&def.id).or_default().push((
                    w.clone(),
                    "*decision".into(),
                    d.at,
                    File::Decisions,
                ));
            }
        }
        for t in &c.topics {
            collect_writes(
                &t.then,
                (&Writer::Topic, t.at, File::Topics),
                ending,
                &mut writes,
            );
        }
        for (flag, list) in writes {
            if c.flag(flag).is_some_and(|d| d.kind == FlagKind::Bitset) {
                continue;
            }
            for (i, (wa, va, _, _)) in list.iter().enumerate() {
                for (wb, vb, at, file) in list.iter().skip(i + 1) {
                    if wa == wb
                        || (va == vb && !va.starts_with('*'))
                        || Self::exclusive(wa, wb)
                        || self.ordered(wa, wb)
                    {
                        continue;
                    }
                    self.err(*file, *at, format!("flag `{flag}` is written by two unordered sources ({wa:?} and {wb:?}): the result would depend on event order"));
                }
            }
        }
    }

    fn exclusive(a: &Writer, b: &Writer) -> bool {
        matches!((a, b), (Writer::Decision(x, _), Writer::Decision(y, _)) if x == y)
    }

    fn ordered(&self, a: &Writer, b: &Writer) -> bool {
        let quest_of = |w: &Writer| match w {
            Writer::Quest(q) | Writer::Decision(_, q) => Some(q.clone()),
            Writer::Topic => None,
        };
        let (Some(qa), Some(qb)) = (quest_of(a), quest_of(b)) else {
            return false;
        };
        qa == qb || self.is_ancestor(&qa, &qb) || self.is_ancestor(&qb, &qa)
    }

    /// Whether `anc` is a transitive prerequisite of `q` (iterative: the data may be hostile).
    fn is_ancestor(&self, anc: &QuestId, q: &QuestId) -> bool {
        let mut stack = vec![q.clone()];
        let mut seen = BTreeSet::new();
        while let Some(cur) = stack.pop() {
            if !seen.insert(cur.clone()) {
                continue;
            }
            for p in self.c.quest(&cur).map_or(&[][..], |d| &d.prereq[..]) {
                if p == anc {
                    return true;
                }
                stack.push(p.clone());
            }
        }
        false
    }

    // --------------------------------------------------------------- texts

    fn texts(&mut self) {
        for (key, first, second) in crate::content::texts::collisions(self.c) {
            self.err(
                File::Texts,
                0,
                format!("text key `{key}` is derived from both {first} and {second} (`-` and `_` give the same key)"),
            );
        }
        let derived = required_keys(self.c);
        let text = self.c.sources.get(File::Texts).to_owned();
        for key in derived.difference(&self.c.texts) {
            self.err(
                File::Texts,
                0,
                format!("missing text key `{key}` (derived from the structure)"),
            );
        }
        for key in self.c.texts.difference(&derived) {
            let at = text.find(&format!("\"{key}\"")).unwrap_or(0);
            self.err(
                File::Texts,
                at,
                format!("orphan text key `{key}`: nothing in the structure needs it"),
            );
        }
    }
}

/// Whether a value fits the declared type of a flag.
pub(crate) fn flag_accepts(def: &FlagDef, v: &FlagValue) -> bool {
    match (def.kind, v) {
        (FlagKind::Bool, FlagValue::Bool(_)) => true,
        (FlagKind::Enum | FlagKind::Bitset, FlagValue::Str(s)) => def.values.contains(s),
        (FlagKind::Counter, FlagValue::Int(n)) => def.max.is_some_and(|m| *n <= m),
        _ => false,
    }
}

type Writes<'a> = BTreeMap<&'a FlagId, Vec<(Writer, String, usize, File)>>;

fn unlocks_in<'a>(blocks: &'a [Block], out: &mut BTreeSet<&'a crate::content::ids::ReadableId>) {
    for e in blocks.iter().flat_map(|b| &b.then) {
        if let Effect::Unlock { readable } = e {
            out.insert(readable);
        }
    }
}

/// Records the flag writes of a list of blocks. `where_` is the writer and its position.
fn collect_writes<'a>(
    blocks: &'a [Block],
    where_: (&Writer, usize, File),
    ending: Option<&'a FlagId>,
    writes: &mut Writes<'a>,
) {
    let (who, at, file) = where_;
    for e in blocks.iter().flat_map(|b| &b.then) {
        match e {
            Effect::SetFlag { flag, value } => {
                writes
                    .entry(flag)
                    .or_default()
                    .push((who.clone(), format!("{value:?}"), at, file));
            }
            Effect::SettleEnding => {
                if let Some(flag) = ending {
                    writes
                        .entry(flag)
                        .or_default()
                        .push((who.clone(), "*".into(), at, file));
                }
            }
            _ => {}
        }
    }
}
