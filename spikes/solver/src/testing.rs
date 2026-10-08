//! Helpers for the property-style tests: random playouts driven by the in-crate PCG32.

use crate::model::{Action, Mission, State, Status, legal_actions, step};
use crate::rng::Pcg32;

/// Picks one legal action uniformly (never `Jackout` unless it is the only winning move).
pub fn random_action(m: &Mission, s: &State, rng: &mut Pcg32) -> Option<Action> {
    let mut v = Vec::new();
    legal_actions(m, s, &mut v);
    if v.is_empty() {
        return None;
    }
    let i = rng.below(u32::try_from(v.len()).unwrap_or(1)) as usize;
    Some(v[i])
}

/// A reachable running state after at most `max_steps` random legal actions.
pub fn random_state(m: &Mission, rng: &mut Pcg32, max_steps: u32) -> State {
    let mut s = m.initial();
    let steps = rng.below(max_steps + 1);
    for _ in 0..steps {
        let Some(a) = random_action(m, &s, rng) else {
            break;
        };
        if a == Action::Jackout {
            continue;
        }
        match step(m, &s, a) {
            Ok(next) if next.status == Status::Running => s = next,
            _ => break,
        }
    }
    s
}

/// A random plan prefix of the current turn: legal actions that neither end the turn
/// nor the run.
pub fn random_prefix(m: &Mission, s: &State, rng: &mut Pcg32, max_len: u32) -> Vec<Action> {
    let mut cur = *s;
    let mut plan = Vec::new();
    for _ in 0..rng.below(max_len + 1) {
        let Some(a) = random_action(m, &cur, rng) else {
            break;
        };
        if matches!(a, Action::End | Action::Jackout) {
            continue;
        }
        let Ok(next) = step(m, &cur, a) else { break };
        if next.status != Status::Running {
            break;
        }
        cur = next;
        plan.push(a);
    }
    plan
}
