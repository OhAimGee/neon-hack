//! The sites of the net, measured with the solver: can each be won, how low can its Trace go,
//! in how many turns, and what margin does that leave under the cap.
//!
//! Every measure is made with the deck every player has (`unlimited_only`: no consumable), the
//! one the proof of solvability uses, and the exact search of the solver, so that the numbers
//! are the same on every machine. Only the counts of expansions measure the work.

use neon_engine::campaign::Difficulty;
use neon_engine::content::Content;
use neon_engine::run::solve::{Outcome, PROOF_EXPANSIONS, optimum};
use serde::{Deserialize, Serialize};

use crate::record::difficulty_name;

/// What the solver found for a site at a difficulty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SiteResult {
    /// A plan that wins.
    Solved {
        /// The lowest Trace a win ends on (when `optimal`), else the best found.
        trace: u8,
        /// The cap of the Trace minus `trace`: how much room the minimal deck leaves.
        margin: u8,
        /// The turns of the plan.
        turns: u32,
        /// Whether `trace` is proven to be the lowest there is.
        optimal: bool,
        /// The states the search expanded.
        expanded: u64,
    },
    /// Proven: no plan keeps the Trace under the cap.
    Unsolvable,
    /// The search ran out of expansions before it knew.
    Unknown {
        /// The states the search expanded.
        expanded: u64,
    },
}

/// A site at a difficulty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteRow {
    /// The site.
    pub site: String,
    /// The tier of the deck the site is played with.
    pub tier: u8,
    /// The difficulty, by its name.
    pub difficulty: String,
    /// The cap of the Trace.
    pub cap: u8,
    /// What the solver found.
    pub result: SiteResult,
}

/// Measures every site (or only `only`) at every difficulty (or only `difficulty`), in the
/// order of the content.
#[must_use]
pub fn measure(
    content: &Content,
    only: Option<&str>,
    difficulty: Option<Difficulty>,
    max_expanded: u64,
) -> Vec<SiteRow> {
    let mut rows = Vec::new();
    for site in &content.sites {
        if only.is_some_and(|wanted| wanted != site.id.as_str()) {
            continue;
        }
        for level in Difficulty::ALL {
            if difficulty.is_some_and(|wanted| wanted != level) {
                continue;
            }
            let Some(mission) = content.mission(&site.id, level.pace()) else {
                continue;
            };
            let mission = mission.unlimited_only();
            let cap = mission.trace_cap();
            let report = optimum(&mission, max_expanded);
            let expanded = report.stats.expanded;
            let result = match report.outcome {
                Outcome::Solved { plan, optimal } => SiteResult::Solved {
                    trace: plan.trace,
                    margin: cap.saturating_sub(plan.trace),
                    turns: plan.turns,
                    optimal,
                    expanded,
                },
                Outcome::Unsolvable => SiteResult::Unsolvable,
                Outcome::Unknown { best, .. } => match best {
                    Some(plan) => SiteResult::Solved {
                        trace: plan.trace,
                        margin: cap.saturating_sub(plan.trace),
                        turns: plan.turns,
                        optimal: false,
                        expanded,
                    },
                    None => SiteResult::Unknown { expanded },
                },
            };
            rows.push(SiteRow {
                site: site.id.to_string(),
                tier: site.unlock.tier.unwrap_or(1),
                difficulty: difficulty_name(level).to_owned(),
                cap,
                result,
            });
        }
    }
    rows
}

/// The expansions the solver may spend on a site by default: those of the proof.
pub const DEFAULT_EXPANSIONS: u64 = PROOF_EXPANSIONS;
