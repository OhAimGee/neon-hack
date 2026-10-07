//! End-to-end checks of the compiled `neon-hack` binary.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_neon-hack"))
        .args(args)
        .output()
        .expect("failed to start neon-hack")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is not UTF-8")
}

#[test]
fn version_flag_prints_the_crate_version() {
    let output = run(&["--version"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_flag_describes_the_game() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("cyberpunk text RPG"));
}

#[test]
fn plain_run_says_the_game_is_not_playable_yet() {
    let output = run(&[]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("not playable yet"));
}

#[test]
fn unknown_flag_is_a_usage_error() {
    let output = run(&["--no-such-flag"]);
    assert_eq!(output.status.code(), Some(2));
}
