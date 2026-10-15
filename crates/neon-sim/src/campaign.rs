//! Batches of games: which plans, which seeds, which difficulties.

use std::ops::RangeInclusive;

use neon_engine::campaign::Difficulty;
use neon_engine::content::{Content, LoadError};

use crate::brain::{Plan, all_plans, representative};
use crate::record::{GameRecord, difficulty_name, play_game};
use crate::report::CampaignReport;

/// The name of the bot that plays the optimistic plans.
pub const OPTIMIST: &str = "optimist";

/// Which decisions the optimist takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanSet {
    /// Every combination of the options of every decision, with and without the optional
    /// objectives: the 216 plans.
    All,
    /// A subset that covers every option of every decision and every ending.
    Representative,
}

/// The plans of a set.
///
/// # Errors
///
/// The content of the game could not be loaded.
pub fn plans(set: PlanSet) -> Result<Vec<Plan>, LoadError> {
    let content = Content::embedded()?;
    let all = all_plans(&content);
    Ok(match set {
        PlanSet::All => all,
        PlanSet::Representative => representative(&all),
    })
}

/// Plays the optimist with every plan, on every seed, at a difficulty, and sums it up. The
/// order is the plans', then the seeds' : the report does not depend on anything else.
#[must_use]
pub fn run_optimist(
    plans: &[Plan],
    difficulty: Difficulty,
    seeds: &RangeInclusive<u64>,
) -> (CampaignReport, Vec<GameRecord>) {
    let mut records = Vec::new();
    for plan in plans {
        for seed in seeds.clone() {
            records.push(play_game(plan, difficulty, seed));
        }
    }
    let report = CampaignReport::of(
        OPTIMIST,
        difficulty_name(difficulty),
        [*seeds.start(), *seeds.end()],
        plans.len() as u64,
        &records,
    );
    (report, records)
}
