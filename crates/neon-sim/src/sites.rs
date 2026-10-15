//! The sites of the net, measured with the solver: can each be won, how low can its Trace go,
//! in how many turns, and what margin does that leave under the cap.
//!
//! Every measure is made with the deck every player has (`unlimited_only`: no consumable), the
//! one the proof of solvability uses, and the exact search of the solver, so that the numbers
//! are the same on every machine. Only the counts of expansions measure the work.

use neon_engine::campaign::Difficulty;
use neon_engine::content::ids::SiteId;
use neon_engine::content::{Content, Stock};
use neon_engine::run::solve::{Goal, Outcome, PROOF_EXPANSIONS, optimum, solve};
use neon_engine::run::{LootKind, Mission, Pace};
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

/// What a measure asks of the intrusion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Loot {
    /// Reach the objective, with the deck every player has (no consumable): the room the site
    /// leaves.
    #[default]
    Objective,
    /// Take every loot of the site, with the whole deck of the tier: what the campaign asks of
    /// the optimist, since the files of the quests are on the nodes that are not the objective.
    All,
}

impl Loot {
    /// Whether this is the default, the one that is not written in the reports.
    #[must_use]
    pub fn is_objective(&self) -> bool {
        matches!(self, Self::Objective)
    }
}

/// The intrusion of a site where every loot is an objective, with the deck of its tier that a
/// player holding `stock` has: the variant the optimist plays, and (with everything) the one the
/// measures of [`Loot::All`] solve.
#[must_use]
pub fn full_loot_mission(
    content: &Content,
    site: &SiteId,
    pace: Pace,
    stock: Stock<'_>,
) -> Option<Mission> {
    let def = content.run(site)?;
    let tier = content.site(site)?.unlock.tier.unwrap_or(1);
    let kit = content.kit(tier)?;
    let mut greedy = def.clone();
    for node in &mut greedy.node {
        if node.loot.is_some() {
            node.loot = Some(LootKind::Objective);
        }
    }
    greedy.mission(kit, &content.deck(kit, stock), pace).ok()
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
    /// What was asked of the intrusion (not written when it is the objective alone).
    #[serde(default, skip_serializing_if = "Loot::is_objective")]
    pub loot: Loot,
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
    loot: Loot,
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
            let mission = match loot {
                Loot::Objective => content
                    .mission(&site.id, level.pace())
                    .map(|mission| mission.unlimited_only()),
                Loot::All => full_loot_mission(content, &site.id, level.pace(), Stock::Everything),
            };
            let Some(mission) = mission else {
                continue;
            };
            let cap = mission.trace_cap();
            // The objective alone asks for the lowest Trace. Taking everything with the whole deck
            // asks only whether it can be won: the exact search is out of reach there, and the
            // plan found is any plan, not the best.
            let report = match loot {
                Loot::Objective => optimum(&mission, max_expanded),
                Loot::All => solve(
                    &mission,
                    &mission.initial_state(),
                    Goal::Feasible,
                    max_expanded,
                    u8::MAX,
                ),
            };
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
                loot,
                result,
            });
        }
    }
    rows
}

/// The expansions the solver may spend on a site by default: those of the proof.
pub const DEFAULT_EXPANSIONS: u64 = PROOF_EXPANSIONS;
