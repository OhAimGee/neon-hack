//! The rules: one function that applies an action, and the list of what it would accept.

use super::action::{Action, ActionError, Fact, Sink, Utility};
use super::mission::{Mission, NodeId};
use super::program::{Program, Supply};
use super::state::{RunState, Status, TraceBand};
use super::{DANGER_EXTRA_NOISE, OVERCLOCK_CYCLES, SCAN_NOISE, SPOOF_AMOUNT, VIRUS_TURNS};

/// Applies an action to a state and returns the next state, telling what happened.
///
/// This is the only place the rules live: the forecast, the session and the solver all go
/// through it. A refused action returns the error and changes nothing.
///
/// # Errors
///
/// Returns the [`ActionError`] that says why the action is not allowed.
pub fn apply<S: Sink>(
    mission: &Mission,
    state: &RunState,
    action: Action,
    sink: &mut S,
) -> Result<RunState, ActionError> {
    if state.status != Status::Running {
        return Err(ActionError::RunOver);
    }
    let mut next = *state;
    match action {
        Action::Move(to) => {
            if !mission.is_linked(next.position, to) {
                return Err(ActionError::NotLinked);
            }
            if !next.is_open(mission, to) {
                return Err(ActionError::IceBlocks);
            }
            spend(&mut next, 1)?;
            next.position = to;
            sink.fact(Fact::Moved { to });
        }
        Action::Breach { node, program } => breach(mission, &mut next, node, program, sink)?,
        Action::Use(utility) => use_utility(mission, &mut next, utility, sink)?,
        Action::Loot => {
            let here = next.position;
            if mission.loot(here).is_none() || next.is_looted(here) {
                return Err(ActionError::NothingToLoot);
            }
            spend(&mut next, 1)?;
            next.loot_taken |= here.bit();
            sink.fact(Fact::Looted { node: here });
        }
        Action::Jackout => {
            if !next.objectives_done(mission) {
                return Err(ActionError::ObjectivesMissing);
            }
            next.status = Status::Won;
            sink.fact(Fact::Ended {
                status: Status::Won,
            });
        }
        Action::End => end_turn(mission, &mut next, sink),
    }
    Ok(next)
}

fn spend(state: &mut RunState, cost: u8) -> Result<(), ActionError> {
    state.cycles = state
        .cycles
        .checked_sub(cost)
        .ok_or(ActionError::NotEnoughCycles)?;
    Ok(())
}

/// Takes one use of a program from the deck.
fn take_charge(
    mission: &Mission,
    state: &mut RunState,
    program: Program,
) -> Result<(), ActionError> {
    match mission.supply(program) {
        None => Err(ActionError::NotInDeck(program)),
        Some(Supply::Unlimited) => Ok(()),
        Some(Supply::Limited(_)) => {
            let charges = state
                .charges
                .get_mut(program.index())
                .ok_or(ActionError::NotInDeck(program))?;
            *charges = charges
                .checked_sub(1)
                .ok_or(ActionError::NoCharges(program))?;
            Ok(())
        }
    }
}

/// The Trace an action makes: nothing under Ghost, two more per breach in the danger band,
/// scaled by the mission.
fn action_noise(mission: &Mission, state: &RunState, raw: u8, is_breach: bool) -> u8 {
    if state.ghost {
        return 0;
    }
    let extra = if is_breach && TraceBand::of(state.trace) == TraceBand::Danger {
        DANGER_EXTRA_NOISE
    } else {
        0
    };
    mission.gain(raw.saturating_add(extra))
}

/// Adds Trace; the run is burned the moment the Trace reaches the cap.
fn add_trace<S: Sink>(mission: &Mission, state: &mut RunState, added: u8, sink: &mut S) {
    if added == 0 || state.status != Status::Running {
        return;
    }
    state.trace = state.trace.saturating_add(added);
    sink.fact(Fact::TraceUp { added });
    if state.trace >= mission.trace_cap() {
        state.status = Status::Burned;
        sink.fact(Fact::Ended {
            status: Status::Burned,
        });
    }
}

fn breach<S: Sink>(
    mission: &Mission,
    state: &mut RunState,
    node: NodeId,
    program: Program,
    sink: &mut S,
) -> Result<(), ActionError> {
    if !program.is_breach() {
        return Err(ActionError::NotABreach(program));
    }
    if !mission.is_linked(state.position, node) {
        return Err(ActionError::NotLinked);
    }
    let ice = mission
        .ice(node)
        .filter(|_| !state.is_open(mission, node))
        .ok_or(ActionError::NoIce)?;
    if program == Program::Social && !state.has_intel(mission) {
        return Err(ActionError::NeedsIntel);
    }
    let spec = program.spec();
    spend(state, spec.cycles)?;
    take_charge(mission, state, program)?;
    let power = program.power_against(ice.family);
    let from = state.progress(node);
    let to = from.saturating_add(power).min(ice.strength);
    if let Some(progress) = state.progress.get_mut(node.index()) {
        *progress = to;
    }
    sink.fact(Fact::Breached {
        node,
        program,
        power,
        from,
        to,
    });
    if to == ice.strength {
        sink.fact(Fact::Opened { node });
    }
    if program == Program::Virus {
        // A Virus also puts down every patrol standing on the node right now: the guardian
        // process of the node, or a patrol passing by.
        for patrol in 0..mission.patrol_count() {
            if mission.patrol_position(patrol, state.turn) == Some(node)
                && let Some(down) = state.down.get_mut(patrol)
            {
                *down = VIRUS_TURNS;
                sink.fact(Fact::PatrolDown {
                    patrol: u8::try_from(patrol).unwrap_or(u8::MAX),
                });
            }
        }
    }
    let noise = action_noise(mission, state, spec.noise, true);
    add_trace(mission, state, noise, sink);
    Ok(())
}

fn use_utility<S: Sink>(
    mission: &Mission,
    state: &mut RunState,
    utility: Utility,
    sink: &mut S,
) -> Result<(), ActionError> {
    let program = utility.program();
    let spec = program.spec();
    match utility {
        Utility::Cloak | Utility::Ghost => {
            let active = if utility == Utility::Cloak {
                state.cloak
            } else {
                state.ghost
            };
            if active {
                return Err(ActionError::AlreadyActive(utility));
            }
            spend(state, spec.cycles)?;
            take_charge(mission, state, program)?;
            if utility == Utility::Cloak {
                // "Cancels the next scan" means every scan of the end of this turn.
                state.cloak = true;
                sink.fact(Fact::CloakOn);
            } else {
                // Ghost silences the actions that follow, not the noise already made, nor
                // the ambient Trace, nor the scans.
                state.ghost = true;
                sink.fact(Fact::GhostOn);
            }
        }
        Utility::Spoof => {
            // Spoof erases what this turn made, never what the turns before left.
            let removed = state
                .trace
                .saturating_sub(state.turn_floor)
                .min(SPOOF_AMOUNT);
            if removed == 0 {
                return Err(ActionError::NothingToErase);
            }
            spend(state, spec.cycles)?;
            take_charge(mission, state, program)?;
            state.trace -= removed;
            sink.fact(Fact::Erased { removed });
        }
        Utility::Overclock => {
            spend(state, spec.cycles)?;
            take_charge(mission, state, program)?;
            state.cycles = state.cycles.saturating_add(OVERCLOCK_CYCLES);
            sink.fact(Fact::ExtraCycles {
                count: OVERCLOCK_CYCLES,
            });
            let noise = action_noise(mission, state, spec.noise, false);
            add_trace(mission, state, noise, sink);
        }
    }
    Ok(())
}

/// The end of the turn: patrols move, they scan from where they stand, the ambient Trace
/// rises, and the next turn begins.
fn end_turn<S: Sink>(mission: &Mission, state: &mut RunState, sink: &mut S) {
    let old_turn = state.turn;
    state.turn = old_turn.saturating_add(1);
    // 1. Patrols move.
    for patrol in 0..mission.patrol_count() {
        let from = mission.patrol_position(patrol, old_turn);
        let to = mission.patrol_position(patrol, state.turn);
        if let (Some(from), Some(to)) = (from, to)
            && mission.patrol_route(patrol).len() > 1
        {
            sink.fact(Fact::PatrolMoved {
                patrol: u8::try_from(patrol).unwrap_or(u8::MAX),
                from,
                to,
            });
        }
    }
    // 2. Scans, from the new positions. Seeing two nodes away depends on the Trace as it was
    //    before the scans, so the result does not depend on the order of the patrols.
    let trace_before = state.trace;
    for patrol in 0..mission.patrol_count() {
        if !state.patrol_scans(mission, patrol) {
            continue;
        }
        let Some(at) = mission.patrol_position(patrol, state.turn) else {
            continue;
        };
        let reach = if trace_before >= TraceBand::ALERT_AT {
            2
        } else {
            1
        };
        if mission.distance(at, state.position) > reach {
            continue;
        }
        let patrol = u8::try_from(patrol).unwrap_or(u8::MAX);
        if state.cloak {
            sink.fact(Fact::ScanCancelled {
                patrol,
                at,
                target: state.position,
            });
        } else {
            let added = mission.gain(SCAN_NOISE);
            sink.fact(Fact::Scanned {
                patrol,
                at,
                target: state.position,
                added,
            });
            add_trace(mission, state, added, sink);
            if state.status != Status::Running {
                // Burned: later patrols must not announce Trace that is never added.
                return;
            }
        }
    }
    // 3. The patrols that were down are one turn closer to scanning again.
    for down in &mut state.down {
        *down = down.saturating_sub(1);
    }
    // 4. Ambient Trace: waiting is never free.
    let added = mission.gain(mission.ambient());
    sink.fact(Fact::Ambient { added });
    add_trace(mission, state, added, sink);
    if state.status != Status::Running {
        return;
    }
    state.cloak = false;
    state.ghost = false;
    state.cycles = mission.cycles_per_turn();
    state.turn_floor = state.trace;
    sink.fact(Fact::TurnStarted { turn: state.turn });
}

/// Every action the rules accept in a state, in a fixed order: moves and breaches by node,
/// then utilities, loot, `End` and `Jackout`.
///
/// It is written out rather than found by trying everything, and a test holds it equal to
/// "what [`apply`] accepts".
#[must_use]
pub fn legal_actions(mission: &Mission, state: &RunState) -> Vec<Action> {
    let mut actions = Vec::new();
    if state.status != Status::Running {
        return actions;
    }
    for node in mission.nodes() {
        if !mission.is_linked(state.position, node) {
            continue;
        }
        if state.is_open(mission, node) {
            if state.cycles >= 1 {
                actions.push(Action::Move(node));
            }
            continue;
        }
        for program in Program::ALL {
            if program.is_breach()
                && state.can_use(mission, program)
                && state.cycles >= program.spec().cycles
                && (program != Program::Social || state.has_intel(mission))
            {
                actions.push(Action::Breach { node, program });
            }
        }
    }
    for utility in Utility::ALL {
        let program = utility.program();
        let allowed = state.can_use(mission, program)
            && state.cycles >= program.spec().cycles
            && match utility {
                Utility::Cloak => !state.cloak,
                Utility::Ghost => !state.ghost,
                Utility::Spoof => state.trace > state.turn_floor,
                Utility::Overclock => true,
            };
        if allowed {
            actions.push(Action::Use(utility));
        }
    }
    if state.cycles >= 1
        && mission.loot(state.position).is_some()
        && !state.is_looted(state.position)
    {
        actions.push(Action::Loot);
    }
    actions.push(Action::End);
    if state.objectives_done(mission) {
        actions.push(Action::Jackout);
    }
    actions
}
