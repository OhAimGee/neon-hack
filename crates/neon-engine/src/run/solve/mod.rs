//! The solver of the intrusion: a bounded search for a plan, and a proof that one exists.
//!
//! It is **not** what the player sees: the forecast is one step of the rules ([`forecast`]).
//! The solver answers three other questions (`docs/spec/solver.md`):
//!
//! * *can this intrusion be won at all* with the deck every player has ([`prove_solvable`]):
//!   run by the tests on the content, so that no intrusion ships that cannot be finished;
//! * *how low can the Trace go* with that deck ([`optimum`]): the margin of difficulty;
//! * *what does an automatic intrusion cost* ([`auto_resolve`]): story mode and the campaign
//!   before the tactical screen, a bounded chain that always ends.
//!
//! The search is A* over canonical states with dominance. It is exact when it is allowed to
//! finish and says so when it is not; it never uses the clock (the bound is a number of
//! expansions), so it gives the same answer on every machine.
//!
//! [`forecast`]: super::forecast::forecast

mod bound;
mod key;
mod search;

#[cfg(test)]
mod tests;

use super::action::Action;
use super::mission::Mission;
use super::state::RunState;

/// Expansions the exact step of [`auto_resolve`] may spend.
pub const AUTO_EXACT_EXPANSIONS: u64 = 5_000;
/// Expansions the anytime step of [`auto_resolve`] may spend.
pub const AUTO_ANYTIME_EXPANSIONS: u64 = 20_000;
/// Expansions [`prove_solvable`] and [`optimum`] may spend by default: generous, because they
/// run in tests and tools, never in a game.
pub const PROOF_EXPANSIONS: u64 = 2_000_000;

/// What a search looks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    /// The lowest final Trace.
    Optimal,
    /// Any plan under the cap, as soon as one is found.
    Feasible,
}

/// A way to win: the actions, ending with `Jackout`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The actions, in order.
    pub actions: Vec<Action>,
    /// The Trace the run ends with.
    pub trace: u8,
    /// The turns ended on the way (the first plays in turn 1, so a plan of `n` ends has `n + 1`
    /// turns).
    pub turns: u32,
}

/// How a search ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// A plan. `optimal` says its Trace is the lowest there is.
    Solved {
        /// The plan.
        plan: Plan,
        /// Whether it is proven optimal.
        optimal: bool,
    },
    /// Proven: no plan keeps the Trace under the cap.
    Unsolvable,
    /// The limit was reached. `lower_bound` is proven: no plan ends under it.
    Unknown {
        /// The best plan met so far, if any.
        best: Option<Plan>,
        /// A proven lower bound of the optimal Trace (0 when the goal was not the optimum).
        lower_bound: u8,
    },
}

/// What a search cost.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// States taken from the queue and expanded.
    pub expanded: u64,
    /// Successor states offered.
    pub generated: u64,
    /// States dropped because an expanded one dominated them.
    pub dominated: u64,
}

/// The result of a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// How it ended.
    pub outcome: Outcome,
    /// What it cost.
    pub stats: Stats,
}

impl Report {
    /// The plan found, if any.
    #[must_use]
    pub fn plan(&self) -> Option<&Plan> {
        match &self.outcome {
            Outcome::Solved { plan, .. } => Some(plan),
            Outcome::Unknown { best, .. } => best.as_ref(),
            Outcome::Unsolvable => None,
        }
    }
}

/// Searches for a plan from a state, spending at most `max_expanded` expansions (0 for no
/// limit). Only plans that end under `cap` count; the cap of the mission applies as well.
#[must_use]
pub fn solve(
    mission: &Mission,
    start: &RunState,
    goal: Goal,
    max_expanded: u64,
    cap: u8,
) -> Report {
    search::search(
        mission,
        start,
        search::Settings {
            goal,
            max_expanded,
            cap,
            dominance: true,
        },
    )
}

/// The lowest final Trace of a mission from its start, within `max_expanded` expansions.
#[must_use]
pub fn optimum(mission: &Mission, max_expanded: u64) -> Report {
    solve(
        mission,
        &mission.initial_state(),
        Goal::Optimal,
        max_expanded,
        u8::MAX,
    )
}

/// Whether the mission can be won from its start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proof {
    /// It can: here is a way.
    Solvable(Plan),
    /// It cannot: no plan keeps the Trace under the cap.
    Unsolvable,
    /// The limit was reached before either could be shown.
    Unknown,
}

/// Proves a mission solvable by finding a way (early exit), or unsolvable by exhausting the
/// search. To prove it for the deck every player has, ask for
/// [`Mission::unlimited_only`](super::mission::Mission::unlimited_only).
#[must_use]
pub fn prove_solvable(mission: &Mission, max_expanded: u64) -> Proof {
    let report = solve(
        mission,
        &mission.initial_state(),
        Goal::Feasible,
        max_expanded,
        u8::MAX,
    );
    match report.outcome {
        Outcome::Solved { plan, .. } => Proof::Solvable(plan),
        Outcome::Unsolvable => Proof::Unsolvable,
        Outcome::Unknown { .. } => Proof::Unknown,
    }
}

/// How an automatic intrusion was settled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// The exact search finished: this plan has the lowest Trace there is.
    Optimal(Plan),
    /// A plan, not proven optimal: the best of the exact search cut short, or the first the
    /// anytime search found.
    Best(Plan),
    /// No plan was found within the limits: the caller falls back on the nominal Trace of the
    /// tier, and says the result is approximate.
    Nominal,
}

impl Resolution {
    /// The Trace this resolution ends with, if it found a plan.
    #[must_use]
    pub fn trace(&self) -> Option<u8> {
        match self {
            Self::Optimal(plan) | Self::Best(plan) => Some(plan.trace),
            Self::Nominal => None,
        }
    }
}

/// The bounded chain behind an automatic intrusion (`docs/spec/solver.md` § 2): the exact
/// search within [`AUTO_EXACT_EXPANSIONS`]; if it does not finish, the best plan it met or,
/// failing that, an anytime search within [`AUTO_ANYTIME_EXPANSIONS`]; failing that, nothing.
///
/// The caller keeps the better of the plan and the nominal Trace of the tier, so an automatic
/// intrusion is never worse than the nominal one.
#[must_use]
pub fn auto_resolve(mission: &Mission, start: &RunState) -> Resolution {
    let exact = solve(
        mission,
        start,
        Goal::Optimal,
        AUTO_EXACT_EXPANSIONS,
        u8::MAX,
    );
    match exact.outcome {
        Outcome::Solved { plan, .. } => return Resolution::Optimal(plan),
        Outcome::Unknown {
            best: Some(plan), ..
        } => return Resolution::Best(plan),
        Outcome::Unknown { best: None, .. } | Outcome::Unsolvable => {}
    }
    let anytime = solve(
        mission,
        start,
        Goal::Feasible,
        AUTO_ANYTIME_EXPANSIONS,
        u8::MAX,
    );
    match anytime.outcome {
        Outcome::Solved { plan, .. } => Resolution::Best(plan),
        _ => Resolution::Nominal,
    }
}
