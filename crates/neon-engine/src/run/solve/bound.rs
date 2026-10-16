//! A lower bound on the final Trace, for the A* search.
//!
//! What a plan must still pay, whatever it does, is:
//!
//! * the noise of the ICE between the player and each objective still to take, at the best
//!   noise per unit of progress the deck has for the family of each (a Ghost charge can cancel
//!   a turn of it, which is credited);
//! * the ambient Trace of the extra turns that the cycles needed will take;
//!
//! and what it may take back is at most 10 per Spoof charge, never below the Trace of the start
//! of the turn. Scans and the extra noise of the danger band are left out: they only add.

use std::cell::RefCell;

use super::super::mission::{MAX_NODES, Mission};
use super::super::program::{Family, Program, Supply};
use super::super::state::{RunState, Status};
use super::super::{OVERCLOCK_CYCLES, SPOOF_AMOUNT};

/// "Cannot be done with this deck."
const HOPELESS: i32 = i32::MAX / 4;

/// What the bound says of a state.
#[derive(Debug, Clone, Copy)]
pub(super) struct Estimate {
    /// A lower bound of the final Trace.
    pub(super) floor: i32,
    /// How far the goal is, as noise and cycles still to spend (a tie-break, not a bound).
    pub(super) distance: i32,
}

pub(super) struct Bound {
    /// `noise[family][remaining]`: least noise to clear `remaining` units of progress.
    noise: [[i32; 7]; 4],
    /// `cycles[family][remaining]`: least cycles to do so.
    cycles: [[i32; 7]; 4],
    has_attack: bool,
    ambient: i32,
    per_turn: i32,
    /// Noise a Ghost charge can cancel in one turn.
    ghost_turn: i32,
    ghost_unlimited: bool,
    /// What [`Bound::path_needs`] answered lately, by what it depends on (the position, the
    /// progress on every node, the objectives left): the search meets the same ones again and
    /// again, one per turn and per Trace. A direct-mapped table, so that nothing depends on the
    /// order of the questions but the questions themselves.
    memo: RefCell<Vec<(u64, (i32, i32))>>,
}

/// Slots of the memo of [`Bound::path_needs`] (a power of two).
const MEMO_SLOTS: usize = 1 << 12;

/// One cost per node, read and written without indexing.
struct Costs([i32; MAX_NODES]);

impl Costs {
    fn filled(value: i32) -> Self {
        Self([value; MAX_NODES])
    }

    fn get(&self, node: usize) -> i32 {
        self.0.get(node).copied().unwrap_or(HOPELESS)
    }

    fn set(&mut self, node: usize, value: i32) {
        if let Some(slot) = self.0.get_mut(node) {
            *slot = value;
        }
    }

    /// Lowers the cost of `to` if going there from `from` is cheaper; true if it did.
    fn relax(&mut self, from: usize, to: usize, weight: &Self) -> bool {
        let reached = self.get(from);
        if reached >= HOPELESS {
            return false;
        }
        let through = reached + weight.get(to);
        if through < self.get(to) {
            self.set(to, through);
            return true;
        }
        false
    }
}

fn less(a: (u32, u32), b: (u32, u32)) -> bool {
    u64::from(a.0) * u64::from(b.1) < u64::from(b.0) * u64::from(a.1)
}

fn ceil_mul(remaining: u32, ratio: (u32, u32)) -> i32 {
    i32::try_from((remaining * ratio.0).div_ceil(ratio.1.max(1))).unwrap_or(HOPELESS)
}

impl Bound {
    pub(super) fn new(mission: &Mission) -> Self {
        let mut noise_ratio = [(u32::MAX, 1u32); 4];
        let mut cycle_ratio = [(u32::MAX, 1u32); 4];
        let mut best_power_per_cycle = (0u32, 1u32);
        let mut has_attack = false;
        for program in Program::ALL {
            if !program.is_breach() || mission.supply(program).is_none() {
                continue;
            }
            has_attack = true;
            let spec = program.spec();
            for (index, family) in Family::ALL.into_iter().enumerate() {
                let power = u32::from(program.power_against(family));
                let noise = (
                    u32::from(spec.noise) * u32::from(mission.gain_quarters),
                    4 * power,
                );
                let cycles = (u32::from(spec.cycles), power);
                if let Some(slot) = noise_ratio.get_mut(index)
                    && (slot.0 == u32::MAX || less(noise, *slot))
                {
                    *slot = noise;
                }
                if let Some(slot) = cycle_ratio.get_mut(index)
                    && (slot.0 == u32::MAX || less(cycles, *slot))
                {
                    *slot = cycles;
                }
                if less(best_power_per_cycle, (power, u32::from(spec.cycles))) {
                    best_power_per_cycle = (power, u32::from(spec.cycles));
                }
            }
        }
        // A Ghost turn cancels the noise of the actions after the Ghost: at most
        // `capacity - 1` cycles of breach work, each unit of progress counted at the dearest of
        // the best ratios of the families.
        let overclock = match mission.supply(Program::Overclock) {
            Some(Supply::Limited(count)) => u32::from(count) * u32::from(OVERCLOCK_CYCLES),
            _ => 0,
        };
        let capacity = u32::from(mission.cycles_per_turn) + overclock;
        let mut dearest = (0u32, 1u32);
        for ratio in noise_ratio {
            if ratio.0 != u32::MAX && less(dearest, ratio) {
                dearest = ratio;
            }
        }
        let units = capacity.saturating_sub(1) * best_power_per_cycle.0;
        let ghost_turn = i32::try_from(
            (units * dearest.0).div_ceil((best_power_per_cycle.1 * dearest.1).max(1)),
        )
        .unwrap_or(HOPELESS);
        let mut noise = [[0i32; 7]; 4];
        let mut cycles = [[0i32; 7]; 4];
        if has_attack {
            for family in 0..4 {
                for remaining in 1..7u32 {
                    if let (Some(n), Some(c)) = (
                        noise
                            .get_mut(family)
                            .and_then(|row| row.get_mut(remaining as usize)),
                        cycles
                            .get_mut(family)
                            .and_then(|row| row.get_mut(remaining as usize)),
                    ) {
                        *n = ceil_mul(
                            remaining,
                            noise_ratio.get(family).copied().unwrap_or((0, 1)),
                        );
                        *c = ceil_mul(
                            remaining,
                            cycle_ratio.get(family).copied().unwrap_or((0, 1)),
                        );
                    }
                }
            }
        }
        Self {
            noise,
            cycles,
            has_attack,
            ambient: i32::from(mission.gain(mission.ambient)),
            per_turn: i32::from(mission.cycles_per_turn).max(1),
            ghost_turn,
            ghost_unlimited: mission.supply(Program::Ghost) == Some(Supply::Unlimited),
            memo: RefCell::new(vec![(u64::MAX, (0, 0)); MEMO_SLOTS]),
        }
    }

    /// Noise and cycles to reach every objective still to take, by the cheapest way.
    fn path_needs(&self, mission: &Mission, state: &RunState, remaining: u8) -> (i32, i32) {
        // 3 bits for the node, 3 bits of progress per node, 8 bits for the objectives left.
        let mut key = (state.position.index() as u64) | (u64::from(remaining) << 3);
        for (index, progress) in state.progress.iter().enumerate().take(MAX_NODES) {
            key |= u64::from(*progress & 7) << (11 + 3 * index);
        }
        // A multiplicative hash, whose top bits pick the slot.
        let slot = (key.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 52) as usize % MEMO_SLOTS;
        if let Some(&(known, needs)) = self.memo.borrow().get(slot)
            && known == key
        {
            return needs;
        }
        let needs = self.path_needs_uncached(mission, state, remaining);
        if let Some(entry) = self.memo.borrow_mut().get_mut(slot) {
            *entry = (key, needs);
        }
        needs
    }

    fn path_needs_uncached(
        &self,
        mission: &Mission,
        state: &RunState,
        remaining: u8,
    ) -> (i32, i32) {
        let mut weight_noise = Costs::filled(0);
        let mut weight_cycles = Costs::filled(1);
        for node in mission.nodes() {
            let Some(ice) = mission.ice(node) else {
                continue;
            };
            let left = usize::from(ice.strength.saturating_sub(state.progress(node)));
            if left == 0 {
                continue;
            }
            if !self.has_attack {
                return (HOPELESS, HOPELESS);
            }
            let family = Family::ALL
                .iter()
                .position(|f| *f == ice.family)
                .unwrap_or(0);
            if let (Some(n), Some(c)) = (
                self.noise.get(family).and_then(|row| row.get(left)),
                self.cycles.get(family).and_then(|row| row.get(left)),
            ) {
                weight_noise.set(node.index(), *n);
                weight_cycles.set(node.index(), 1 + *c);
            }
        }
        let mut noise_to = Costs::filled(HOPELESS);
        let mut cycles_to = Costs::filled(HOPELESS);
        noise_to.set(state.position.index(), 0);
        cycles_to.set(state.position.index(), 0);
        loop {
            let mut changed = false;
            for from in mission.nodes() {
                for to in mission.neighbours(from) {
                    changed |= noise_to.relax(from.index(), to.index(), &weight_noise);
                    changed |= cycles_to.relax(from.index(), to.index(), &weight_cycles);
                }
            }
            if !changed {
                break;
            }
        }
        let (mut noise_need, mut cycle_need) = (0, 0);
        for node in mission.nodes() {
            if remaining & node.bit() == 0 {
                continue;
            }
            if noise_to.get(node.index()) >= HOPELESS {
                return (HOPELESS, HOPELESS);
            }
            noise_need = noise_need.max(noise_to.get(node.index()));
            cycle_need = cycle_need.max(cycles_to.get(node.index()));
        }
        (noise_need, cycle_need)
    }

    /// What is left to pay from a running state, or `None` when it cannot be won.
    pub(super) fn estimate(&self, mission: &Mission, state: &RunState) -> Option<Estimate> {
        let spoof = match state.supply(mission, Program::Spoof) {
            Some(Supply::Limited(count)) => i32::from(count),
            _ => 0,
        };
        let trace = i32::from(state.trace);
        let held_back = trace - spoof * i32::from(SPOOF_AMOUNT);
        if state.status == Status::Won {
            return Some(Estimate {
                floor: trace,
                distance: 0,
            });
        }
        let remaining = mission.objective_mask & !state.loot_taken;
        let (noise_need, cycle_need) = if remaining == 0 {
            (0, 0)
        } else {
            self.path_needs(mission, state, remaining)
        };
        if noise_need >= HOPELESS {
            return None;
        }
        let cycle_need = cycle_need + i32::try_from(remaining.count_ones()).unwrap_or(0);
        let overclock = match state.supply(mission, Program::Overclock) {
            Some(Supply::Limited(count)) => i32::from(count),
            _ => 0,
        };
        let capacity = i32::from(state.cycles) + i32::from(OVERCLOCK_CYCLES) * overclock;
        let missing = u32::try_from((cycle_need - capacity).max(0)).unwrap_or(0);
        let per_turn = u32::try_from(self.per_turn).unwrap_or(1);
        let extra_turns = i32::try_from(missing.div_ceil(per_turn)).unwrap_or(HOPELESS);
        let ambient = extra_turns.saturating_mul(self.ambient);
        let credit = if self.ghost_unlimited {
            HOPELESS
        } else {
            let ghosts = match state.supply(mission, Program::Ghost) {
                Some(Supply::Limited(count)) => i32::from(count),
                _ => 0,
            } + i32::from(state.ghost);
            ghosts.saturating_mul(self.ghost_turn)
        };
        let noise = (noise_need - credit).max(0);
        let floor = (held_back + noise + ambient)
            .max(i32::from(state.turn_floor))
            .max(0);
        Some(Estimate {
            floor,
            distance: noise_need + ambient + cycle_need,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::rules::{apply, legal_actions};
    use super::*;
    use crate::rng::Pcg32;

    /// What the bound says does not depend on what it was asked before: the memo of the paths
    /// only saves the work.
    #[test]
    fn the_memo_of_the_paths_never_changes_an_estimate() {
        let mission = crate::run::tests::payroll();
        let remembering = Bound::new(&mission);
        let mut rng = Pcg32::new(7, 11);
        for _ in 0..40 {
            let mut state = mission.initial_state();
            for _ in 0..60 {
                let fresh = Bound::new(&mission).estimate(&mission, &state);
                let asked = remembering.estimate(&mission, &state);
                let again = remembering.estimate(&mission, &state);
                assert_eq!(
                    asked.map(|e| (e.floor, e.distance)),
                    fresh.map(|e| (e.floor, e.distance))
                );
                assert_eq!(
                    again.map(|e| (e.floor, e.distance)),
                    fresh.map(|e| (e.floor, e.distance))
                );
                let actions = legal_actions(&mission, &state);
                if actions.is_empty() {
                    break;
                }
                let pick = rng.next_u32() as usize % actions.len();
                let Ok(next) = apply(&mission, &state, actions[pick], &mut ()) else {
                    break;
                };
                if next.status != Status::Running {
                    break;
                }
                state = next;
            }
        }
    }
}
