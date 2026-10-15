//! The timings of the harness (T13): how long a campaign takes, how long the bounded resolution
//! of a site takes in the worst case, and whether two runs give the same bytes.
//!
//! This is the only place where a clock is read. The canonical reports never contain a duration
//! (they would differ from one machine to the next): the timings are written apart, in
//! `bench.json`, and are not compared byte for byte.

use std::time::Instant;

use neon_engine::campaign::Difficulty;
use neon_engine::content::{Content, Stock};
use neon_engine::run::solve::auto_resolve;
use serde::{Deserialize, Serialize};

use crate::brain::Plan;
use crate::campaign::{Bot, run_bot};
use crate::driver::Tactic;
use crate::record::play_game;
use crate::report::Report;
use crate::sites::full_loot_mission;

/// The timings, in microseconds (whole numbers: no float in the files of the harness).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchReport {
    /// The campaigns that were timed (the optimist, at Normal, one plan after the other).
    pub games: u64,
    /// The campaigns together.
    pub campaigns_total_micros: u64,
    /// The median campaign.
    pub campaign_median_micros: u64,
    /// The slowest campaign.
    pub campaign_worst_micros: u64,
    /// What a thousand campaigns would take, in milliseconds (the mean campaign, in
    /// microseconds, is that many milliseconds per thousand).
    pub thousand_campaigns_millis: u64,
    /// The bounded resolutions timed: every site at every difficulty, taking every loot with
    /// the whole deck of its tier, as an automatic intrusion does.
    pub resolutions: u64,
    /// The slowest of them.
    pub resolution_worst_micros: u64,
    /// The site and difficulty of the slowest.
    pub resolution_worst_at: String,
    /// Whether two runs of the same games gave the same bytes of report.
    pub deterministic: bool,
}

fn micros(since: Instant) -> u64 {
    u64::try_from(since.elapsed().as_micros()).unwrap_or(u64::MAX)
}

/// Times `games` campaigns (the plans in turn), every bounded resolution, and a double run.
#[must_use]
pub fn bench(content: &Content, plans: &[Plan], games: usize) -> BenchReport {
    let mut times: Vec<u64> = Vec::new();
    if !plans.is_empty() {
        for index in 0..games {
            let plan = plans.get(index % plans.len());
            let Some(plan) = plan else { continue };
            let start = Instant::now();
            let _ = play_game(
                plan,
                Difficulty::Normal,
                1,
                Tactic::Solver,
                None,
                None,
                false,
            );
            times.push(micros(start));
        }
    }
    times.sort_unstable();
    let total: u64 = times.iter().sum();
    let count = times.len() as u64;
    let median = times.get(times.len() / 2).copied().unwrap_or(0);
    let worst = times.last().copied().unwrap_or(0);

    let (resolutions, resolution_worst_micros, resolution_worst_at) = time_resolutions(content);

    BenchReport {
        games: count,
        campaigns_total_micros: total,
        campaign_median_micros: median,
        campaign_worst_micros: worst,
        thousand_campaigns_millis: total.checked_div(count).unwrap_or(0),
        resolutions,
        resolution_worst_micros,
        resolution_worst_at,
        deterministic: same_bytes_twice(plans),
    }
}

/// The count of resolutions, the slowest one and where it was.
fn time_resolutions(content: &Content) -> (u64, u64, String) {
    let (mut count, mut worst, mut at) = (0_u64, 0_u64, String::new());
    for site in &content.sites {
        for level in Difficulty::ALL {
            let Some(mission) =
                full_loot_mission(content, &site.id, level.pace(), Stock::Everything)
            else {
                continue;
            };
            let start = Instant::now();
            let _ = auto_resolve(&mission, &mission.initial_state());
            let took = micros(start);
            count += 1;
            if took > worst {
                worst = took;
                at = format!("{} at {}", site.id, crate::record::difficulty_name(level));
            }
        }
    }
    (count, worst, at)
}

/// Whether the same games, played twice, write the same report to the byte.
fn same_bytes_twice(plans: &[Plan]) -> bool {
    let few: Vec<Plan> = plans.iter().take(4).cloned().collect();
    let play = || {
        let (report, _) = run_bot(Bot::Optimist, &few, Difficulty::Normal, &(1..=1));
        Report::Campaign(report).to_json().ok()
    };
    let (first, second) = (play(), play());
    first.is_some() && first == second
}
