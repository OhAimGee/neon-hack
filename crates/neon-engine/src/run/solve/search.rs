//! Best-first search (A*) over canonical states, with dominance.
//!
//! The objective is the final Trace, then the number of turns. The Trace is part of the state,
//! so the cost of a state does not depend on the path: states are ordered by a lower bound of
//! the final Trace ([`Bound`]) and a won state is queued with its exact Trace, so the first
//! won state taken out of the queue is optimal. Everything is deterministic: ordered maps, no
//! seed, no clock.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use super::super::action::{Action, Utility};
use super::super::mission::Mission;
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
#[derive(Clone, Copy)]
struct Entry {
    trace: u8,
    floor: u8,
    turns: u32,
    progress: [u8; 8],
    down: [u8; 4],
    charges: [u8; PROGRAM_COUNT],
}

impl Entry {
    fn of(state: &RunState, turns: u32) -> Self {
        Self {
            trace: state.trace,
            floor: state.turn_floor,
            turns,
            progress: state.progress,
            down: state.down,
            charges: state.charges,
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
            && self
                .progress
                .iter()
                .zip(&other.progress)
                .all(|(a, b)| if exact_progress { a == b } else { a >= b })
            && self.down.iter().zip(&other.down).all(|(a, b)| a >= b)
            && self.charges.iter().zip(&other.charges).all(|(a, b)| a >= b)
    }
}

/// The fields that must be equal for one state to replace another: position, cycles, turn,
/// Cloak, Ghost and loot taken.
type Bucket = (usize, u8, u32, bool, bool, u8);

/// The expanded states, by bucket.
struct Dominance {
    buckets: BTreeMap<Bucket, Vec<Entry>>,
    /// Whether progress must be equal for one state to dominate another (the deck has a Virus).
    exact_progress: bool,
}

impl Dominance {
    fn new(mission: &Mission) -> Self {
        Self {
            buckets: BTreeMap::new(),
            exact_progress: mission.supply(Program::Virus).is_some(),
        }
    }

    /// True if `state` is dominated; otherwise records it, dropping what it dominates.
    fn check_insert(&mut self, state: &RunState, turns: u32) -> bool {
        let entry = Entry::of(state, turns);
        let bucket = self
            .buckets
            .entry((
                state.position.index(),
                state.cycles,
                state.turn,
                state.cloak,
                state.ghost,
                state.loot_taken,
            ))
            .or_default();
        let mut index = 0;
        while let Some(other) = bucket.get(index) {
            if other.dominates(&entry, self.exact_progress) {
                return true;
            }
            if entry.dominates(other, self.exact_progress) {
                bucket.swap_remove(index);
            } else {
                index += 1;
            }
        }
        bucket.push(entry);
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
    index: BTreeMap<u128, u32>,
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
        if let Some(&known) = self.index.get(&key) {
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
    let mut search = Search {
        mission,
        settings,
        bound: Bound::new(mission),
        index: BTreeMap::new(),
        nodes: Vec::new(),
        heap: BinaryHeap::new(),
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
