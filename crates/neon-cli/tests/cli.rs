//! End-to-end checks of the compiled `neon-hack` binary.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str]) -> Output {
    run_with_input(args, "")
}

/// Runs the binary with the given text on its standard input (so, as on a pipe).
fn run_with_input(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_neon-hack"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start neon-hack");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(input.as_bytes())
        .expect("failed to write the input");
    child
        .wait_with_output()
        .expect("failed to wait for neon-hack")
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
fn help_flag_describes_the_game_and_its_options() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("cyberpunk text RPG"));
    for flag in [
        "--demo",
        "--plain",
        "--screen-reader",
        "--ascii",
        "--lang",
        "--verbosity",
        "--seed",
    ] {
        assert!(text.contains(flag), "{flag} is not documented");
    }
}

#[test]
fn without_the_demo_the_game_says_it_is_not_playable_yet() {
    let output = run(&[]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("not playable yet"));
}

#[test]
fn unknown_flag_is_a_usage_error() {
    let output = run(&["--no-such-flag"]);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn a_demo_session_on_a_pipe_plays_to_the_end_and_exits_cleanly() {
    let output = run_with_input(&["--demo", "--seed", "7"], "\nNeon\ny\nscan\nquit\ny\n");
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(
        text.contains("> scan\n"),
        "lines read are echoed on a pipe:\n{text}"
    );
    assert!(text.contains("[Reward] +"));
    assert!(text.ends_with("Goodbye, Neon.\n"), "{text:?}");
    assert!(!text.contains('\u{1b}'), "no escape sequence on a pipe");
    assert!(output.stderr.is_empty());
}

#[test]
fn closing_the_input_ends_the_demo_instead_of_looping() {
    let output = run_with_input(&["--demo"], "");
    assert!(output.status.success());
    assert!(stdout(&output).ends_with("Input closed: ending the session.\n"));
}

#[test]
fn the_same_seed_gives_the_same_bytes_and_another_seed_does_not() {
    let script = "\nNeon\ny\nscan\nscan\nscan\nscan\nquit\ny\n";
    let play = |seed: &str| stdout(&run_with_input(&["--demo", "--seed", seed], script));
    assert_eq!(play("11"), play("11"));
    assert_ne!(play("11"), play("12"));
}

#[test]
fn the_language_and_the_reading_mode_are_options() {
    let script = "\nNeon\ny\nscan\nquit\nn\nquit\ny\n";
    let french = stdout(&run_with_input(
        &["--demo", "--lang", "fr", "--seed", "1"],
        script,
    ));
    assert!(french.contains("[Gain] +"), "{french}");
    assert!(french.contains("Quitter le réseau ? [o/N] "));

    let reader = stdout(&run_with_input(
        &["--demo", "--screen-reader", "--seed", "1"],
        script,
    ));
    assert!(reader.contains("Trace up by "), "{reader}");
    assert!(reader.contains("ECHO-7: Finally"));
    assert!(!reader.contains("NEON HACK"), "decoration is dropped");

    let ascii = stdout(&run_with_input(
        &["--demo", "--ascii", "--seed", "1"],
        script,
    ));
    assert!(ascii.is_ascii(), "--ascii output must be 7-bit:\n{ascii}");
}

#[test]
fn brief_verbosity_keeps_alerts_and_drops_the_atmosphere() {
    let script = "\nNeon\ny\nscan\nscan\nscan\nscan\nscan\nscan\nscan\nscan\nquit\ny\n";
    let full = stdout(&run_with_input(
        &["--demo", "--verbosity", "full", "--seed", "3"],
        script,
    ));
    let brief = stdout(&run_with_input(
        &["--demo", "--verbosity", "brief", "--seed", "3"],
        script,
    ));
    assert!(full.contains("An old cyberdeck boots up on its own."));
    assert!(!brief.contains("An old cyberdeck"), "atmosphere is dropped");
    assert!(!brief.contains("ECHO-7 »"), "dialogue is not essential");
    assert!(
        brief.contains("[ALERT]"),
        "alerts can never be hidden:\n{brief}"
    );
    assert!(brief.contains("[Reward]"));
}
