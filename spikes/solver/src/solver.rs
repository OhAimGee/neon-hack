//! Exact search: best-first (Dijkstra / A*) over packed states.
//!
//! The objective is the final Trace, then the number of turns. The Trace is part of the
//! state, so the cost of a state does not depend on the path: the search orders states by
//! `f = trace - 10 * spoof_charges + h` (see [`crate::heuristic`]). A terminal state
//! (`jackout`) is pushed with its exact Trace, so the first terminal popped is optimal.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::time::{Duration, Instant};

use crate::heuristic::{Heuristic, INFEASIBLE};
use crate::model::{Action, Mission, State, Status, legal_actions, step};
use crate::table::{BTreeTables, FxTables, IndexMap, LocalSet, TableKind, Tables};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal {
    /// Optimal plan: Trace first, number of turns second.
    Optimal,
    /// Any plan under the mission's Trace cap, early exit at the first one found.
    Feasible,
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub tables: TableKind,
    /// Expand a whole turn at once and store only end-of-turn states.
    pub macro_turns: bool,
    /// Use the admissible lower bound `h` (A*); without it, uniform-cost search.
    pub heuristic: bool,
    /// Skip a state dominated by an already expanded one.
    pub dominance: bool,
    /// Break ties between equal `f` by the lowest remaining bound instead of the fewest turns.
    pub close_first: bool,
    /// Stop with `Unknown` after this many expansions (0 = no limit).
    pub max_expanded: u64,
    pub goal: Goal,
}

impl Config {
    /// Plain uniform-cost search, every action is a transition: the simplest exact solver.
    pub const fn baseline() -> Self {
        Config {
            tables: TableKind::Fx,
            macro_turns: false,
            heuristic: false,
            dominance: false,
            close_first: false,
            max_expanded: 0,
            goal: Goal::Optimal,
        }
    }

    /// The best combination found by the measurements: individual states (no end-of-turn
    /// merging, which never wins in time), A*, dominance, "closest to the goal" tie-break,
    /// open-addressing table.
    pub const fn tuned() -> Self {
        Config {
            tables: TableKind::Fx,
            macro_turns: false,
            heuristic: true,
            dominance: true,
            close_first: true,
            max_expanded: 0,
            goal: Goal::Optimal,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Final Trace.
    pub cost: u8,
    pub turns: u16,
    /// Ends with `Jackout`.
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A plan. `optimal`: its final Trace is the lowest possible. `turns_minimal`: among the
    /// plans with that Trace it also has the fewest turns; only the plain search without
    /// heuristic nor dominance guarantees it (an inconsistent bound or a skipped dominated
    /// state can leave a later, slower path to the same Trace).
    Solved {
        plan: Plan,
        optimal: bool,
        turns_minimal: bool,
    },
    /// Proven: no plan keeps the Trace under the cap.
    Unsolvable,
    /// The expansion limit was hit. `lower_bound` is proven: no plan ends under it.
    Unknown {
        best_known: Option<u8>,
        lower_bound: u8,
    },
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    /// States taken from the queue and expanded.
    pub expanded: u64,
    /// Successor states offered to the visited set.
    pub generated: u64,
    /// Nodes kept in the arena.
    pub stored: u64,
    /// Mid-turn states visited (macro mode only).
    pub intra_states: u64,
    /// States skipped because an expanded state dominated them.
    pub dominated: u64,
    /// Bytes of the arena and of the visited set (length based).
    pub bytes_tight: usize,
    /// Same, counting spare capacity (what the process really holds).
    pub bytes_capacity: usize,
    pub elapsed: Duration,
}

#[derive(Clone, Debug)]
pub struct Report {
    pub outcome: Outcome,
    pub stats: Stats,
}

impl Report {
    pub fn plan(&self) -> Option<&Plan> {
        match &self.outcome {
            Outcome::Solved { plan, .. } => Some(plan),
            _ => None,
        }
    }

    pub fn cost(&self) -> Option<u8> {
        self.plan().map(|p| p.cost)
    }
}

/// Solves from the mission's initial state.
pub fn solve(m: &Mission, cfg: &Config) -> Report {
    solve_from(m, &m.initial(), cfg)
}

/// Solves from any state (used for the per-turn preview).
pub fn solve_from(m: &Mission, start: &State, cfg: &Config) -> Report {
    match cfg.tables {
        TableKind::BTree => run::<BTreeTables>(m, start, cfg),
        TableKind::Fx => run::<FxTables>(m, start, cfg),
    }
}

/// "Is the mission solvable with a final Trace strictly under `budget`?" with early exit.
pub fn solvable_within(m: &Mission, budget: u8, cfg: &Config) -> Report {
    solvable_within_from(m, &m.initial(), budget, cfg)
}

/// [`solvable_within`] from any state.
pub fn solvable_within_from(m: &Mission, start: &State, budget: u8, cfg: &Config) -> Report {
    let mut capped = m.clone();
    capped.trace_cap = capped.trace_cap.min(budget);
    let cfg = Config {
        goal: Goal::Feasible,
        ..*cfg
    };
    solve_from(&capped, start, &cfg)
}

const NO_PARENT: u32 = u32::MAX;

#[derive(Default)]
struct Arena {
    keys: Vec<u128>,
    parent: Vec<u32>,
    turns: Vec<u16>,
    act: Vec<u8>,
    closed: Vec<bool>,
}

impl Arena {
    fn push(&mut self, key: u128, parent: u32, turns: u16, act: u8) {
        self.keys.push(key);
        self.parent.push(parent);
        self.turns.push(turns);
        self.act.push(act);
        self.closed.push(false);
    }

    fn bytes_tight(&self) -> usize {
        self.keys.len() * 16 + self.parent.len() * 4 + self.turns.len() * 2 + self.act.len() * 2
    }

    fn bytes_capacity(&self) -> usize {
        self.keys.capacity() * 16
            + self.parent.capacity() * 4
            + self.turns.capacity() * 2
            + self.act.capacity()
            + self.closed.capacity()
    }
}

/// Queue entry: 12 bits of priority, 12 bits of tie-break, 8 bits of turns, 32 of index.
fn entry(prio: (i32, i32), turns: u16, idx: u32) -> Reverse<u64> {
    let p = u64::try_from(prio.0.clamp(0, 0xFFF)).unwrap_or(0);
    let sub = u64::try_from(prio.1.clamp(0, 0xFFF)).unwrap_or(0);
    Reverse((p << 52) | (sub << 40) | (u64::from(turns.min(0xFF)) << 32) | u64::from(idx))
}

/// Fields compared with `>=` are packed in lanes with a guard bit, so that one subtraction
/// compares all of them: progress (8 lanes of 4 bits), neutral (4 x 3) and charges (11 x 3).
#[derive(Clone, Copy)]
struct DomEntry {
    lanes: u128,
    trace: u8,
    turns: u16,
}

const fn guard_bits() -> u128 {
    let mut g = 0u128;
    let mut i = 0;
    while i < 8 {
        g |= 8u128 << (4 * i);
        i += 1;
    }
    i = 0;
    while i < 4 {
        g |= 4u128 << (32 + 3 * i);
        i += 1;
    }
    i = 0;
    while i < 11 {
        g |= 4u128 << (44 + 3 * i);
        i += 1;
    }
    g
}
const GUARD: u128 = guard_bits();

impl DomEntry {
    fn of(s: &State, turns: u16) -> Self {
        let mut lanes = 0u128;
        for (i, &p) in s.progress.iter().enumerate() {
            lanes |= u128::from(p) << (4 * i);
        }
        for (i, &n) in s.neutral.iter().enumerate() {
            lanes |= u128::from(n) << (32 + 3 * i);
        }
        for (i, &c) in s.charges.iter().enumerate() {
            lanes |= u128::from(c.min(3)) << (44 + 3 * i);
        }
        DomEntry {
            lanes,
            trace: s.trace,
            turns,
        }
    }

    /// `self` is at least as good as `o` on every axis.
    #[inline]
    fn dominates(&self, o: &DomEntry) -> bool {
        self.trace <= o.trace
            && self.turns <= o.turns
            && ((self.lanes | GUARD) - o.lanes) & GUARD == GUARD
    }
}

/// Antichain of expanded states, bucketed by the fields that must be equal.
#[derive(Default)]
struct Dominance {
    buckets: BTreeMap<u64, Vec<DomEntry>>,
    entries: usize,
}

impl Dominance {
    fn bucket(s: &State) -> u64 {
        u64::from(s.pos)
            | u64::from(s.cycles) << 3
            | u64::from(s.phase) << 7
            | u64::from(s.cloak) << 11
            | u64::from(s.ghost) << 12
            | u64::from(s.loot_taken) << 13
    }

    /// True if `s` is dominated; otherwise records it (dropping the entries it dominates).
    fn check_insert(&mut self, s: &State, turns: u16) -> bool {
        let e = DomEntry::of(s, turns);
        let list = self.buckets.entry(Self::bucket(s)).or_default();
        // One pass over an antichain: if an entry dominates `e`, none of them is dominated
        // by `e` (it would then be dominated by that entry too).
        let mut i = 0;
        while i < list.len() {
            if list[i].dominates(&e) {
                return true;
            }
            if e.dominates(&list[i]) {
                list.swap_remove(i);
                self.entries -= 1;
            } else {
                i += 1;
            }
        }
        list.push(e);
        self.entries += 1;
        false
    }

    fn bytes(&self) -> usize {
        self.entries * size_of::<DomEntry>()
    }
}

enum Offer {
    Skip,
    Pushed,
    Goal(u32),
}

struct Search<'a, T: Tables> {
    m: &'a Mission,
    cfg: &'a Config,
    heur: Heuristic,
    index: T::Index,
    local: T::Local,
    arena: Arena,
    heap: BinaryHeap<Reverse<u64>>,
    dom: Option<Dominance>,
    stats: Stats,
    /// Best terminal Trace generated so far.
    incumbent: i32,
}

impl<T: Tables> Search<'_, T> {
    /// `(priority, tie-break)` of a running state, `None` if it can be discarded.
    fn priority(&self, s: &State) -> Option<(i32, i32)> {
        let (base, h, dist) = self.heur.parts3(self.m, s, self.cfg.heuristic);
        if h >= INFEASIBLE {
            return None;
        }
        let f = (base + h).max(0);
        if f >= i32::from(self.m.trace_cap) || f > self.incumbent {
            return None;
        }
        Some(match self.cfg.goal {
            // Among equal `f`, either the fewest turns first (exact tie-break) or the state
            // closest to the goal first (much faster on plateaus, turns no longer minimal).
            Goal::Optimal => (
                f,
                if self.cfg.close_first {
                    dist.clamp(0, 0xFFF)
                } else {
                    0
                },
            ),
            // Greedy: closest to the goal first, then lowest Trace.
            Goal::Feasible => (dist.clamp(0, 0xFFF), i32::from(s.trace)),
        })
    }

    fn offer(&mut self, child: &State, parent: u32, turns: u16, act: u8) -> Offer {
        let key = child.canonical().pack();
        self.stats.generated += 1;
        if child.status == Status::Burned {
            return Offer::Skip;
        }
        // Already known: only the number of turns can still improve (decrease-key); the
        // priority of a state does not depend on the path, so it is read from the queue
        // entry that exists already.
        if let Some(i) = self.index.find(key, &self.arena.keys) {
            let iu = i as usize;
            if turns < self.arena.turns[iu] && !self.arena.closed[iu] {
                let prio = match child.status {
                    Status::Won => (i32::from(child.trace), 0),
                    _ => match self.priority(child) {
                        Some(p) => p,
                        None => return Offer::Skip,
                    },
                };
                self.arena.turns[iu] = turns;
                self.arena.parent[iu] = parent;
                self.arena.act[iu] = act;
                self.heap.push(entry(prio, turns, i));
            }
            return Offer::Skip;
        }
        let prio = match child.status {
            Status::Won => {
                if i32::from(child.trace) > self.incumbent {
                    return Offer::Skip;
                }
                (i32::from(child.trace), 0)
            }
            _ => match self.priority(child) {
                Some(p) => p,
                None => return Offer::Skip,
            },
        };
        let new_idx = self.arena.keys.len() as u32;
        self.index.find_or_insert(key, new_idx, &self.arena.keys);
        self.arena.push(key, parent, turns, act);
        self.heap.push(entry(prio, turns, new_idx));
        if child.status == Status::Won {
            self.incumbent = self.incumbent.min(i32::from(child.trace));
            if self.cfg.goal == Goal::Feasible {
                return Offer::Goal(new_idx);
            }
        }
        Offer::Pushed
    }

    /// Fine-grained expansion: one transition per action.
    fn expand_fine(
        &mut self,
        idx: u32,
        state: &State,
        turns: u16,
        actions: &mut Vec<Action>,
    ) -> Option<u32> {
        legal_actions(self.m, state, actions);
        for &a in actions.iter() {
            let Ok(child) = step(self.m, state, a) else {
                continue;
            };
            let t = turns + u16::from(a == Action::End);
            if let Offer::Goal(g) = self.offer(&child, idx, t, a.encode()) {
                return Some(g);
            }
        }
        None
    }

    /// Whole-turn expansion: depth-first inside the turn, merged by a local set; only the
    /// end-of-turn states and the terminal states reach the arena.
    fn expand_turn(
        &mut self,
        idx: u32,
        state: &State,
        turns: u16,
        actions: &mut Vec<Action>,
    ) -> Option<u32> {
        let mut stack: Vec<State> = vec![*state];
        self.local.clear();
        self.local.insert(state.pack());
        while let Some(cur) = stack.pop() {
            legal_actions(self.m, &cur, actions);
            for &a in actions.iter() {
                let Ok(child) = step(self.m, &cur, a) else {
                    continue;
                };
                match a {
                    Action::End => {
                        if let Offer::Goal(g) = self.offer(&child, idx, turns + 1, 0) {
                            return Some(g);
                        }
                    }
                    Action::Jackout => {
                        if let Offer::Goal(g) = self.offer(&child, idx, turns, 0) {
                            return Some(g);
                        }
                    }
                    _ => {
                        if child.status != Status::Running {
                            continue;
                        }
                        let (base, _) = self.heur.parts(self.m, &child, false);
                        if base.max(0) > self.incumbent {
                            continue;
                        }
                        if self.local.insert(child.pack()) {
                            self.stats.intra_states += 1;
                            stack.push(child);
                        }
                    }
                }
            }
        }
        None
    }

    fn finish_stats(&mut self, start: Instant) {
        self.stats.stored = self.arena.keys.len() as u64;
        let dom = self.dom.as_ref().map_or(0, Dominance::bytes);
        let other = self.heap.capacity() * 8 + self.local.heap_bytes() + dom;
        self.stats.bytes_tight = self.arena.bytes_tight() + self.index.heap_bytes();
        self.stats.bytes_capacity = self.arena.bytes_capacity() + self.index.heap_bytes() + other;
        self.stats.elapsed = start.elapsed();
    }

    fn plan(&self, won: u32) -> Plan {
        let mut chain = vec![won];
        let mut cur = won;
        while self.arena.parent[cur as usize] != NO_PARENT {
            cur = self.arena.parent[cur as usize];
            chain.push(cur);
        }
        chain.reverse();
        let mut actions = Vec::new();
        for pair in chain.windows(2) {
            let (p, c) = (pair[0] as usize, pair[1] as usize);
            if self.cfg.macro_turns {
                let from = State::unpack(self.m, self.arena.keys[p]);
                let mut path = Vec::new();
                let mut seen = BTreeSet::new();
                let found = find_turn_path(self.m, &from, self.arena.keys[c], &mut seen, &mut path);
                assert!(found, "turn path must exist");
                actions.extend(path);
            } else {
                actions.push(Action::decode(self.arena.act[c]));
            }
        }
        let last = State::unpack(self.m, self.arena.keys[won as usize]);
        Plan {
            cost: last.trace,
            turns: self.arena.turns[won as usize],
            actions,
        }
    }
}

/// Depth-first search of the action sequence that leads, inside one turn, from `s` to the
/// state whose packed key is `target` (an end-of-turn or terminal state).
pub(crate) fn find_turn_path(
    m: &Mission,
    s: &State,
    target: u128,
    seen: &mut BTreeSet<u128>,
    path: &mut Vec<Action>,
) -> bool {
    let mut actions = Vec::new();
    legal_actions(m, s, &mut actions);
    for a in actions {
        let Ok(child) = step(m, s, a) else { continue };
        path.push(a);
        if child.canonical().pack() == target {
            return true;
        }
        if a != Action::End
            && a != Action::Jackout
            && child.status == Status::Running
            && seen.insert(child.pack())
            && find_turn_path(m, &child, target, seen, path)
        {
            return true;
        }
        path.pop();
    }
    false
}

fn run<T: Tables>(m: &Mission, start: &State, cfg: &Config) -> Report {
    let t0 = Instant::now();
    let mut s = Search::<T> {
        m,
        cfg,
        heur: Heuristic::new(m),
        index: T::Index::new(),
        local: T::Local::new(),
        arena: Arena::default(),
        heap: BinaryHeap::new(),
        dom: cfg.dominance.then(Dominance::default),
        stats: Stats::default(),
        incumbent: i32::MAX,
    };
    let mut root = *start;
    root.cycles = start.cycles;
    let key = root.pack();
    s.index.find_or_insert(key, 0, &s.arena.keys);
    s.arena.push(key, NO_PARENT, 0, 0);
    let prio = s.priority(&root);
    let mut actions: Vec<Action> = Vec::new();
    let mut outcome = Outcome::Unsolvable;
    if let Some(p) = prio {
        s.heap.push(entry(p, 0, 0));
        'search: while let Some(Reverse(e)) = s.heap.pop() {
            let idx = (e & 0xFFFF_FFFF) as u32;
            let turns = ((e >> 32) & 0xFF) as u16;
            let i = idx as usize;
            if s.arena.closed[i] || s.arena.turns[i].min(0xFF) != turns {
                continue;
            }
            let state = State::unpack(m, s.arena.keys[i]);
            if state.status == Status::Won {
                outcome = Outcome::Solved {
                    plan: s.plan(idx),
                    optimal: cfg.goal == Goal::Optimal,
                    turns_minimal: cfg.goal == Goal::Optimal && !cfg.heuristic && !cfg.dominance,
                };
                break;
            }
            s.arena.closed[i] = true;
            if let Some(d) = s.dom.as_mut()
                && d.check_insert(&state, turns)
            {
                s.stats.dominated += 1;
                continue;
            }
            // The limit is checked before the count: the state popped here is not expanded
            // when the limit is reached, so `expanded` never exceeds `max_expanded`.
            if cfg.max_expanded != 0 && s.stats.expanded >= cfg.max_expanded {
                let best =
                    (s.incumbent != i32::MAX).then(|| u8::try_from(s.incumbent).unwrap_or(u8::MAX));
                // The queue holds every unexpanded state: the priority of the entry just
                // popped is the smallest of them, so it is a proven lower bound of the
                // optimum (meaningful for `Goal::Optimal` only).
                let lb = if cfg.goal == Goal::Optimal {
                    ((e >> 52) & 0xFFF).min(255) as u8
                } else {
                    0
                };
                outcome = Outcome::Unknown {
                    best_known: best,
                    lower_bound: lb,
                };
                break 'search;
            }
            s.stats.expanded += 1;
            let goal = if cfg.macro_turns {
                s.expand_turn(idx, &state, turns, &mut actions)
            } else {
                s.expand_fine(idx, &state, turns, &mut actions)
            };
            if let Some(g) = goal {
                outcome = Outcome::Solved {
                    plan: s.plan(g),
                    optimal: false,
                    turns_minimal: false,
                };
                break;
            }
        }
    }
    s.finish_stats(t0);
    Report {
        outcome,
        stats: s.stats,
    }
}
