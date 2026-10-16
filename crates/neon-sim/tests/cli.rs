//! The binary: `solve`, `run`, `check` through the command line, on files.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "this crate is test code: a failed expectation is the failure to report"
)]

use std::path::Path;
use std::process::{Command, Output};

fn sim(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_neon-sim"))
        .args(args)
        .output()
        .unwrap()
}

fn out(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn path(dir: &Path, name: &str) -> String {
    dir.join(name).to_string_lossy().into_owned()
}

#[test]
fn solve_writes_a_report_that_check_reads() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(dir.path(), "sites.json");
    let solved = sim(&["solve", "--site", "localhost", "--out", &file]);
    assert!(solved.status.success(), "{}", out(&solved));
    assert!(out(&solved).contains("localhost story"), "{}", out(&solved));
    let json = std::fs::read_to_string(&file).unwrap();
    assert!(json.contains("\"kind\": \"sites\""), "{json}");

    // The report holds the tutorial: it is won, and the order of the difficulties holds. The
    // length, the gap and the bounds start at tier 3.
    let checked = sim(&["check", &file]);
    assert!(checked.status.success(), "{}", out(&checked));
    let text = out(&checked);
    assert!(text.contains("[ok  ] T2 (P)"), "{text}");
    assert!(
        text.contains("[ok  ] T4 (P): the margin never grows"),
        "{text}"
    );
}

#[test]
fn run_plays_a_game_and_writes_a_report_per_difficulty() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(dir.path(), "optimist.json");
    let ran = sim(&[
        "run",
        "--difficulty",
        "story",
        "--seeds",
        "1..=1",
        "--limit",
        "1",
        "--out",
        &file,
    ]);
    assert!(ran.status.success(), "{}", out(&ran));
    assert!(
        out(&ran).contains("1 games, 1 finished, 0 lost, 0 gave up, 0 blocked, 0 stuck"),
        "{}",
        out(&ran)
    );
    let json = std::fs::read_to_string(&file).unwrap();
    assert!(json.contains("\"kind\": \"campaign\""), "{json}");
    assert!(json.contains("\"difficulty\": \"story\""), "{json}");
}

#[test]
fn a_failed_barrier_exits_with_one() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(dir.path(), "optimist.json");
    let ran = sim(&[
        "run",
        "--difficulty",
        "story",
        "--limit",
        "1",
        "--out",
        &file,
    ]);
    assert!(ran.status.success(), "{}", out(&ran));
    // One game cannot reach the five endings: T1 fails.
    let checked = sim(&["check", &file]);
    assert_eq!(checked.status.code(), Some(1), "{}", out(&checked));
    assert!(out(&checked).contains("[FAIL] T1 (P)"), "{}", out(&checked));
}

#[test]
fn bad_arguments_and_files_are_refused_with_two() {
    let dir = tempfile::tempdir().unwrap();
    let missing = path(dir.path(), "nothing.json");
    let checked = sim(&["check", &missing]);
    assert_eq!(checked.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&checked.stderr).contains("neon-sim:"));
    let solved = sim(&["solve", "--site", "nowhere"]);
    assert_eq!(solved.status.code(), Some(2));
    let seeds = sim(&["run", "--seeds", "9..=2"]);
    assert_ne!(seeds.status.code(), Some(0));
}

#[test]
fn a_bot_that_plays_by_hand_is_named_in_its_summary_and_checked_against_its_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(dir.path(), "reckless.json");
    let ran = sim(&[
        "run",
        "--bot",
        "reckless",
        "--difficulty",
        "story",
        "--seeds",
        "1",
        "--limit",
        "1",
        "--out",
        &file,
    ]);
    assert!(ran.status.success(), "{}", out(&ran));
    assert!(
        out(&ran).contains("reckless story: 1 games, 1 finished"),
        "{}",
        out(&ran)
    );
    let json = std::fs::read_to_string(&file).unwrap();
    assert!(json.contains("\"bot\": \"reckless\""), "{json}");
    let checked = sim(&["check", &file]);
    assert!(checked.status.success(), "{}", out(&checked));
    assert!(
        out(&checked).contains("[ok  ] T6 (O): reckless in Story finishes 1000 per mille"),
        "{}",
        out(&checked)
    );
}

#[test]
fn the_story_bot_plays_story_and_is_checked_against_t14() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(dir.path(), "story.json");
    let ran = sim(&[
        "run", "--bot", "story", "--seeds", "1", "--limit", "1", "--out", &file,
    ]);
    assert!(ran.status.success(), "{}", out(&ran));
    assert!(
        out(&ran).contains("story story: 1 games, 1 finished"),
        "{}",
        out(&ran)
    );
    let checked = sim(&["check", &file]);
    assert!(checked.status.success(), "{}", out(&checked));
    assert!(
        out(&checked).contains("[ok  ] T14 (O): story finishes 1000 per mille"),
        "{}",
        out(&checked)
    );
    // It plays one difficulty only.
    let wrong = sim(&[
        "run",
        "--bot",
        "story",
        "--difficulty",
        "normal",
        "--limit",
        "1",
    ]);
    assert_eq!(wrong.status.code(), Some(2), "{}", out(&wrong));
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("only plays story"));
}

#[test]
fn list_worst_names_the_games_to_replay_and_refuses_a_report_without_games() {
    let dir = tempfile::tempdir().unwrap();
    let hoarder = path(dir.path(), "hoarder.json");
    let ran = sim(&[
        "run",
        "--bot",
        "hoarder",
        "--difficulty",
        "normal",
        "--seeds",
        "7",
        "--limit",
        "1",
        "--out",
        &hoarder,
    ]);
    assert!(ran.status.success(), "{}", out(&ran));
    let listed = sim(&["list-worst", "--report", &hoarder]);
    assert!(listed.status.success(), "{}", out(&listed));
    let text = out(&listed);
    assert!(
        text.contains("hoarder at normal: 1 of 1 games did not finish"),
        "{text}"
    );
    assert!(text.contains("  seed 7, plan "), "{text}");
    assert!(text.contains(": blocked by "), "{text}");
    assert!(
        text.contains("replay: neon-sim run --bot hoarder --difficulty normal --seeds SEED"),
        "{text}"
    );
    // A report in which every game finished says so.
    let optimist = path(dir.path(), "optimist.json");
    let ran = sim(&[
        "run",
        "--difficulty",
        "story",
        "--seeds",
        "1",
        "--limit",
        "1",
        "--out",
        &optimist,
    ]);
    assert!(ran.status.success(), "{}", out(&ran));
    let listed = sim(&["list-worst", "--report", &optimist]);
    assert!(listed.status.success(), "{}", out(&listed));
    assert!(
        out(&listed).contains("no game went wrong"),
        "{}",
        out(&listed)
    );
    // Sites hold no games, and the report is required.
    let sites = path(dir.path(), "sites.json");
    let solved = sim(&["solve", "--site", "localhost", "--out", &sites]);
    assert!(solved.status.success(), "{}", out(&solved));
    let refused = sim(&["list-worst", "--report", &sites]);
    assert_eq!(refused.status.code(), Some(2), "{}", out(&refused));
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("holds no games"),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );
    assert_eq!(sim(&["list-worst"]).status.code(), Some(2));
}

#[test]
fn the_monkey_writes_its_report_and_a_short_one_does_not_pass_t12() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(dir.path(), "monkey.json");
    let ran = sim(&[
        "run", "--bot", "monkey", "--seeds", "1..=6", "--steps", "60", "--out", &file,
    ]);
    assert!(ran.status.success(), "{}", out(&ran));
    assert!(out(&ran).contains("monkey: 6 games"), "{}", out(&ran));
    let json = std::fs::read_to_string(&file).unwrap();
    assert!(json.contains("\"kind\": \"monkey\""), "{json}");
    // Nothing broke, but six games say nothing: T12 is a barrier and wants more.
    let checked = sim(&["check", &file]);
    assert_eq!(checked.status.code(), Some(1), "{}", out(&checked));
    let text = out(&checked);
    assert!(
        text.contains("[FAIL] T12 (P): the monkey played 6 games"),
        "{text}"
    );
    assert!(
        text.contains("[ok  ] T12 (P): the monkey found 0 panics"),
        "{text}"
    );
}

#[test]
fn the_farmer_is_checked_against_the_optimist_by_t8() {
    let dir = tempfile::tempdir().unwrap();
    let optimist = path(dir.path(), "optimist.json");
    let farmer = path(dir.path(), "farmer.json");
    for (bot, file) in [("optimist", &optimist), ("farmer", &farmer)] {
        let ran = sim(&[
            "run",
            "--bot",
            bot,
            "--difficulty",
            "story",
            "--seeds",
            "1",
            "--limit",
            "1",
            "--out",
            file,
        ]);
        assert!(ran.status.success(), "{}", out(&ran));
    }
    let json = std::fs::read_to_string(&farmer).unwrap();
    assert!(
        json.contains("\"bot\": \"farmer\"") && json.contains("\"repeated\""),
        "{json}"
    );
    let checked = sim(&["check", &optimist, &farmer]);
    let text = out(&checked);
    assert!(
        text.contains("[ok  ] T8 (P): at story the farmer sent again"),
        "{text}"
    );
}

#[test]
fn the_economy_and_the_hoarder_are_reported_and_read_by_t7() {
    let dir = tempfile::tempdir().unwrap();
    let economy = path(dir.path(), "economy.json");
    let hoarder = path(dir.path(), "hoarder.json");
    let told = sim(&["economy", "--out", &economy]);
    assert!(told.status.success(), "{}", out(&told));
    assert!(
        out(&told).contains("the stall: 17 items for 24660 credits"),
        "{}",
        out(&told)
    );
    assert!(
        std::fs::read_to_string(&economy)
            .unwrap()
            .contains("\"kind\": \"economy\"")
    );
    let ran = sim(&[
        "run",
        "--bot",
        "hoarder",
        "--difficulty",
        "normal",
        "--seeds",
        "1",
        "--limit",
        "1",
        "--out",
        &hoarder,
    ]);
    assert!(ran.status.success(), "{}", out(&ran));
    assert!(out(&ran).contains("1 blocked"), "{}", out(&ran));
    // Only objectives: the check goes on, and says what it found.
    let checked = sim(&["check", &economy, &hoarder]);
    assert!(checked.status.success(), "{}", out(&checked));
    let text = out(&checked);
    assert!(
        text.contains("[ok  ] T7 (O): the main quests of every tier pay for the item of the next"),
        "{text}"
    );
    assert!(
        text.contains("[ok  ] T7 (O): the hoarder finishes 0 per mille"),
        "{text}"
    );
}

#[test]
fn the_programs_of_a_site_are_measured_and_read_by_t9() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(dir.path(), "programs.json");
    let told = sim(&["programs", "--site", "corp-server-01", "--out", &file]);
    assert!(told.status.success(), "{}", out(&told));
    assert!(out(&told).contains("spoof (80 credits)"), "{}", out(&told));
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("\"kind\": \"programs\"")
    );
    // One site is not the campaign: most programs lower no margin there, and it is only said.
    let checked = sim(&["check", &file]);
    assert!(checked.status.success(), "{}", out(&checked));
    assert!(out(&checked).contains("T9 (O)"), "{}", out(&checked));
}
