//! Bounded fallback for `AutoResolve` when the exact search is cut off: a deterministic
//! beam search over end-of-turn states. It returns a valid plan (an upper bound of the
//! optimal Trace), never a proof.

use std::collections::{BTreeMap, BTreeSet};

use crate::heuristic::{Heuristic, INFEASIBLE};
use crate::model::{Action, Event, Mission, State, Status, legal_actions, step, step_with};
use crate::solver::Plan;

struct Candidate {
    state: State,
    plan: Vec<Action>,
    f: i32,
    /// Distance to the goal (uncredited noise and cycles), second ranking key.
    dist: i32,
}

/// Depth-first walk of one turn from `from`; calls `out` on every end-of-turn state and
/// every won state with the actions that lead to it.
fn walk_turn(
    m: &Mission,
    s: &State,
    path: &mut Vec<Action>,
    seen: &mut BTreeSet<u128>,
    out: &mut impl FnMut(&State, &[Action]),
) {
    let mut actions = Vec::new();
    legal_actions(m, s, &mut actions);
    for a in actions {
        let Ok(child) = step(m, s, a) else { continue };
        path.push(a);
        match a {
            Action::End | Action::Jackout => out(&child, path),
            _ => {
                if child.status == Status::Running && seen.insert(child.pack()) {
                    walk_turn(m, &child, path, seen, out);
                }
            }
        }
        path.pop();
    }
}

/// Beam search of the given `width` (states kept per turn, by lowest `f`).
/// Returns the best won plan of the first turn that has one.
pub fn beam_plan(m: &Mission, start: &State, width: usize) -> Option<Plan> {
    let heur = Heuristic::new(m);
    let mut layer = vec![Candidate {
        state: *start,
        plan: Vec::new(),
        f: 0,
        dist: 0,
    }];
    // Trace never decreases by more than the Spoof charges, so the run ends well before this.
    for _ in 0..100 {
        let mut next: BTreeMap<u128, Candidate> = BTreeMap::new();
        let mut best_won: Option<(u8, Vec<Action>)> = None;
        for cand in &layer {
            let mut path = Vec::new();
            let mut seen = BTreeSet::new();
            seen.insert(cand.state.pack());
            walk_turn(m, &cand.state, &mut path, &mut seen, &mut |child, acts| {
                let mut plan = cand.plan.clone();
                plan.extend_from_slice(acts);
                match child.status {
                    Status::Won => {
                        let better = best_won.as_ref().is_none_or(|(t, _)| child.trace < *t);
                        if better {
                            best_won = Some((child.trace, plan));
                        }
                    }
                    Status::Running => {
                        let (base, h, dist) = heur.parts3(m, child, true);
                        let f = if h >= INFEASIBLE {
                            INFEASIBLE
                        } else {
                            (base + h).max(0)
                        };
                        if f < INFEASIBLE && f < i32::from(m.trace_cap) {
                            let key = child.pack();
                            let c = Candidate {
                                state: *child,
                                plan,
                                f,
                                dist,
                            };
                            match next.get(&key) {
                                Some(old) if old.plan.len() <= c.plan.len() => {}
                                _ => {
                                    next.insert(key, c);
                                }
                            }
                        }
                    }
                    Status::Burned => {}
                }
            });
        }
        if let Some((cost, actions)) = best_won {
            let ends = actions.iter().filter(|a| **a == Action::End).count();
            return Some(Plan {
                cost,
                turns: u16::try_from(ends).unwrap_or(u16::MAX),
                actions,
            });
        }
        let mut ranked: Vec<(i32, i32, u128, Candidate)> =
            next.into_iter().map(|(k, c)| (c.f, c.dist, k, c)).collect();
        ranked.sort_by_key(|(f, d, k, _)| (*f, *d, *k));
        ranked.truncate(width);
        if ranked.is_empty() {
            return None;
        }
        layer = ranked.into_iter().map(|(_, _, _, c)| c).collect();
    }
    None
}

/// Cheapest fallback: no search at all. Plays the action that brings the objective closest
/// at the lowest bound, cloaks when the end of turn would be scanned, spends the Spoof
/// charges once the objectives are looted, and ends the turn when nothing advances.
/// At most a few hundred `step` calls; may fail (`None`) and is rarely optimal.
pub fn greedy_plan(m: &Mission, start: &State) -> Option<Plan> {
    let heur = Heuristic::new(m);
    let mut s = *start;
    let mut plan: Vec<Action> = Vec::new();
    let mut actions = Vec::new();
    for _ in 0..600 {
        if s.status != Status::Running {
            break;
        }
        legal_actions(m, &s, &mut actions);
        if m.objectives_done(&s) {
            let a = if actions.contains(&Action::Spoof) {
                Action::Spoof
            } else {
                Action::Jackout
            };
            s = step(m, &s, a).ok()?;
            plan.push(a);
            continue;
        }
        let (_, _, dist_now) = heur.parts3(m, &s, true);
        let mut best: Option<(i32, i32, Action, State)> = None;
        for &a in &actions {
            let skip = matches!(
                a,
                Action::End
                    | Action::Jackout
                    | Action::Cloak
                    | Action::Spoof
                    | Action::Ghost
                    | Action::Overclock
            );
            if skip {
                continue;
            }
            let Ok(next) = step(m, &s, a) else { continue };
            if next.status != Status::Running {
                continue;
            }
            let (base, h, dist) = heur.parts3(m, &next, true);
            if h >= INFEASIBLE || (dist >= dist_now && a != Action::Loot) {
                continue;
            }
            let key = ((base + h).max(0), dist);
            if best.as_ref().is_none_or(|b| key < (b.0, b.1)) {
                best = Some((key.0, key.1, a, next));
            }
        }
        if let Some((_, _, a, next)) = best {
            plan.push(a);
            s = next;
            continue;
        }
        // Nothing advances this turn: protect the end of turn, then end it.
        if actions.contains(&Action::Cloak) {
            let mut ev = Vec::new();
            let _ = step_with(m, &s, Action::End, &mut ev);
            if ev.iter().any(|e| matches!(e, Event::Scan { .. })) {
                s = step(m, &s, Action::Cloak).ok()?;
                plan.push(Action::Cloak);
            }
        }
        s = step(m, &s, Action::End).ok()?;
        plan.push(Action::End);
    }
    let ends = plan.iter().filter(|a| **a == Action::End).count();
    (s.status == Status::Won).then(|| Plan {
        cost: s.trace,
        turns: u16::try_from(ends).unwrap_or(u16::MAX),
        actions: plan,
    })
}
