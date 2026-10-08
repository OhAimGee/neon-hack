//! Uses of the solver beyond "find the plan": the domination check of invariant I19, the
//! per-turn preview (re-solving from the current state) and the bracket
//! `[lower bound, plan found]` used when the exact search is cut off.

use std::time::{Duration, Instant};

use crate::fallback::beam_plan;
use crate::model::{Action, Mission, Program, Status, step};
use crate::solver::{Config, Outcome, Plan, solvable_within_from, solve, solve_from};

/// Result of the solver on one mission, reduced to what the checks compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Cost(u8),
    Unsolvable,
    Unknown,
}

pub fn verdict(m: &Mission, cfg: &Config) -> (Verdict, Option<Plan>) {
    let r = solve(m, cfg);
    match r.outcome {
        Outcome::Solved { plan, .. } => (Verdict::Cost(plan.cost), Some(plan)),
        Outcome::Unsolvable => (Verdict::Unsolvable, None),
        Outcome::Unknown { .. } => (Verdict::Unknown, None),
    }
}

/// How removing one program changed one mission.
#[derive(Clone, Debug)]
pub struct Removal {
    pub mission: String,
    pub with: Verdict,
    pub without: Verdict,
    /// The optimal plan found with the program uses it.
    pub used_in_plan: bool,
}

impl Removal {
    /// Strictly worse without the program (a higher Trace, or no plan at all).
    pub fn hurts(&self) -> bool {
        match (self.with, self.without) {
            (Verdict::Cost(a), Verdict::Cost(b)) => b > a,
            (Verdict::Cost(_), Verdict::Unsolvable) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DominationRow {
    pub program: Program,
    pub removals: Vec<Removal>,
}

impl DominationRow {
    /// I19: some mission gets strictly worse without the program.
    pub fn is_needed(&self) -> bool {
        self.removals.iter().any(Removal::hurts)
    }

    pub fn is_ever_used(&self) -> bool {
        self.removals.iter().any(|r| r.used_in_plan)
    }
}

/// For every program of every kit: solve again without it.
pub fn domination_check(missions: &[Mission], cfg: &Config) -> Vec<DominationRow> {
    let base: Vec<(Verdict, Option<Plan>)> = missions.iter().map(|m| verdict(m, cfg)).collect();
    let mut rows = Vec::new();
    for p in Program::ALL {
        let mut removals = Vec::new();
        for (m, (with, plan)) in missions.iter().zip(&base) {
            if !m.has(p) || *with == Verdict::Unknown {
                continue;
            }
            let (without, _) = verdict(&m.without(p), cfg);
            let used_in_plan = plan.as_ref().is_some_and(|pl| plan_uses(pl, p));
            removals.push(Removal {
                mission: m.name.clone(),
                with: *with,
                without,
                used_in_plan,
            });
        }
        rows.push(DominationRow {
            program: p,
            removals,
        });
    }
    rows
}

pub fn plan_uses(plan: &Plan, p: Program) -> bool {
    plan.actions.iter().any(|a| match *a {
        Action::Breach { program, .. } => program == p,
        Action::Cloak => p == Program::Cloak,
        Action::Spoof => p == Program::Spoof,
        Action::Ghost => p == Program::Ghost,
        Action::Overclock => p == Program::Overclock,
        _ => false,
    })
}

/// One re-solve of the per-turn preview.
#[derive(Clone, Copy, Debug)]
pub struct TurnSolve {
    pub turn: u16,
    pub expanded: u64,
    pub elapsed: Duration,
    /// Optimal final Trace found from this turn's start (equal to the first one if the
    /// optimum is consistent, which the tests check).
    pub cost: Option<u8>,
}

/// Follows the optimal plan and re-solves from the start of every turn.
pub fn preview_each_turn(m: &Mission, cfg: &Config) -> Vec<TurnSolve> {
    let first = solve(m, cfg);
    let Some(plan) = first.plan() else {
        return Vec::new();
    };
    let mut out = vec![TurnSolve {
        turn: 0,
        expanded: first.stats.expanded,
        elapsed: first.stats.elapsed,
        cost: first.cost(),
    }];
    let mut s = m.initial();
    let mut turn = 0;
    for &a in &plan.actions {
        let Ok(next) = step(m, &s, a) else { break };
        s = next;
        if a == Action::End && s.status == Status::Running {
            turn += 1;
            let r = solve_from(m, &s, cfg);
            out.push(TurnSolve {
                turn,
                expanded: r.stats.expanded,
                elapsed: r.stats.elapsed,
                cost: r.cost(),
            });
        }
    }
    out
}

/// What is known about a mission after a bounded exact search and a beam search.
#[derive(Clone, Debug)]
pub struct Bracket {
    /// Proven: no plan ends under this Trace.
    pub lower: u8,
    /// A real plan ending at this Trace (not necessarily optimal).
    pub upper: Option<u8>,
    /// The exact search finished: `lower == upper` is the optimum.
    pub exact: bool,
    pub expanded: u64,
    pub elapsed: Duration,
}

/// Exact search limited to `cfg.max_expanded` expansions, then a beam of `beam_width`.
pub fn bracket(m: &Mission, cfg: &Config, beam_width: usize) -> Bracket {
    let r = solve(m, cfg);
    match r.outcome {
        Outcome::Solved { plan, .. } => Bracket {
            lower: plan.cost,
            upper: Some(plan.cost),
            exact: true,
            expanded: r.stats.expanded,
            elapsed: r.stats.elapsed,
        },
        Outcome::Unsolvable => Bracket {
            lower: m.trace_cap,
            upper: None,
            exact: true,
            expanded: r.stats.expanded,
            elapsed: r.stats.elapsed,
        },
        Outcome::Unknown { lower_bound, .. } => {
            let t0 = Instant::now();
            let upper = beam_plan(m, &m.initial(), beam_width).map(|p| p.cost);
            Bracket {
                lower: lower_bound,
                upper,
                exact: false,
                expanded: r.stats.expanded,
                elapsed: r.stats.elapsed + t0.elapsed(),
            }
        }
    }
}

/// Anytime fallback: repeated early-exit searches under a shrinking Trace cap, within a
/// total budget of expansions. Each round is a cheap greedy best-first search, so the
/// first plan comes after a few hundred expansions; later rounds only improve it.
/// Returns the best plan found, never a proof.
pub fn anytime_plan(m: &Mission, start: &crate::model::State, budget: u64) -> Option<(Plan, u64)> {
    let mut cfg = Config::tuned();
    cfg.macro_turns = false;
    cfg.dominance = true;
    let mut best: Option<Plan> = None;
    let mut used = 0u64;
    let mut cap = m.trace_cap;
    while used < budget {
        cfg.max_expanded = budget - used;
        let r = solvable_within_from(m, start, cap, &cfg);
        used += r.stats.expanded;
        match r.outcome {
            Outcome::Solved { plan, .. } => {
                if plan.cost == 0 {
                    return Some((plan, used));
                }
                cap = plan.cost;
                best = Some(plan);
            }
            _ => break,
        }
    }
    best.map(|p| (p, used))
}
