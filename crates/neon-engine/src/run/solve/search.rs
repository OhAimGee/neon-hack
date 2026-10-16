//! Best-first search (A*) over canonical states, with dominance.
//!
//! The objective is the final Trace, then the number of turns. The Trace is part of the state,
//! so the cost of a state does not depend on the path: states are ordered by a lower bound of
//! the final Trace ([`Bound`]) and a won state is queued with its exact Trace, so the first
//! won state taken out of the queue is optimal. Everything is deterministic: ordered maps, no
//! seed, no clock.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::super::action::{Action, Utility};
use super::super::mission::{MAX_NODES, MAX_PATROLS, Mission};
use super::super::program::{PROGRAM_COUNT, Program};
use super::super::rules::{apply, legal_actions};
use super::super::state::{RunState, Status};
use super::bound::Bound;
use super::key::{canonical, pack};
use super::{Goal, Outcome, Plan, Report, Stats};

const NO_PARENT: u32 = u32::MAX;

/// What a search is asked.
#[derive(Debug, Clone, Copy)]
pub(super) struct Settings {
    pub(super) goal: Goal,
    /// Stop after this many expansions (0 for no limit).
    pub(super) max_expanded: u64,
    /// Only plans that end under this Trace count (at most the cap of the mission).
    pub(super) cap: u8,
    /// Skip a state that an expanded state dominates.
    pub(super) dominance: bool,
}

struct Node {
    state: RunState,
    parent: u32,
    action: Option<Action>,
    turns: u32,
    closed: bool,
}

/// What can be said of a state that is at least as good as another on every axis.
///
/// The lanes where more is better (the progress on each node, the turns each patrol stays down,
/// the charges of each program) are kept as bytes of a few words, each below 128, so that one
/// subtraction compares eight of them at once.
#[derive(Clone, Copy)]
struct Entry {
    trace: u8,
    floor: u8,
    turns: u32,
    /// The progress on the eight nodes.
    progress: u64,
    /// The turns down of the patrols, then the charges of the programs, then padding.
    rest: [u64; 2],
}

const _: () = assert!(MAX_NODES == 8 && MAX_PATROLS + PROGRAM_COUNT <= 16);

/// The top bit of every byte of a word.
const HIGH: u64 = 0x8080_8080_8080_8080;

/// Whether every byte of `a` is at least the byte of `b` (all bytes being below 128: setting the
/// top bit of each byte of `a` keeps the subtraction from borrowing across bytes, and the bit
/// survives where `a` is not below `b`).
fn all_at_least(a: u64, b: u64) -> bool {
    (a | HIGH).wrapping_sub(b) & HIGH == HIGH
}

impl Entry {
    fn of(state: &RunState, turns: u32) -> Self {
        let mut bytes = [0u8; 16];
        for (slot, value) in bytes
            .iter_mut()
            .zip(state.down.iter().chain(state.charges.iter()))
        {
            *slot = *value;
        }
        let word = |at: usize| {
            let mut word = [0u8; 8];
            for (slot, value) in word.iter_mut().zip(bytes.iter().skip(at)) {
                *slot = *value;
            }
            u64::from_le_bytes(word)
        };
        Self {
            trace: state.trace,
            floor: state.turn_floor,
            turns,
            progress: u64::from_le_bytes(state.progress),
            rest: [word(0), word(8)],
        }
    }

    /// `self` is no worse than `other` on every axis: lower Trace, lower floor, fewer turns,
    /// more progress, patrols down at least as long, at least as many charges.
    ///
    /// With `exact_progress`, the progress must be the same: more progress is no better when a
    /// breach has an effect of its own (a Virus puts down the patrols standing on the node), so a
    /// node that is only part way open can still be worth more than one that is open.
    fn dominates(&self, other: &Self, exact_progress: bool) -> bool {
        self.trace <= other.trace
            && self.floor <= other.floor
            && self.turns <= other.turns
            && if exact_progress {
                self.progress == other.progress
            } else {
                all_at_least(self.progress, other.progress)
            }
            && self
                .rest
                .iter()
                .zip(&other.rest)
                .all(|(a, b)| all_at_least(*a, *b))
    }
}

/// The fields that must be equal for one state to replace another: position, cycles, turn,
/// Cloak, Ghost and loot taken.
type Bucket = (usize, u8, u32, bool, bool, u8);

/// A table from 128-bit keys to small numbers, open addressing with linear probing. It is never
/// walked in any order, only asked, so nothing depends on how it is laid out: the search is as
/// deterministic as with an ordered map, and a good deal quicker.
struct Table {
    slots: Vec<(u128, u32)>,
    len: usize,
}

/// A slot that holds nothing.
const VACANT: u32 = u32::MAX;

impl Table {
    /// A table that holds `expected` keys without growing (a power of two, twice as many slots).
    fn with_capacity(expected: usize) -> Self {
        Self {
            slots: vec![(0, VACANT); (expected * 2).next_power_of_two().max(1 << 10)],
            len: 0,
        }
    }

    fn home(&self, key: u128) -> usize {
        let low = u64::try_from(key & u128::from(u64::MAX)).unwrap_or(0);
        let high = u64::try_from(key >> 64).unwrap_or(0);
        let mixed = (low ^ high.rotate_left(23)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        usize::try_from(mixed ^ (mixed >> 29)).unwrap_or(0) & (self.slots.len() - 1)
    }

    fn get(&self, key: u128) -> Option<u32> {
        let mask = self.slots.len() - 1;
        let mut at = self.home(key);
        loop {
            let &(known, value) = self.slots.get(at)?;
            if value == VACANT {
                return None;
            }
            if known == key {
                return Some(value);
            }
            at = (at + 1) & mask;
        }
    }

    /// Records `key` (which must not be there yet).
    fn insert(&mut self, key: u128, value: u32) {
        if (self.len + 1) * 2 > self.slots.len() {
            self.grow();
        }
        self.place(key, value);
        self.len += 1;
    }

    fn place(&mut self, key: u128, value: u32) {
        let mask = self.slots.len() - 1;
        let mut at = self.home(key);
        while self.slots.get(at).is_some_and(|slot| slot.1 != VACANT) {
            at = (at + 1) & mask;
        }
        if let Some(slot) = self.slots.get_mut(at) {
            *slot = (key, value);
        }
    }

    fn grow(&mut self) {
        let old = std::mem::replace(&mut self.slots, vec![(0, VACANT); 0]);
        self.slots = vec![(0, VACANT); old.len() * 2];
        for (key, value) in old {
            if value != VACANT {
                self.place(key, value);
            }
        }
    }
}

/// The expanded states, by bucket.
struct Dominance {
    /// Bucket to its place in `entries`.
    buckets: Table,
    entries: Vec<Vec<Entry>>,
    /// Whether progress must be equal for one state to dominate another (the deck has a Virus).
    exact_progress: bool,
}

impl Dominance {
    fn new(mission: &Mission) -> Self {
        Self {
            buckets: Table::with_capacity(0),
            entries: Vec::new(),
            exact_progress: mission.supply(Program::Virus).is_some(),
        }
    }

    /// True if `state` is dominated; otherwise records it, dropping what it dominates.
    fn check_insert(&mut self, state: &RunState, turns: u32) -> bool {
        let entry = Entry::of(state, turns);
        let bucket: Bucket = (
            state.position.index(),
            state.cycles,
            state.turn,
            state.cloak,
            state.ghost,
            state.loot_taken,
        );
        let key = (bucket.0 as u128)
            | (u128::from(bucket.1) << 8)
            | (u128::from(bucket.2) << 16)
            | (u128::from(bucket.3) << 48)
            | (u128::from(bucket.4) << 49)
            | (u128::from(bucket.5) << 56);
        let place = if let Some(place) = self.buckets.get(key) {
            place as usize
        } else {
            let place = self.entries.len();
            self.buckets
                .insert(key, u32::try_from(place).unwrap_or(VACANT - 1));
            self.entries.push(Vec::new());
            place
        };
        let Some(bucket) = self.entries.get_mut(place) else {
            return false;
        };
        // The entries of a bucket are kept by increasing Trace, and none dominates another. An
        // entry that dominates `entry` has a Trace no higher than its own, an entry that `entry`
        // dominates one no lower: each side only looks at its half.
        let not_above = bucket.partition_point(|other| other.trace <= entry.trace);
        if bucket
            .iter()
            .take(not_above)
            .any(|other| other.dominates(&entry, self.exact_progress))
        {
            return true;
        }
        let mut index = bucket.partition_point(|other| other.trace < entry.trace);
        while let Some(other) = bucket.get(index) {
            if entry.dominates(other, self.exact_progress) {
                bucket.remove(index);
            } else {
                index += 1;
            }
        }
        let at = bucket.partition_point(|other| other.trace <= entry.trace);
        bucket.insert(at, entry);
        false
    }
}

enum Offer {
    Skip,
    Pushed,
    Goal,
}

struct Search<'a> {
    mission: &'a Mission,
    settings: Settings,
    bound: Bound,
    index: Table,
    nodes: Vec<Node>,
    heap: BinaryHeap<Reverse<(i32, i32, u32, u32)>>,
    dominance: Option<Dominance>,
    stats: Stats,
    /// The lowest Trace of a won state met so far, and its node.
    incumbent: i32,
    incumbent_node: Option<u32>,
}

impl Search<'_> {
    /// `(priority, tie-break)` of a running state; `None` when it can be dropped.
    fn priority(&self, state: &RunState) -> Option<(i32, i32)> {
        let estimate = self.bound.estimate(self.mission, state)?;
        let f = estimate.floor;
        if f >= i32::from(self.settings.cap) || f > self.incumbent {
            return None;
        }
        let distance = estimate.distance.clamp(0, 0xFFF);
        Some(match self.settings.goal {
            // Among equal bounds, the state closest to the goal first.
            Goal::Optimal => (f, distance),
            // Greedy: closest to the goal first, then the lowest Trace.
            Goal::Feasible => (distance, i32::from(state.trace)),
        })
    }

    fn offer(&mut self, child: &RunState, parent: u32, turns: u32, action: Action) -> Offer {
        self.stats.generated += 1;
        if child.status == Status::Burned || i32::from(child.trace) >= i32::from(self.settings.cap)
        {
            return Offer::Skip;
        }
        let canon = canonical(self.mission, child);
        let key = pack(&canon);
        let priority = |search: &Self| match canon.status {
            Status::Won => {
                (i32::from(canon.trace) <= search.incumbent).then_some((i32::from(canon.trace), 0))
            }
            _ => search.priority(&canon),
        };
        if let Some(known) = self.index.get(key) {
            // Known: only the number of turns can still improve.
            let better = self
                .nodes
                .get(known as usize)
                .is_some_and(|node| turns < node.turns && !node.closed);
            if better && let Some(prio) = priority(self) {
                if let Some(node) = self.nodes.get_mut(known as usize) {
                    node.turns = turns;
                    node.parent = parent;
                    node.action = Some(action);
                }
                self.heap.push(Reverse((prio.0, prio.1, turns, known)));
            }
            return Offer::Skip;
        }
        let Some(prio) = priority(self) else {
            return Offer::Skip;
        };
        let id = u32::try_from(self.nodes.len()).unwrap_or(u32::MAX);
        self.index.insert(key, id);
        self.nodes.push(Node {
            state: canon,
            parent,
            action: Some(action),
            turns,
            closed: false,
        });
        self.heap.push(Reverse((prio.0, prio.1, turns, id)));
        if canon.status == Status::Won {
            if i32::from(canon.trace) < self.incumbent {
                self.incumbent = i32::from(canon.trace);
                self.incumbent_node = Some(id);
            }
            if self.settings.goal == Goal::Feasible {
                return Offer::Goal;
            }
        }
        Offer::Pushed
    }

    /// The actions worth trying: those of the rules, minus the ones that only reorder a plan.
    /// A Cloak only matters at the end of the turn, so once it is on only the end is offered;
    /// a Ghost silences what follows, so it is only offered first in a turn.
    fn actions(&self, state: &RunState) -> Vec<Action> {
        let mut actions = legal_actions(self.mission, state);
        if state.cloak {
            actions.retain(|action| matches!(action, Action::End | Action::Jackout));
        }
        if state.cycles != self.mission.cycles_per_turn {
            actions.retain(|action| *action != Action::Use(Utility::Ghost));
        }
        actions
    }

    fn plan(&self, won: u32) -> Plan {
        let mut actions = Vec::new();
        let mut current = won;
        while let Some(node) = self.nodes.get(current as usize) {
            if node.parent == NO_PARENT {
                break;
            }
            actions.extend(node.action);
            current = node.parent;
        }
        actions.reverse();
        let node = self.nodes.get(won as usize);
        Plan {
            actions,
            trace: node.map_or(0, |n| n.state.trace),
            turns: node.map_or(0, |n| n.turns),
        }
    }
}

/// Searches from `start`.
pub(super) fn search(mission: &Mission, start: &RunState, settings: Settings) -> Report {
    let cap = settings.cap.min(mission.trace_cap);
    let settings = Settings { cap, ..settings };
    // A search that is allowed a few thousand expansions meets some tens of thousands of states.
    let expected = usize::try_from(settings.max_expanded)
        .map_or(1 << 16, |expansions| expansions.saturating_mul(6))
        .clamp(1 << 10, 1 << 16);
    let mut search = Search {
        mission,
        settings,
        bound: Bound::new(mission),
        index: Table::with_capacity(expected),
        nodes: Vec::with_capacity(expected),
        heap: BinaryHeap::with_capacity(expected),
        dominance: settings.dominance.then(|| Dominance::new(mission)),
        stats: Stats::default(),
        incumbent: i32::MAX,
        incumbent_node: None,
    };
    let root = canonical(mission, start);
    search.index.insert(pack(&root), 0);
    search.nodes.push(Node {
        state: root,
        parent: NO_PARENT,
        action: None,
        turns: 0,
        closed: false,
    });
    let mut outcome = Outcome::Unsolvable;
    if let Some(prio) = search.priority(&root) {
        search.heap.push(Reverse((prio.0, prio.1, 0, 0)));
        outcome = run(&mut search);
    }
    Report {
        outcome,
        stats: search.stats,
    }
}

fn run(search: &mut Search<'_>) -> Outcome {
    while let Some(Reverse((priority, _, turns, id))) = search.heap.pop() {
        let Some(node) = search.nodes.get_mut(id as usize) else {
            continue;
        };
        if node.closed || node.turns != turns {
            continue;
        }
        let state = node.state;
        if state.status == Status::Won {
            return Outcome::Solved {
                plan: search.plan(id),
                optimal: search.settings.goal == Goal::Optimal,
            };
        }
        node.closed = true;
        if let Some(dominance) = search.dominance.as_mut()
            && dominance.check_insert(&state, turns)
        {
            search.stats.dominated += 1;
            continue;
        }
        let limit = search.settings.max_expanded;
        if limit != 0 && search.stats.expanded >= limit {
            // The queue holds every unexpanded state, and the one just taken out has the
            // lowest bound: no plan ends under it (meaningful when looking for the optimum).
            let lower_bound = if search.settings.goal == Goal::Optimal {
                u8::try_from(priority.clamp(0, 255)).unwrap_or(u8::MAX)
            } else {
                0
            };
            return Outcome::Unknown {
                best: search.incumbent_node.map(|won| search.plan(won)),
                lower_bound,
            };
        }
        search.stats.expanded += 1;
        for action in search.actions(&state) {
            let Ok(child) = apply(search.mission, &state, action, &mut ()) else {
                continue;
            };
            let spent = turns + u32::from(action == Action::End);
            if matches!(search.offer(&child, id, spent, action), Offer::Goal) {
                let won = search.incumbent_node.unwrap_or(0);
                return Outcome::Solved {
                    plan: search.plan(won),
                    optimal: false,
                };
            }
        }
    }
    Outcome::Unsolvable
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn bytes_ge(a: &[u8], b: &[u8]) -> bool {
        a.iter().zip(b).all(|(x, y)| x >= y)
    }

    fn entry(trace: u8, floor: u8, turns: u32, progress: [u8; 8], rest: [u8; 16]) -> Entry {
        let word = |at: usize| {
            let mut word = [0u8; 8];
            word.copy_from_slice(&rest[at..at + 8]);
            u64::from_le_bytes(word)
        };
        Entry {
            trace,
            floor,
            turns,
            progress: u64::from_le_bytes(progress),
            rest: [word(0), word(8)],
        }
    }

    proptest! {
        /// The words compare bytes as bytes do, for every byte below 128.
        #[test]
        fn a_word_compares_its_bytes_one_by_one(
            a in proptest::array::uniform8(0u8..128),
            b in proptest::array::uniform8(0u8..128),
        ) {
            prop_assert_eq!(
                all_at_least(u64::from_le_bytes(a), u64::from_le_bytes(b)),
                bytes_ge(&a, &b)
            );
        }

        /// Dominance is the plain comparison on every axis, with or without exact progress.
        #[test]
        fn dominance_is_the_comparison_of_every_axis(
            ta in 0u8..100, tb in 0u8..100,
            fa in 0u8..100, fb in 0u8..100,
            na in 0u32..40, nb in 0u32..40,
            pa in proptest::array::uniform8(0u8..7), pb in proptest::array::uniform8(0u8..7),
            ra in proptest::array::uniform16(0u8..4), rb in proptest::array::uniform16(0u8..4),
            exact in any::<bool>(),
        ) {
            let (a, b) = (entry(ta, fa, na, pa, ra), entry(tb, fb, nb, pb, rb));
            let progress = if exact { pa == pb } else { bytes_ge(&pa, &pb) };
            let plain = ta <= tb && fa <= fb && na <= nb && progress && bytes_ge(&ra, &rb);
            prop_assert_eq!(a.dominates(&b, exact), plain);
        }

        /// A table answers like the map it replaces, whatever the keys and however it grew.
        #[test]
        fn a_table_remembers_every_key_it_was_given(
            keys in proptest::collection::btree_set(any::<u128>(), 0..3000),
        ) {
            let mut table = Table::with_capacity(0);
            for (number, key) in keys.iter().enumerate() {
                table.insert(*key, u32::try_from(number).unwrap());
            }
            for (number, key) in keys.iter().enumerate() {
                prop_assert_eq!(table.get(*key), Some(u32::try_from(number).unwrap()));
            }
            prop_assert_eq!(table.get(7), keys.contains(&7).then(|| {
                u32::try_from(keys.iter().position(|k| *k == 7).unwrap()).unwrap()
            }));
        }
    }

    #[test]
    fn a_bucket_keeps_the_entries_that_nothing_beats_by_increasing_trace() {
        let mission = crate::run::tests::payroll();
        let mut dominance = Dominance::new(&mission);
        let state = mission.initial_state();
        let mut worse = state;
        worse.trace = 9;
        let mut better = state;
        better.trace = 3;
        // The worse one is kept, the better one replaces it, and the worse one is then beaten.
        assert!(!dominance.check_insert(&worse, 2));
        assert!(!dominance.check_insert(&better, 2));
        assert!(dominance.check_insert(&worse, 2));
        assert!(dominance.check_insert(&better, 2));
        let all: usize = dominance.entries.iter().map(Vec::len).sum();
        assert_eq!(all, 1);
    }
}
