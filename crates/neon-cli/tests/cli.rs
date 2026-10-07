//! End-to-end checks of the compiled `neon-hack` binary.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str]) -> Output {
    run_with_input(args, "")
}

/// Runs the binary with the given text on its standard input (so, as on a pipe).
///
/// A test never touches the real data folder: a demo run without an explicit `--data-dir`
/// gets `--no-save`.
fn run_with_input(args: &[&str], input: &str) -> Output {
    let isolated = args.contains(&"--demo")
        && !args.contains(&"--data-dir")
        && !args.contains(&"--no-save")
        && !args.contains(&"--list-saves");
    let mut child = Command::new(env!("CARGO_BIN_EXE_neon-hack"))
        .args(args)
        .args(isolated.then_some("--no-save"))
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
    assert!(
        ascii.is_ascii(),
        "the English demo with --ascii and an ASCII handle must be ASCII:\n{ascii}"
    );
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
    assert!(!brief.contains("N E O N"), "decoration is dropped too");
    assert!(full.contains("N E O N"));
    assert!(!brief.contains("ECHO-7 »"), "dialogue is not essential");
    assert!(
        brief.contains("[ALERT]"),
        "alerts can never be hidden:\n{brief}"
    );
    assert!(brief.contains("[Reward]"));
}

#[test]
fn ascii_mode_is_seven_bit_in_french_and_when_the_player_types_accents() {
    let script = "\nZoë « 影 »\no\nscan\nquit\no\n";
    for lang in ["en", "fr"] {
        let output = run_with_input(
            &["--demo", "--ascii", "--lang", lang, "--seed", "1"],
            script,
        );
        assert!(output.status.success());
        let text = stdout(&output);
        assert!(text.is_ascii(), "{lang}:\n{text}");
        assert!(
            text.contains("Zoe \" ? \""),
            "{lang}: the handle is kept, in ASCII"
        );
    }
}

#[test]
fn plurals_follow_the_language_in_a_real_session() {
    let play = |lang: &str, seed: &str| {
        stdout(&run_with_input(
            &["--demo", "--lang", lang, "--seed", seed],
            "\nNeon\ny\nscan\nquit\ny\n",
        ))
    };
    let mut found_one_port = false;
    for seed in 1..40 {
        let seed = seed.to_string();
        let (en, fr) = (play("en", &seed), play("fr", &seed));
        // The same seed finds the same number of ports in both languages.
        assert_eq!(
            en.contains("finds 1 open port."),
            fr.contains("trouve 1 port ouvert."),
            "seed {seed}:\n{en}\n{fr}"
        );
        assert!(!en.contains("1 open ports"), "{en}");
        assert!(!fr.contains("1 ports"), "{fr}");
        found_one_port |= en.contains("finds 1 open port.");
    }
    assert!(found_one_port, "some seed finds exactly one port");
}

#[test]
fn screen_reader_and_ascii_can_be_asked_together() {
    let script = "\nZoë\no\nscan\nshop\n0\nquit\no\n";
    let output = run_with_input(
        &[
            "--demo",
            "--screen-reader",
            "--ascii",
            "--lang",
            "fr",
            "--seed",
            "1",
        ],
        script,
    );
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.is_ascii(), "{text}");
    assert!(
        text.contains("Trace en hausse de "),
        "the spoken wording:\n{text}"
    );
    assert!(
        text.contains("\n1. Chaine de proxys (30 credits)\n"),
        "{text}"
    );
    assert!(!text.contains("N E O N"), "no decoration:\n{text}");
}

// ---- Saves ---------------------------------------------------------------------------------

/// Runs the demo with its saves in `dir`.
fn play_in(dir: &std::path::Path, args: &[&str], input: &str) -> Output {
    let dir = dir.to_str().expect("a UTF-8 temporary path");
    let mut all = vec!["--demo", "--data-dir", dir, "--seed", "7"];
    all.extend_from_slice(args);
    run_with_input(&all, input)
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is not UTF-8")
}

const PROLOGUE: &str = "\nNeon\ny\n";

#[test]
fn a_game_is_resumed_where_it_was_left_without_the_prologue() {
    let dir = tempfile::tempdir().unwrap();
    let first = play_in(dir.path(), &[], &format!("{PROLOGUE}scan\nscan\nquit\ny\n"));
    assert!(first.status.success());
    assert!(dir.path().join("saves/auto.toml").exists());

    let second = play_in(dir.path(), &[], "status\nquit\ny\n");
    assert!(second.status.success(), "{}", stderr(&second));
    let text = stdout(&second);
    assert!(text.contains("Resuming your saved game."), "{text}");
    assert!(
        !text.contains("Your handle"),
        "the prologue is not replayed:\n{text}"
    );
    assert!(text.contains("Handle: Neon"), "{text}");
    assert!(
        !text.contains("Trace: 0/"),
        "the trace of the first session is back:\n{text}"
    );
}

#[test]
fn new_starts_over_and_keeps_the_previous_autosave_as_backup() {
    let dir = tempfile::tempdir().unwrap();
    play_in(dir.path(), &[], &format!("{PROLOGUE}scan\nquit\ny\n"));
    let fresh = play_in(
        dir.path(),
        &["--new"],
        &format!("{PROLOGUE}scan\nquit\ny\n"),
    );
    let text = stdout(&fresh);
    assert!(
        text.contains("Your handle"),
        "a new game has its prologue:\n{text}"
    );
    assert!(!text.contains("Resuming"), "{text}");
    assert!(dir.path().join("saves/auto.toml.bak").exists());
}

#[test]
fn saves_can_be_listed_and_loaded_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let empty = play_in(dir.path(), &["--list-saves"], "");
    let text = stdout(&empty);
    assert!(text.starts_with("Saves folder: "), "{text}");
    assert!(text.contains("saves"), "the folder is shown: {text}");
    assert!(text.ends_with("No saved game.\n"), "{text}");

    play_in(
        dir.path(),
        &[],
        &format!("{PROLOGUE}scan\nsave 2\nshop\n0\nscan\nquit\ny\n"),
    );
    let listing = stdout(&play_in(dir.path(), &["--list-saves"], ""));
    assert!(listing.contains("auto: Neon, 5 actions\n"), "{listing}");
    assert!(
        listing.contains("checkpoint-1: Neon, 3 actions\n"),
        "{listing}"
    );
    assert!(listing.contains("slot-2: Neon, 2 actions\n"), "{listing}");
    let french = stdout(&play_in(dir.path(), &["--list-saves", "--lang", "fr"], ""));
    assert!(french.contains("slot-2 : Neon, 2 actions\n"), "{french}");

    // Slot 2 was saved after one scan: loading it is not the end of the game.
    let from_slot = play_in(dir.path(), &["--load", "slot-2"], "status\nquit\ny\n");
    assert!(from_slot.status.success(), "{}", stderr(&from_slot));
    assert!(stdout(&from_slot).contains("Resuming your saved game."));

    // The checkpoint was made on entering the stall: it comes back inside the stall.
    let from_checkpoint = play_in(dir.path(), &["--load", "checkpoint-1"], "0\nquit\ny\n");
    let text = stdout(&from_checkpoint);
    assert!(text.contains("R4Z0R's stall"), "{text}");
}

#[test]
fn asking_for_a_save_that_does_not_exist_is_an_error_that_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let output = play_in(dir.path(), &["--load", "slot-4"], "");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("There is no save called slot-4."),
        "{}",
        stderr(&output)
    );
    assert!(!dir.path().join("saves").exists());

    let bad_name = play_in(dir.path(), &["--load", "slot-12"], "");
    assert_eq!(
        bad_name.status.code(),
        Some(2),
        "a bad name is a usage error"
    );
}

#[test]
fn a_damaged_autosave_is_explained_and_never_overwritten_until_the_player_starts_over() {
    let dir = tempfile::tempdir().unwrap();
    let saves = dir.path().join("saves");
    std::fs::create_dir_all(&saves).unwrap();
    std::fs::write(saves.join("auto.toml"), "garbage").unwrap();

    let refused = play_in(dir.path(), &[], "quit\ny\n");
    assert_eq!(refused.status.code(), Some(1));
    let message = stderr(&refused);
    assert!(
        message.contains("damaged") && message.contains("--new"),
        "{message}"
    );
    assert_eq!(
        std::fs::read_to_string(saves.join("auto.toml")).unwrap(),
        "garbage"
    );

    let started = play_in(
        dir.path(),
        &["--new"],
        &format!("{PROLOGUE}scan\nquit\ny\n"),
    );
    assert!(started.status.success(), "{}", stderr(&started));
    assert_eq!(
        std::fs::read_to_string(saves.join("auto.toml.corrupt")).unwrap(),
        "garbage",
        "the damaged file is set aside, not destroyed"
    );
}

#[test]
fn a_damaged_latest_save_resumes_from_the_previous_copy_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    play_in(dir.path(), &[], &format!("{PROLOGUE}scan\nscan\nquit\ny\n"));
    let auto = dir.path().join("saves/auto.toml");
    let text = std::fs::read_to_string(&auto).unwrap();
    std::fs::write(&auto, &text[..text.len() / 2]).unwrap();

    let output = play_in(dir.path(), &[], "quit\ny\n");
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("resuming from the previous copy"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn no_save_writes_nothing_and_a_manual_save_says_it_is_off() {
    let dir = tempfile::tempdir().unwrap();
    let output = play_in(
        dir.path(),
        &["--no-save"],
        &format!("{PROLOGUE}scan\nsave\nquit\ny\n"),
    );
    assert!(output.status.success());
    assert!(
        stdout(&output).contains("Saving is turned off (--no-save)."),
        "{}",
        stdout(&output)
    );
    assert!(!dir.path().join("saves").exists());
}

#[test]
fn save_options_need_the_demo() {
    for flag in ["--new", "--list-saves", "--no-save"] {
        assert_eq!(run(&[flag]).status.code(), Some(2), "{flag}");
    }
}

#[test]
fn save_texts_follow_the_language_and_stay_ascii_with_ascii() {
    let dir = tempfile::tempdir().unwrap();
    let output = play_in(
        dir.path(),
        &["--lang", "fr", "--ascii"],
        "\nZoë\no\nsave 3\nsave 99\nquit\no\n",
    );
    let text = stdout(&output);
    assert!(text.is_ascii(), "{text}");
    assert!(
        text.contains("Partie sauvegardee dans l'emplacement 3."),
        "{text}"
    );
    assert!(text.contains("Les emplacements vont de 1 a 9."), "{text}");
    let resumed = play_in(dir.path(), &["--lang", "fr", "--ascii"], "quit\no\n");
    assert!(
        stdout(&resumed).contains("Reprise de votre partie sauvegardee."),
        "{}",
        stdout(&resumed)
    );
}
