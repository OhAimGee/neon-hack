//! The reports: what a batch of games or a measure of the sites says, as canonical JSON.
//!
//! A report holds no clock and no float: the same inputs give the same bytes on every machine
//! (invariant I5), so two reports can be compared with `diff`. Shares are per-mille integers.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::record::{GameRecord, Outcome};
use crate::sites::SiteRow;

/// The version of the report format.
pub const SCHEMA_VERSION: u32 = 1;
/// The games of a campaign report that are listed by name when they went wrong.
const WORST: usize = 20;

/// The spread of a number over the games: the least, the tenth percentile, the median, the
/// ninetieth percentile and the most (nearest rank).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spread {
    /// The least.
    pub min: u64,
    /// The tenth percentile.
    pub p10: u64,
    /// The median.
    pub median: u64,
    /// The ninetieth percentile.
    pub p90: u64,
    /// The most.
    pub max: u64,
}

impl Spread {
    /// The spread of some values, or `None` for none.
    #[must_use]
    pub fn of(mut values: Vec<u64>) -> Option<Self> {
        values.sort_unstable();
        let last = values.len().checked_sub(1)?;
        let at = |percent: usize| values.get(last * percent / 100).copied();
        Some(Self {
            min: at(0)?,
            p10: at(10)?,
            median: at(50)?,
            p90: at(90)?,
            max: at(100)?,
        })
    }
}

/// A game worth replaying.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worst {
    /// The seed, to replay it.
    pub seed: u64,
    /// The plan.
    pub plan: String,
    /// What went wrong.
    pub why: String,
}

/// What the intrusions of the games say.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrusionStats {
    /// All the intrusions played.
    pub total: u64,
    /// The intrusions per finished game.
    pub per_game: Option<Spread>,
    /// The share of intrusions that burned out, per mille.
    pub burned_per_mille: u64,
    /// The turns of an intrusion.
    pub turns: Option<Spread>,
}

/// The report of a batch of games of one bot at one difficulty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignReport {
    /// The version of the format.
    pub schema_version: u32,
    /// The bot.
    pub bot: String,
    /// The difficulty, by its name.
    pub difficulty: String,
    /// The first and last seeds.
    pub seeds: [u64; 2],
    /// The plans each seed was played with.
    pub plans: u64,
    /// The games played.
    pub games: u64,
    /// The games that reached the epilogue.
    pub finished: u64,
    /// The games the bot could not go on with.
    pub stuck: u64,
    /// The endings reached, by name.
    pub endings: BTreeMap<String, u64>,
    /// The command lines of a finished game.
    pub commands: Option<Spread>,
    /// The credits announced as rewards in a finished game.
    pub credits: Option<Spread>,
    /// The highest Notoriety of a finished game.
    pub max_notoriety: Option<Spread>,
    /// The intrusions.
    pub intrusions: IntrusionStats,
    /// The games that went wrong, or lasted the longest.
    pub worst: Vec<Worst>,
}

impl CampaignReport {
    /// Sums up the records of a batch.
    #[must_use]
    pub fn of(
        bot: &str,
        difficulty: &str,
        seeds: [u64; 2],
        plans: u64,
        records: &[GameRecord],
    ) -> Self {
        let finished: Vec<&GameRecord> = records
            .iter()
            .filter(|r| matches!(r.outcome, Outcome::Finished { .. }))
            .collect();
        let mut endings: BTreeMap<String, u64> = BTreeMap::new();
        for record in &finished {
            if let Outcome::Finished { ending } = &record.outcome {
                *endings.entry(ending.clone()).or_insert(0) += 1;
            }
        }
        let intrusions: Vec<_> = finished.iter().flat_map(|r| &r.intrusions).collect();
        let burned = intrusions.iter().filter(|i| i.burned).count() as u64;
        let total = intrusions.len() as u64;
        let mut worst: Vec<Worst> = records
            .iter()
            .filter_map(|r| match &r.outcome {
                Outcome::Stuck { why } => Some(Worst {
                    seed: r.seed,
                    plan: r.plan.clone(),
                    why: why.clone(),
                }),
                Outcome::Finished { .. } => None,
            })
            .collect();
        worst.truncate(WORST);
        Self {
            schema_version: SCHEMA_VERSION,
            bot: bot.to_owned(),
            difficulty: difficulty.to_owned(),
            seeds,
            plans,
            games: records.len() as u64,
            finished: finished.len() as u64,
            stuck: (records.len() - finished.len()) as u64,
            endings,
            commands: Spread::of(finished.iter().map(|r| r.commands).collect()),
            credits: Spread::of(
                finished
                    .iter()
                    .map(|r| u64::try_from(r.credits).unwrap_or(0))
                    .collect(),
            ),
            max_notoriety: Spread::of(
                finished
                    .iter()
                    .map(|r| u64::from(r.max_notoriety))
                    .collect(),
            ),
            intrusions: IntrusionStats {
                total,
                per_game: Spread::of(finished.iter().map(|r| r.intrusions.len() as u64).collect()),
                burned_per_mille: (burned * 1000).checked_div(total).unwrap_or(0),
                turns: Spread::of(intrusions.iter().map(|i| u64::from(i.turns)).collect()),
            },
            worst,
        }
    }
}

/// The report of the sites measured with the solver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SitesReport {
    /// The version of the format.
    pub schema_version: u32,
    /// The expansions the solver could spend on each measure.
    pub expansions: u64,
    /// One row per site and difficulty.
    pub rows: Vec<SiteRow>,
}

/// A report, whichever kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(
    clippy::large_enum_variant,
    reason = "a report is built once and read once: boxing the campaign report would only add noise"
)]
pub enum Report {
    /// A batch of games.
    Campaign(CampaignReport),
    /// The sites measured.
    Sites(SitesReport),
}

impl Report {
    /// The report as JSON: indented, keys in a fixed order, one line feed at the end.
    ///
    /// # Errors
    ///
    /// The serialization error (it cannot happen for these types).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }

    /// Reads a report from its JSON.
    ///
    /// # Errors
    ///
    /// The parse error.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "test code: a failed expectation is the failure to report"
    )]

    use super::*;

    #[test]
    fn the_spread_is_by_nearest_rank() {
        assert_eq!(Spread::of(Vec::new()), None);
        let one = Spread::of(vec![7]).unwrap();
        assert_eq!((one.min, one.median, one.max), (7, 7, 7));
        let ten = Spread::of((1..=10).rev().collect()).unwrap();
        assert_eq!(
            (ten.min, ten.p10, ten.median, ten.p90, ten.max),
            (1, 1, 5, 9, 10)
        );
    }

    #[test]
    fn a_report_survives_its_json() {
        let report = Report::Campaign(CampaignReport::of("optimist", "normal", [1, 2], 3, &[]));
        let json = report.to_json().unwrap();
        assert!(json.ends_with("}\n"), "{json:?}");
        assert_eq!(Report::from_json(&json).unwrap(), report);
        assert!(json.contains("\"kind\": \"campaign\""));
    }

    #[test]
    fn a_report_of_no_game_is_empty_and_does_not_divide_by_zero() {
        let report = CampaignReport::of("optimist", "normal", [1, 1], 0, &[]);
        assert_eq!((report.games, report.finished, report.stuck), (0, 0, 0));
        assert_eq!(report.intrusions.burned_per_mille, 0);
        assert_eq!(report.commands, None);
    }
}
