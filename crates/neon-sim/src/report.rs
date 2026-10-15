//! The reports: what a batch of games or a measure of the sites says, as canonical JSON.
//!
//! A report holds no clock and no float: the same inputs give the same bytes on every machine
//! (invariant I5), so two reports can be compared with `diff`. Shares are per-mille integers.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::economy::EconomyReport;
use crate::monkey::{Fault, MonkeyRecord};
use crate::record::{GameRecord, Outcome};
use crate::sites::SiteRow;

/// The version of the report format.
pub const SCHEMA_VERSION: u32 = 5;
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
    /// The share of intrusions the game played (`skip`), per mille.
    pub automatic_per_mille: u64,
    /// The turns of an intrusion.
    pub turns: Option<Spread>,
    /// The most burned runs in a row on the same site, in any game.
    pub longest_burn_streak: u64,
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
    /// The games lost in Hardcore.
    pub lost: u64,
    /// The games the bot gave up.
    pub gave_up: u64,
    /// The games a hoarder could not finish: a quest asks it to own an item it will not buy.
    pub blocked: u64,
    /// The games the harness could not go on with: a bug of the walk, never a result.
    pub stuck: u64,
    /// The endings reached, by name.
    pub endings: BTreeMap<String, u64>,
    /// The command lines of a finished game.
    pub commands: Option<Spread>,
    /// The credits announced as rewards in a finished game.
    pub credits: Option<Spread>,
    /// The reputation points announced as rewards in a finished game.
    pub reputation: Option<Spread>,
    /// The highest Notoriety of a game, whatever its end.
    pub max_notoriety: Option<Spread>,
    /// The games that reached the Hunted band, per mille of the games.
    pub hunted_per_mille: u64,
    /// The share of the steps of a game spent inside an intrusion, per mille.
    pub in_intrusion: Option<Spread>,
    /// The intrusions.
    pub intrusions: IntrusionStats,
    /// The lines a farmer sent again in a game; absent for the other bots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeated: Option<Spread>,
    /// The games that went wrong, or lasted the longest.
    pub worst: Vec<Worst>,
}

/// The Notoriety from which the market is closed: the Hunted band.
const HUNTED_AT: u8 = 80;

/// The most burned runs in a row on one site in a game: the retries of a burned intrusion
/// follow one another, so a streak is a run of burned records on the same site.
fn burn_streak(record: &GameRecord) -> u64 {
    let mut longest = 0;
    let mut current = 0;
    let mut site: Option<&str> = None;
    for intrusion in &record.intrusions {
        if intrusion.burned && site == Some(intrusion.site.as_str()) {
            current += 1;
        } else if intrusion.burned {
            current = 1;
        } else {
            current = 0;
        }
        site = Some(intrusion.site.as_str());
        longest = longest.max(current);
    }
    longest
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
        let count = |wanted: fn(&Outcome) -> bool| {
            records.iter().filter(|r| wanted(&r.outcome)).count() as u64
        };
        let lost = count(|o| matches!(o, Outcome::Lost { .. }));
        let gave_up = count(|o| matches!(o, Outcome::GaveUp { .. }));
        let blocked = count(|o| matches!(o, Outcome::Blocked { .. }));
        let stuck = count(|o| matches!(o, Outcome::Stuck { .. }));
        let mut endings: BTreeMap<String, u64> = BTreeMap::new();
        for record in &finished {
            if let Outcome::Finished { ending } = &record.outcome {
                *endings.entry(ending.clone()).or_insert(0) += 1;
            }
        }
        // What happened in the intrusions, and how the Notoriety went, counts for every game,
        // whatever its end: a bot that burns out is what these numbers are about.
        let intrusions: Vec<_> = records.iter().flat_map(|r| &r.intrusions).collect();
        let burned = intrusions.iter().filter(|i| i.burned).count() as u64;
        let automatic = intrusions.iter().filter(|i| i.automatic).count() as u64;
        let total = intrusions.len() as u64;
        let hunted = records
            .iter()
            .filter(|r| r.max_notoriety >= HUNTED_AT)
            .count() as u64;
        let mut worst: Vec<Worst> = records
            .iter()
            .filter_map(|r| {
                let why = match &r.outcome {
                    Outcome::Stuck { why } => why.clone(),
                    Outcome::Lost { site } => format!("lost on {site}"),
                    Outcome::GaveUp { site } => format!("gave up on {site}"),
                    Outcome::Blocked { by } => format!("blocked by {by}"),
                    Outcome::Finished { .. } => return None,
                };
                Some(Worst {
                    seed: r.seed,
                    plan: r.plan.clone(),
                    why,
                })
            })
            .collect();
        worst.truncate(WORST);
        let repeated = Spread::of(records.iter().filter_map(|r| r.repeated).collect());
        Self {
            schema_version: SCHEMA_VERSION,
            bot: bot.to_owned(),
            difficulty: difficulty.to_owned(),
            seeds,
            plans,
            games: records.len() as u64,
            finished: finished.len() as u64,
            lost,
            gave_up,
            blocked,
            stuck,
            endings,
            commands: Spread::of(finished.iter().map(|r| r.commands).collect()),
            credits: Spread::of(
                finished
                    .iter()
                    .map(|r| u64::try_from(r.credits).unwrap_or(0))
                    .collect(),
            ),
            reputation: Spread::of(
                finished
                    .iter()
                    .map(|r| u64::try_from(r.reputation).unwrap_or(0))
                    .collect(),
            ),
            max_notoriety: Spread::of(records.iter().map(|r| u64::from(r.max_notoriety)).collect()),
            hunted_per_mille: (hunted * 1000)
                .checked_div(records.len() as u64)
                .unwrap_or(0),
            in_intrusion: Spread::of(records.iter().map(|r| r.intrusion_per_mille).collect()),
            intrusions: IntrusionStats {
                total,
                per_game: Spread::of(records.iter().map(|r| r.intrusions.len() as u64).collect()),
                burned_per_mille: (burned * 1000).checked_div(total).unwrap_or(0),
                automatic_per_mille: (automatic * 1000).checked_div(total).unwrap_or(0),
                turns: Spread::of(intrusions.iter().map(|i| u64::from(i.turns)).collect()),
                longest_burn_streak: records.iter().map(burn_streak).max().unwrap_or(0),
            },
            repeated,
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

/// A game of the monkey that went wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonkeyWorst {
    /// The seed, to replay it.
    pub seed: u64,
    /// The difficulty, by its name.
    pub difficulty: String,
    /// What went wrong.
    pub fault: Fault,
    /// The last inputs before it, the oldest first.
    pub trail: Vec<String>,
}

/// The report of a batch of games of the monkey.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonkeyReport {
    /// The version of the format.
    pub schema_version: u32,
    /// The first and last seeds.
    pub seeds: [u64; 2],
    /// The most inputs a game was given before the monkey left it.
    pub steps: u64,
    /// The games played.
    pub games: u64,
    /// The inputs sent in all of them.
    pub inputs: u64,
    /// The inputs typed inside an intrusion, per mille of all of them.
    pub intrusion_per_mille: u64,
    /// The games the engine panicked in.
    pub panics: u64,
    /// The games that could not be left.
    pub not_quittable: u64,
    /// The games where a save did not round trip, or a damaged one changed the game.
    pub save_faults: u64,
    /// The games where a step did not say what the game waited for.
    pub contract_faults: u64,
    /// The damaged saves that were refused.
    pub damaged_refused: u64,
    /// The damaged saves that were taken for valid ones.
    pub damaged_accepted: u64,
    /// The games that went wrong, the first ones.
    pub worst: Vec<MonkeyWorst>,
}

impl MonkeyReport {
    /// Sums up the games of a batch.
    #[must_use]
    pub fn of(seeds: [u64; 2], steps: u64, records: &[MonkeyRecord]) -> Self {
        let count = |wanted: fn(&Fault) -> bool| {
            records
                .iter()
                .filter(|r| r.fault.as_ref().is_some_and(wanted))
                .count() as u64
        };
        let inputs: u64 = records.iter().map(|r| r.inputs).sum();
        let in_intrusion: u64 = records.iter().map(|r| r.intrusion_inputs).sum();
        let mut worst: Vec<MonkeyWorst> = records
            .iter()
            .filter_map(|r| {
                Some(MonkeyWorst {
                    seed: r.seed,
                    difficulty: r.difficulty.clone(),
                    fault: r.fault.clone()?,
                    trail: r.trail.clone(),
                })
            })
            .collect();
        worst.truncate(WORST);
        Self {
            schema_version: SCHEMA_VERSION,
            seeds,
            steps,
            games: records.len() as u64,
            inputs,
            intrusion_per_mille: (in_intrusion * 1000).checked_div(inputs).unwrap_or(0),
            panics: count(|f| matches!(f, Fault::Panic { .. })),
            not_quittable: count(|f| matches!(f, Fault::NotQuittable { .. })),
            save_faults: count(|f| matches!(f, Fault::Save { .. })),
            contract_faults: count(|f| matches!(f, Fault::Contract { .. })),
            damaged_refused: records.iter().map(|r| r.damaged_refused).sum(),
            damaged_accepted: records.iter().map(|r| r.damaged_accepted).sum(),
            worst,
        }
    }
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
    /// A batch of games of the monkey.
    Monkey(MonkeyReport),
    /// The economy of the content.
    Economy(EconomyReport),
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
    use crate::record::IntrusionRecord;

    fn intrusion(site: &str, burned: bool) -> IntrusionRecord {
        IntrusionRecord {
            site: site.to_owned(),
            turns: 3,
            trace: 40,
            burned,
            automatic: false,
        }
    }

    fn game(outcome: Outcome, notoriety: u8, intrusions: Vec<IntrusionRecord>) -> GameRecord {
        GameRecord {
            plan: "p".to_owned(),
            difficulty: "normal".to_owned(),
            seed: 1,
            outcome,
            commands: 100,
            credits: 0,
            reputation: 0,
            max_notoriety: notoriety,
            intrusion_per_mille: 250,
            intrusions,
            repeated: None,
        }
    }

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

    #[test]
    fn the_ends_of_a_game_are_counted_apart_and_a_lost_game_still_counts_for_the_bot() {
        let ending = Outcome::Finished {
            ending: "e2".to_owned(),
        };
        let records = [
            game(ending, 20, vec![intrusion("a", false)]),
            game(
                Outcome::Lost {
                    site: "b".to_owned(),
                },
                90,
                vec![intrusion("a", false), intrusion("b", true)],
            ),
            game(
                Outcome::GaveUp {
                    site: "c".to_owned(),
                },
                80,
                vec![
                    intrusion("c", true),
                    intrusion("c", true),
                    intrusion("c", true),
                ],
            ),
            game(
                Outcome::Stuck {
                    why: "a bug".to_owned(),
                },
                10,
                Vec::new(),
            ),
        ];
        let report = CampaignReport::of("prudent", "expert", [1, 1], 4, &records);
        assert_eq!(
            (
                report.games,
                report.finished,
                report.lost,
                report.gave_up,
                report.stuck
            ),
            (4, 1, 1, 1, 1)
        );
        // 6 intrusions, 4 burned: the unfinished games count.
        assert_eq!(report.intrusions.total, 6);
        assert_eq!(report.intrusions.burned_per_mille, 666);
        assert_eq!(report.intrusions.longest_burn_streak, 3);
        // Two of four games reached the Hunted band (80 and more).
        assert_eq!(report.hunted_per_mille, 500);
        let why: Vec<&str> = report.worst.iter().map(|w| w.why.as_str()).collect();
        assert_eq!(why, ["lost on b", "gave up on c", "a bug"]);
    }

    #[test]
    fn a_streak_is_burned_runs_in_a_row_on_the_same_site() {
        let apart = game(
            Outcome::Stuck { why: String::new() },
            0,
            vec![
                intrusion("a", true),
                intrusion("b", true),
                intrusion("a", true),
            ],
        );
        assert_eq!(burn_streak(&apart), 1);
        let retries = game(
            Outcome::Stuck { why: String::new() },
            0,
            vec![
                intrusion("a", true),
                intrusion("a", true),
                intrusion("a", false),
                intrusion("a", true),
            ],
        );
        assert_eq!(burn_streak(&retries), 2);
    }
}
