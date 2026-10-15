//! Batches of games: which plans, which seeds, which difficulties.

use std::ops::RangeInclusive;

use neon_engine::campaign::Difficulty;
use neon_engine::content::{Content, LoadError};

use crate::brain::{Plan, all_plans, representative};
use crate::driver::Tactic;
use crate::record::{GameRecord, difficulty_name, play_game};
use crate::report::CampaignReport;
use crate::tactics::Style;

/// The name of the bot that plays the optimistic plans.
pub const OPTIMIST: &str = "optimist";
/// The name of the bot that plays like a reasonable player.
pub const PRUDENT: &str = "prudent";
/// The name of the bot that plays like a reckless one.
pub const RECKLESS: &str = "reckless";
/// The name of the bot that skips the intrusions that last.
pub const STORY: &str = "story";

/// A bot: the campaign it plays is the plan's, what differs is how it plays an intrusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bot {
    /// Follows the plan of the solver: never burns.
    Optimist,
    /// Plays by hand with the quietest program, the forecast, the cloak and the spoof.
    Prudent,
    /// Plays by hand with the strongest program, and nothing else.
    Reckless,
    /// Plays the Story difficulty and has the game play every intrusion that lasts.
    Story,
}

impl Bot {
    /// The name written in the reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Optimist => OPTIMIST,
            Self::Prudent => PRUDENT,
            Self::Reckless => RECKLESS,
            Self::Story => STORY,
        }
    }

    /// The only difficulty it plays, when it plays one only.
    #[must_use]
    pub const fn only_difficulty(self) -> Option<Difficulty> {
        match self {
            Self::Story => Some(Difficulty::Story),
            Self::Optimist | Self::Prudent | Self::Reckless => None,
        }
    }

    /// How it plays an intrusion.
    #[must_use]
    pub const fn tactic(self) -> Tactic {
        match self {
            Self::Optimist => Tactic::Solver,
            Self::Prudent => Tactic::Hand(Style::Prudent),
            Self::Reckless => Tactic::Hand(Style::Reckless),
            Self::Story => Tactic::Story,
        }
    }

    /// The notoriety from which it buys cover on its own: the careful player does not let the
    /// market close on it, the others only do what a quest asks. The band "watched" starts at
    /// 30.
    #[must_use]
    pub const fn cools_from(self) -> Option<u8> {
        match self {
            Self::Prudent => Some(30),
            Self::Optimist | Self::Reckless | Self::Story => None,
        }
    }
}

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

/// Plays a bot with every plan, on every seed, at a difficulty, and sums it up. The order is
/// the plans', then the seeds' : the report does not depend on anything else.
#[must_use]
pub fn run_bot(
    bot: Bot,
    plans: &[Plan],
    difficulty: Difficulty,
    seeds: &RangeInclusive<u64>,
) -> (CampaignReport, Vec<GameRecord>) {
    let mut records = Vec::new();
    for plan in plans {
        for seed in seeds.clone() {
            records.push(play_game(
                plan,
                difficulty,
                seed,
                bot.tactic(),
                bot.cools_from(),
            ));
        }
    }
    let report = CampaignReport::of(
        bot.name(),
        difficulty_name(difficulty),
        [*seeds.start(), *seeds.end()],
        plans.len() as u64,
        &records,
    );
    (report, records)
}
