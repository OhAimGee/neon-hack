//! The harness against the real game: a game is played and recorded, the sites are measured,
//! the thresholds shipped parse, and the reports say the same thing every time.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "this crate is test code: a failed expectation is the failure to report"
)]

use neon_engine::campaign::Difficulty;
use neon_engine::content::Content;
use neon_sim::campaign::{Bot, PlanSet, plans, run_bot};
use neon_sim::monkey::play_monkey;
use neon_sim::record::Outcome;
use neon_sim::report::{MonkeyReport, Report, SCHEMA_VERSION, SitesReport};
use neon_sim::sites::{Loot, SiteResult, measure};
use neon_sim::targets::{Level, Targets, check};

#[test]
fn the_shipped_thresholds_parse() {
    let targets = Targets::embedded().unwrap();
    assert_eq!(targets.t1.level, Level::P);
    assert_eq!(targets.t1.endings.len(), 5);
    assert!(targets.t3.min_turns < targets.t3.max_turns);
    assert!(targets.t4.normal_min < targets.t4.normal_max);
}

#[test]
fn every_site_is_measured_and_won_at_every_difficulty() {
    let content = Content::embedded().unwrap();
    let rows = measure(&content, None, None, Loot::Objective, 2_000_000);
    assert_eq!(rows.len(), content.sites.len() * Difficulty::ALL.len());
    for row in &rows {
        match &row.result {
            SiteResult::Solved {
                trace,
                margin,
                optimal,
                ..
            } => {
                assert!(*optimal, "{} {}", row.site, row.difficulty);
                assert_eq!(u16::from(*trace) + u16::from(*margin), u16::from(row.cap));
            }
            other => panic!("{} {}: {other:?}", row.site, row.difficulty),
        }
    }
}

#[test]
fn a_site_can_be_measured_alone() {
    let content = Content::embedded().unwrap();
    let rows = measure(
        &content,
        Some("localhost"),
        Some(Difficulty::Normal),
        Loot::Objective,
        2_000_000,
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].site, "localhost");
    assert_eq!(rows[0].difficulty, "normal");
    assert!(measure(&content, Some("nowhere"), None, Loot::Objective, 10).is_empty());
}

#[test]
fn the_shipped_game_passes_the_barriers_on_the_sites() {
    let content = Content::embedded().unwrap();
    let rows = measure(&content, None, None, Loot::Objective, 2_000_000);
    let report = Report::Sites(SitesReport {
        schema_version: SCHEMA_VERSION,
        expansions: 2_000_000,
        rows,
    });
    let findings = check(&Targets::embedded().unwrap(), &[report]);
    assert!(!findings.is_empty());
    for finding in &findings {
        assert!(!finding.fails(), "{}: {}", finding.id, finding.detail);
    }
    // T2 holds, and it is a barrier.
    let t2 = findings.iter().find(|f| f.id == "T2").unwrap();
    assert!(t2.ok && t2.level == Level::P, "{}", t2.detail);
}

#[test]
fn a_game_is_played_to_its_ending_and_the_report_is_the_same_every_time() {
    let plans = plans(PlanSet::Representative).unwrap();
    let first = &plans[..1];
    let (report, records) = run_bot(Bot::Optimist, first, Difficulty::Story, &(3..=3));
    let (again, _) = run_bot(Bot::Optimist, first, Difficulty::Story, &(3..=3));
    assert_eq!(
        Report::Campaign(report.clone()).to_json().unwrap(),
        Report::Campaign(again).to_json().unwrap(),
        "the same inputs give the same bytes"
    );
    assert_eq!((report.games, report.finished, report.stuck), (1, 1, 0));
    let record = &records[0];
    let Outcome::Finished { ending } = &record.outcome else {
        panic!("{:?}", record.outcome);
    };
    assert!(["e1a", "e1b", "e2", "e3", "e4"].contains(&ending.as_str()));
    assert_eq!(report.endings.get(ending), Some(&1));
    assert!(record.commands > 100, "{}", record.commands);
    assert!(record.credits > 0);
    assert!(!record.intrusions.is_empty());
    // Story mode never burns the optimist out.
    assert!(record.intrusions.iter().all(|i| !i.burned));
    assert_eq!(report.seeds, [3, 3]);
}

#[test]
fn a_campaign_report_is_checked_against_the_first_threshold() {
    let plans = plans(PlanSet::Representative).unwrap();
    let (report, _) = run_bot(Bot::Optimist, &plans[..1], Difficulty::Story, &(1..=1));
    let findings = check(&Targets::embedded().unwrap(), &[Report::Campaign(report)]);
    // One game finished: the share is right, and four of the five endings are missing.
    assert_eq!(findings.len(), 2);
    assert!(findings[0].ok, "{}", findings[0].detail);
    assert!(
        !findings[1].ok && findings[1].fails(),
        "{}",
        findings[1].detail
    );
}

#[test]
fn the_whole_loot_of_a_site_can_be_asked_for_and_is_not_written_by_default() {
    let content = Content::embedded().unwrap();
    let all = measure(
        &content,
        Some("corp-server-01"),
        Some(Difficulty::Normal),
        Loot::All,
        2_000_000,
    );
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].loot, Loot::All);
    assert!(matches!(all[0].result, SiteResult::Solved { .. }));
    let json = serde_json::to_string(&all[0]).unwrap();
    assert!(json.contains("\"loot\":\"all\""), "{json}");
    let objective = measure(
        &content,
        Some("corp-server-01"),
        Some(Difficulty::Normal),
        Loot::Objective,
        2_000_000,
    );
    let json = serde_json::to_string(&objective[0]).unwrap();
    assert!(!json.contains("loot"), "{json}");
}

#[test]
fn the_bots_that_play_by_hand_finish_story_and_normal_without_burning() {
    let plans = plans(PlanSet::Representative).unwrap();
    for bot in [Bot::Prudent, Bot::Reckless] {
        for difficulty in [Difficulty::Story, Difficulty::Normal] {
            let (report, _) = run_bot(bot, &plans[..1], difficulty, &(1..=1));
            assert_eq!(report.bot, bot.name());
            assert_eq!(
                (report.finished, report.lost, report.gave_up, report.stuck),
                (1, 0, 0, 0),
                "{} {difficulty:?}",
                bot.name()
            );
            assert_eq!(report.intrusions.burned_per_mille, 0);
        }
    }
}

#[test]
fn a_game_burned_out_in_hardcore_is_lost_and_recorded_as_such() {
    let plans = plans(PlanSet::Representative).unwrap();
    let (report, records) = run_bot(Bot::Reckless, &plans[..1], Difficulty::Hardcore, &(1..=1));
    assert_eq!((report.finished, report.lost, report.stuck), (0, 1, 0));
    let Outcome::Lost { site } = &records[0].outcome else {
        panic!("{:?}", records[0].outcome);
    };
    // The burned intrusion is the last one, on the site the game was lost on.
    let last = records[0].intrusions.last().unwrap();
    assert!(last.burned);
    assert_eq!(&last.site, site);
    assert!(
        report.worst[0].why.starts_with("lost on "),
        "{:?}",
        report.worst
    );
}

#[test]
fn a_bot_that_burns_the_same_intrusion_again_and_again_gives_up() {
    let plans = plans(PlanSet::Representative).unwrap();
    let (report, records) = run_bot(Bot::Prudent, &plans[..1], Difficulty::Expert, &(1..=1));
    assert_eq!((report.finished, report.gave_up, report.stuck), (0, 1, 0));
    assert!(matches!(records[0].outcome, Outcome::GaveUp { .. }));
    // A dozen runs in a row burned on one site: the streak is what T5 reads.
    assert!(
        report.intrusions.longest_burn_streak >= 12,
        "{:?}",
        report.intrusions
    );
}

#[test]
fn the_bots_are_checked_against_the_thresholds_of_the_spec() {
    let plans = plans(PlanSet::Representative).unwrap();
    let mut reports = Vec::new();
    for (bot, difficulty) in [
        (Bot::Prudent, Difficulty::Normal),
        (Bot::Reckless, Difficulty::Normal),
        (Bot::Reckless, Difficulty::Story),
        (Bot::Reckless, Difficulty::Expert),
    ] {
        let (report, _) = run_bot(bot, &plans[..1], difficulty, &(1..=1));
        reports.push(Report::Campaign(report));
    }
    let findings = check(&Targets::embedded().unwrap(), &reports);
    let ids: Vec<&str> = findings.iter().map(|f| f.id).collect();
    for id in ["T5", "T6", "T10", "T11"] {
        assert!(ids.contains(&id), "{id} is missing from {ids:?}");
    }
    // The reckless floor holds: Story is finished, Expert is not.
    let t6: Vec<_> = findings.iter().filter(|f| f.id == "T6").collect();
    assert_eq!(t6.len(), 2);
    assert!(t6.iter().all(|f| f.ok), "{t6:?}");
    // None of them is a barrier yet: the numbers are the owner's to validate.
    assert!(findings.iter().all(|f| f.id == "T1" || !f.fails()));
}

#[test]
fn the_story_bot_skips_what_lasts_and_still_finishes_the_campaign() {
    let plans = plans(PlanSet::Representative).unwrap();
    let (report, records) = run_bot(Bot::Story, &plans[..1], Difficulty::Story, &(1..=1));
    assert_eq!(report.bot, "story");
    assert_eq!(
        (report.finished, report.gave_up, report.stuck),
        (1, 0, 0),
        "{:?}",
        report.worst
    );
    // Some intrusions are played by the game (`skip`), the ones that cost a single turn and the
    // visits back to a site `skip` left something at are played by the bot.
    let record = &records[0];
    assert!(record.intrusions.iter().any(|i| i.automatic));
    assert!(record.intrusions.iter().any(|i| !i.automatic));
    assert!(report.intrusions.automatic_per_mille > 0);
    assert_eq!(Bot::Story.only_difficulty(), Some(Difficulty::Story));
    let findings = check(&Targets::embedded().unwrap(), &[Report::Campaign(report)]);
    let t14 = findings.iter().find(|f| f.id == "T14").unwrap();
    assert!(t14.ok && !t14.fails(), "{}", t14.detail);
}

#[test]
fn the_monkey_breaks_nothing_and_is_the_same_game_every_time() {
    let games = |seeds: std::ops::RangeInclusive<u64>| {
        seeds
            .map(|seed| {
                let difficulty = Difficulty::ALL[usize::try_from(seed % 4).unwrap()];
                play_monkey(seed, difficulty, 120)
            })
            .collect::<Vec<_>>()
    };
    let records = games(1..=8);
    let report = MonkeyReport::of([1, 8], 120, &records);
    assert_eq!(
        (
            report.panics,
            report.not_quittable,
            report.save_faults,
            report.contract_faults
        ),
        (0, 0, 0, 0),
        "{:?}",
        report.worst
    );
    assert!(report.inputs > 8 * 20, "{}", report.inputs);
    // Damaged saves are tried, and nearly all of them are refused.
    assert!(report.damaged_refused > 0);
    let again = MonkeyReport::of([1, 8], 120, &games(1..=8));
    assert_eq!(
        Report::Monkey(report).to_json().unwrap(),
        Report::Monkey(again).to_json().unwrap(),
        "the same seeds give the same bytes"
    );
}
