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

    // The report holds one site: T2 holds, T3 and T4 are objectives, nothing fails.
    let checked = sim(&["check", &file]);
    assert!(checked.status.success(), "{}", out(&checked));
    let text = out(&checked);
    assert!(text.contains("[ok  ] T2 (P)"), "{text}");
    assert!(text.contains("T3 (O)"), "{text}");
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
        out(&ran).contains("1 games, 1 finished, 0 stuck"),
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
