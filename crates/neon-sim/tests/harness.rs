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
use neon_sim::campaign::{PlanSet, plans, run_optimist};
use neon_sim::record::Outcome;
use neon_sim::report::{Report, SCHEMA_VERSION, SitesReport};
use neon_sim::sites::{SiteResult, measure};
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
    let rows = measure(&content, None, None, 2_000_000);
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
        2_000_000,
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].site, "localhost");
    assert_eq!(rows[0].difficulty, "normal");
    assert!(measure(&content, Some("nowhere"), None, 10).is_empty());
}

#[test]
fn the_shipped_game_passes_the_barriers_on_the_sites() {
    let content = Content::embedded().unwrap();
    let rows = measure(&content, None, None, 2_000_000);
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
    let (report, records) = run_optimist(first, Difficulty::Story, &(3..=3));
    let (again, _) = run_optimist(first, Difficulty::Story, &(3..=3));
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
    let (report, _) = run_optimist(&plans[..1], Difficulty::Story, &(1..=1));
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
