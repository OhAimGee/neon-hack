//! Independent exhaustive checker for small missions.
//!
//! Shares only [`step`] with the solver. It does not use the action generator, the packed
//! state, the heuristic, the queue, the dominance test nor the canonical filters: it tries
//! every conceivable action, explores every state reachable inside a turn (merging only
//! identical `State` values) and keeps whole `State` values in `BTreeSet`s, layer by layer
//! (one layer per turn).

use std::collections::BTreeSet;

use crate::model::{Action, Mission, PROGRAM_COUNT, Program, State, Status, step};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BruteResult {
    /// Lowest final Trace over all winning plans, `None` if no plan wins.
    pub best_cost: Option<u8>,
    /// Fewest turns among the plans that reach `best_cost`.
    pub best_turns: Option<u16>,
    /// Distinct end-of-turn states met.
    pub states: u64,
    /// Transitions tried (every accepted action from every state met).
    pub sequences: u64,
}

/// Every action value that exists for this mission, legal or not.
fn universe(m: &Mission) -> Vec<Action> {
    let mut v = Vec::new();
    for n in 0..m.n as u8 {
        v.push(Action::Move(n));
        for p in Program::ALL {
            if p.is_breach() {
                v.push(Action::Breach {
                    node: n,
                    program: p,
                });
            }
        }
    }
    v.extend([
        Action::Cloak,
        Action::Spoof,
        Action::Ghost,
        Action::Overclock,
        Action::Loot,
    ]);
    v.push(Action::Jackout);
    v.push(Action::End);
    debug_assert!(PROGRAM_COUNT == Program::ALL.len());
    v
}

struct Ctx<'a> {
    m: &'a Mission,
    all: Vec<Action>,
    next: BTreeSet<State>,
    seen: BTreeSet<State>,
    won: Option<u8>,
    sequences: u64,
}

fn turn(ctx: &mut Ctx<'_>, s: &State) {
    for i in 0..ctx.all.len() {
        let a = ctx.all[i];
        let Ok(child) = step(ctx.m, s, a) else {
            continue;
        };
        ctx.sequences += 1;
        match (a, child.status) {
            (_, Status::Burned) => {}
            (_, Status::Won) => {
                ctx.won = Some(ctx.won.map_or(child.trace, |w| w.min(child.trace)));
            }
            (Action::End, _) => {
                ctx.next.insert(child);
            }
            _ => {
                if ctx.seen.insert(child) {
                    turn(ctx, &child);
                }
            }
        }
    }
}

/// Exhaustive search from the mission's initial state.
pub fn brute_force(m: &Mission) -> BruteResult {
    let mut layer: BTreeSet<State> = BTreeSet::new();
    layer.insert(m.initial());
    let mut best: Option<(u8, u16)> = None;
    let mut states = 0u64;
    let mut sequences = 0u64;
    let mut turns = 0u16;
    while !layer.is_empty() {
        let mut ctx = Ctx {
            m,
            all: universe(m),
            next: BTreeSet::new(),
            seen: BTreeSet::new(),
            won: None,
            sequences: 0,
        };
        for s in &layer {
            states += 1;
            ctx.seen.clear();
            turn(&mut ctx, s);
        }
        sequences += ctx.sequences;
        if let Some(w) = ctx.won
            && best.is_none_or(|(c, _)| w < c)
        {
            best = Some((w, turns));
        }
        layer = ctx.next;
        turns += 1;
        // Stop when no later layer can beat the best plan: every end of turn adds at least
        // the ambient gain, and the Spoof charges remove at most 10 each, once.
        if let Some((c, _)) = best {
            let spoof = i32::from(m.kit[Program::Spoof.index()]);
            let floor = i32::from(turns) * i32::from(m.gain(m.ambient)) - 10 * spoof;
            if floor >= i32::from(c) {
                break;
            }
        }
    }
    BruteResult {
        best_cost: best.map(|b| b.0),
        best_turns: best.map(|b| b.1),
        states,
        sequences,
    }
}
