//! The identity of a state for the search: the same place in the run, whatever the turn.

use super::super::mission::{MAX_NODES, MAX_PATROLS, Mission};
use super::super::program::{PROGRAM_COUNT, Program};
use super::super::state::{RunState, Status};

/// The state the search stores: the turn only counts modulo the period of the patrols, the
/// floor of Spoof only counts when the deck has one, and a won run keeps only its Trace (every
/// way of winning with the same Trace is the same).
pub(super) fn canonical(mission: &Mission, state: &RunState) -> RunState {
    let mut canon = *state;
    if state.status == Status::Won {
        canon = mission.initial_state();
        canon.status = Status::Won;
        canon.trace = state.trace;
        canon.turn_floor = 0;
        canon.cycles = 0;
        canon.charges = [0; PROGRAM_COUNT];
        return canon;
    }
    let period = u32::from(mission.period.max(1));
    canon.turn = state.turn.saturating_sub(1) % period + 1;
    if mission.supply(Program::Spoof).is_none() {
        canon.turn_floor = 0;
    }
    canon
}

/// Packs a canonical state in 94 bits. Every field fits its lane: the checks that make it so
/// are in the builder of a mission (nodes, strengths, charges, cycles, cap, period).
pub(super) fn pack(state: &RunState) -> u128 {
    let status = match state.status {
        Status::Running => 0u128,
        Status::Won => 1,
        Status::Burned => 2,
    };
    let mut key = status;
    key |= (state.position.index() as u128) << 2;
    key |= u128::from(state.trace) << 5;
    key |= u128::from(state.turn_floor) << 13;
    key |= u128::from(state.cycles.min(15)) << 21;
    key |= u128::from(state.turn.min(31)) << 25;
    key |= u128::from(state.cloak) << 30;
    key |= u128::from(state.ghost) << 31;
    key |= u128::from(state.loot_taken) << 32;
    for (index, progress) in state.progress.iter().enumerate().take(MAX_NODES) {
        key |= u128::from(*progress & 7) << (40 + 3 * index);
    }
    for (index, down) in state.down.iter().enumerate().take(MAX_PATROLS) {
        key |= u128::from(*down & 3) << (64 + 2 * index);
    }
    for (index, charges) in state.charges.iter().enumerate() {
        key |= u128::from(*charges & 3) << (72 + 2 * index);
    }
    key
}
