//! End-to-end checks of the compiled `neon-hack` binary.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str]) -> Output {
    run_with_input(args, "")
}

/// Runs the binary with the given text on its standard input (so, as on a pipe).
fn run_with_input(args: &[&str], input: &str) -> Output {
    run_in_env(args, input, &[])
}

/// Runs the binary with **only** the environment variables given, so that the test does not
/// depend on the machine's locale, `NO_COLOR` or settings.
///
/// A test never touches the real data folder: without an explicit `--data-dir` (or
/// `NEON_HACK_DATA_DIR`), the run gets a folder of its own, and a run that plays is also given
/// `--no-save`.
fn run_in_env(args: &[&str], input: &str, vars: &[(&str, &str)]) -> Output {
    let scratch = tempfile::tempdir().expect("a temporary folder");
    let mut command = Command::new(env!("CARGO_BIN_EXE_neon-hack"));
    command.env_clear().envs(vars.iter().copied());
    #[cfg(windows)]
    if let Some(root) = std::env::var_os("SystemRoot") {
        command.env("SystemRoot", root);
    }
    command.args(args);
    let has_folder =
        args.contains(&"--data-dir") || vars.iter().any(|(name, _)| *name == "NEON_HACK_DATA_DIR");
    if !has_folder {
        command.arg("--data-dir").arg(scratch.path());
        // Any run that plays (the campaign by default, or the demo) writes no save.
        let informs = [
            "--help",
            "-h",
            "--version",
            "-V",
            "--print-settings",
            "--list-saves",
        ];
        if !args
            .iter()
            .any(|arg| informs.contains(arg) || *arg == "--no-save")
        {
            command.arg("--no-save");
        }
    }
    let mut child = command
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
    // The campaign is what runs by default; the demo is the option that says so.
    assert!(text.contains("plays the campaign"), "{text}");
    assert!(
        text.contains("--demo") && text.contains("toy game"),
        "{text}"
    );
    assert!(text.contains("--difficulty"), "{text}");
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
fn without_a_game_option_the_campaign_is_played() {
    // The input closes at the handle: the session ends, but it was the campaign's.
    let output = run(&[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("N E O N"), "{text}");
    assert!(text.contains("Handle [Neon]: "), "{text}");
    assert!(!text.contains("not playable"), "{text}");
    assert!(
        text.ends_with("Input closed: ending the session.\n"),
        "{text:?}"
    );
    // The option that used to ask for it is still understood, and says nothing more.
    let explicit = run(&["--campaign"]);
    assert_eq!(stdout(&explicit), text);
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
fn save_options_work_for_either_game_without_naming_one() {
    // They used to need a game option; the campaign is the game when none is named.
    let listing = run(&["--list-saves"]);
    assert!(listing.status.success(), "{}", stderr(&listing));
    assert!(
        stdout(&listing).contains("saves-campaign"),
        "{}",
        stdout(&listing)
    );
    for flag in ["--new", "--no-save"] {
        let output = run_with_input(&[flag], "");
        assert!(output.status.success(), "{flag}: {}", stderr(&output));
    }
    let output = run(&["--load", "slot-1"]);
    assert_eq!(output.status.code(), Some(1), "no such save is an error");
    assert!(
        stderr(&output).contains("There is no save called slot-1."),
        "{}",
        stderr(&output)
    );
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

// ---- Settings ------------------------------------------------------------------------------

fn settings_in(dir: &std::path::Path, content: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("settings.toml"), content).unwrap();
}

fn dir_arg(dir: &tempfile::TempDir) -> &str {
    dir.path().to_str().expect("a UTF-8 temporary path")
}

#[test]
fn options_are_grouped_in_the_three_families() {
    let text = stdout(&run(&["--help"]));
    for heading in ["Presentation:", "Game:", "Development:"] {
        assert!(text.contains(heading), "{heading} is missing:\n{text}");
    }
}

#[test]
fn print_settings_shows_each_value_and_where_it_comes_from() {
    let dir = tempfile::tempdir().unwrap();
    let args = ["--print-settings", "--data-dir", dir_arg(&dir)];
    let nothing = stdout(&run_in_env(&args, "", &[]));
    assert!(nothing.contains("lang = \"en\"  # default"), "{nothing}");
    assert!(
        nothing.contains("verbosity = \"normal\"  # default"),
        "{nothing}"
    );
    assert!(nothing.contains("ascii = false  # default"), "{nothing}");
    assert!(
        nothing.contains("screen_reader = false  # default"),
        "{nothing}"
    );
    assert!(
        nothing.contains("# command line"),
        "the data folder: {nothing}"
    );
    assert!(nothing.contains("settings.toml\"  # missing"), "{nothing}");

    let french = stdout(&run_in_env(&args, "", &[("LANG", "fr_FR.UTF-8")]));
    assert!(french.contains("lang = \"fr\"  # locale"), "{french}");
}

#[test]
fn the_most_specific_source_wins_for_each_setting() {
    let dir = tempfile::tempdir().unwrap();
    settings_in(dir.path(), "lang = \"en\"\nverbosity = \"full\"\n");
    let shown = |extra: &[&str], vars: &[(&str, &str)]| {
        let mut args = vec!["--print-settings", "--data-dir", dir_arg(&dir)];
        args.extend_from_slice(extra);
        stdout(&run_in_env(&args, "", vars))
    };
    let vars = [("NEON_HACK_LANG", "fr"), ("LANG", "en_US.UTF-8")];
    assert!(shown(&[], &vars).contains("lang = \"fr\"  # NEON_HACK_LANG"));
    assert!(shown(&["--lang", "en"], &vars).contains("lang = \"en\"  # command line"));
    assert!(shown(&[], &[("LANG", "fr")]).contains("lang = \"en\"  # settings.toml"));
    let verbosity = shown(&["--verbosity", "brief"], &[]);
    assert!(
        verbosity.contains("verbosity = \"brief\"  # command line"),
        "{verbosity}"
    );
    assert!(shown(&[], &[]).contains("verbosity = \"full\"  # settings.toml"));
}

#[test]
fn the_settings_file_shapes_the_game_and_the_command_line_overrides_it() {
    let dir = tempfile::tempdir().unwrap();
    settings_in(dir.path(), "lang = \"fr\"\nverbosity = \"brief\"\n");
    let script = format!("{PROLOGUE}scan\nquit\no\n");
    let play = |extra: &[&str]| {
        let mut args = vec![
            "--demo",
            "--no-save",
            "--data-dir",
            dir_arg(&dir),
            "--seed",
            "3",
        ];
        args.extend_from_slice(extra);
        stdout(&run_in_env(&args, &script, &[]))
    };
    let from_file = play(&[]);
    assert!(
        from_file.contains("[Gain]"),
        "French from the file:\n{from_file}"
    );
    assert!(
        !from_file.contains("Neo-Tokyo"),
        "brief hides the atmosphere:\n{from_file}"
    );
    let overridden = play(&["--lang", "en", "--verbosity", "full"]);
    assert!(overridden.contains("[Reward]"), "{overridden}");
    assert!(overridden.contains("Neo-Tokyo"), "{overridden}");
}

#[test]
fn a_damaged_settings_file_is_reported_used_as_defaults_and_never_rewritten() {
    let dir = tempfile::tempdir().unwrap();
    settings_in(dir.path(), "lang = \"klingon\"\n");
    let output = run_in_env(
        &["--demo", "--no-save", "--data-dir", dir_arg(&dir)],
        &format!("{PROLOGUE}quit\ny\n"),
        &[],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("settings.toml could not be used"), "{text}");
    assert!(text.contains("Your handle"), "the defaults apply:\n{text}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("settings.toml")).unwrap(),
        "lang = \"klingon\"\n"
    );
}

#[test]
fn print_settings_in_ascii_mode_transliterates_paths_and_reasons_too() {
    let dir = tempfile::tempdir().unwrap();
    let accented = dir.path().join("dossier-é");
    settings_in(&accented, "lang = \"élan\"\n");
    let args = [
        "--print-settings",
        "--ascii",
        "--data-dir",
        accented.to_str().unwrap(),
    ];
    let text = stdout(&run_in_env(&args, "", &[]));
    assert!(text.is_ascii(), "{text}");
    assert!(text.contains("dossier-e"), "{text}");
    assert!(text.contains("damaged"), "the bad file is reported: {text}");
}

#[test]
fn the_data_folder_can_come_from_the_environment() {
    let dir = tempfile::tempdir().unwrap();
    let vars = [("NEON_HACK_DATA_DIR", dir_arg(&dir))];
    run_in_env(
        &["--demo", "--seed", "7"],
        &format!("{PROLOGUE}scan\nquit\ny\n"),
        &vars,
    );
    assert!(dir.path().join("saves/auto.toml").exists());
    let shown = stdout(&run_in_env(&["--print-settings"], "", &vars));
    assert!(shown.contains("# NEON_HACK_DATA_DIR"), "{shown}");
}

#[test]
fn a_locale_with_another_charset_gives_ascii_output_by_itself() {
    let script = "\nZoë\no\nscan\nquit\no\n";
    let latin1 = run_in_env(
        &["--demo", "--seed", "1"],
        script,
        &[("LANG", "fr_FR.ISO-8859-1")],
    );
    let text = stdout(&latin1);
    assert!(text.is_ascii(), "{text}");
    assert!(
        text.contains("[Gain]"),
        "the locale also chose French:\n{text}"
    );
    let utf8 = stdout(&run_in_env(
        &["--demo", "--seed", "1"],
        script,
        &[("LANG", "fr_FR.UTF-8")],
    ));
    assert!(!utf8.is_ascii(), "UTF-8 keeps the accents:\n{utf8}");
}

// ---- Colour --------------------------------------------------------------------------------

/// The text with every `ESC [ ... m` sequence removed.
fn strip_sgr(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for end in chars.by_ref() {
                if end == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn colour_session(extra: &[&str], vars: &[(&str, &str)]) -> String {
    let mut args = vec!["--demo", "--seed", "3"];
    args.extend_from_slice(extra);
    stdout(&run_in_env(
        &args,
        "\nNeon\ny\nscan\nscan\nnope\nquit\ny\n",
        vars,
    ))
}

#[test]
fn a_pipe_gets_no_colour_unless_asked_and_asking_changes_nothing_but_the_escapes() {
    let plain = colour_session(&[], &[]);
    assert!(!plain.contains('\u{1b}'), "a pipe has no colour by itself");
    let forced = colour_session(&["--color", "always"], &[]);
    assert!(forced.contains("\u{1b}["), "{forced:?}");
    assert_eq!(strip_sgr(&forced), plain, "the transcript is the same text");
    // 8 colours and bold: no exact colour value, no background.
    assert!(
        !forced.contains("38;") && !forced.contains("48;"),
        "{forced:?}"
    );
}

#[test]
fn no_color_wins_over_the_settings_file_and_the_command_line_wins_over_no_color() {
    let dir = tempfile::tempdir().unwrap();
    settings_in(dir.path(), "color = \"always\"\n");
    let play = |vars: &[(&str, &str)], extra: &[&str]| {
        let mut args = vec![
            "--demo",
            "--no-save",
            "--data-dir",
            dir_arg(&dir),
            "--seed",
            "3",
        ];
        args.extend_from_slice(extra);
        stdout(&run_in_env(&args, "\nNeon\ny\nscan\nquit\ny\n", vars))
    };
    assert!(
        play(&[], &[]).contains("\u{1b}["),
        "the file asks for colour"
    );
    assert!(
        !play(&[("NO_COLOR", "1")], &[]).contains('\u{1b}'),
        "NO_COLOR beats the file"
    );
    assert!(
        play(&[("NO_COLOR", "")], &[]).contains("\u{1b}["),
        "an empty NO_COLOR is ignored"
    );
    assert!(
        play(&[("NO_COLOR", "1")], &["--color", "always"]).contains("\u{1b}["),
        "--color beats NO_COLOR"
    );
    assert!(
        !play(&[], &["--no-color"]).contains('\u{1b}'),
        "--no-color beats the file"
    );
}

#[test]
fn a_screen_reader_never_gets_an_escape_sequence() {
    let out = colour_session(&["--screen-reader", "--color", "always"], &[]);
    assert!(!out.contains('\u{1b}'), "{out:?}");
}

#[test]
fn colour_options_are_checked() {
    assert_eq!(
        run(&["--demo", "--color", "sometimes"]).status.code(),
        Some(2)
    );
    assert_eq!(run(&["--demo", "--palette", "neon"]).status.code(), Some(2));
    assert_eq!(
        run(&["--demo", "--no-color", "--color", "always"])
            .status
            .code(),
        Some(2),
        "the two cannot be given together"
    );
}

#[test]
fn print_settings_shows_the_colour_decision_and_the_palette() {
    let shown = |extra: &[&str], vars: &[(&str, &str)]| {
        let mut args = vec!["--print-settings"];
        args.extend_from_slice(extra);
        stdout(&run_in_env(&args, "", vars))
    };
    let piped = shown(&[], &[]);
    assert!(piped.contains("color = false  # detection"), "{piped}");
    assert!(piped.contains("palette = \"mono\"  # no colour"), "{piped}");

    let forced = shown(
        &["--color", "always", "--palette", "cvd"],
        &[("NO_COLOR", "1")],
    );
    assert!(forced.contains("color = true  # command line"), "{forced}");
    assert!(
        forced.contains("palette = \"cvd\"  # command line"),
        "{forced}"
    );

    let silenced = shown(&[], &[("NO_COLOR", "1"), ("NEON_HACK_PALETTE", "cvd")]);
    assert!(silenced.contains("color = false  # NO_COLOR"), "{silenced}");
    assert!(
        silenced.contains("palette = \"mono\"  # no colour"),
        "{silenced}"
    );

    let from_env = shown(
        &["--color", "always"],
        &[("NEON_HACK_PALETTE", "high-contrast")],
    );
    assert!(
        from_env.contains("palette = \"high-contrast\"  # NEON_HACK_PALETTE"),
        "{from_env}"
    );

    let reader = shown(&["--screen-reader", "--color", "always"], &[]);
    assert!(
        reader.contains("color = false  # screen reader"),
        "{reader}"
    );
}

#[cfg(unix)]
#[test]
fn a_no_color_value_that_is_not_utf8_still_turns_colour_off() {
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    settings_in(dir.path(), "color = \"always\"\n");
    let output = Command::new(env!("CARGO_BIN_EXE_neon-hack"))
        .env_clear()
        .env("NO_COLOR", std::ffi::OsStr::from_bytes(&[0xff, 0xfe]))
        .args(["--print-settings", "--data-dir", dir_arg(&dir)])
        .output()
        .expect("failed to run neon-hack");
    let shown = stdout(&output);
    assert!(shown.contains("color = false  # NO_COLOR"), "{shown}");
}

// ---- Whole games, as golden transcripts -----------------------------------------------------

/// A winning game of the prudent player (found by the engine's own tests for seed 5).
const WINNING_GAME: &str = "\nNeon\ny\nscan\nscan\nlaylow\nshop\ncloak\n0\nscan\nscan\nscan\nscan\n\
    laylow\nscan\nscan\nscan\nlaylow\nscan\nscan\nlaylow\nscan\nscan\nscan\nlaylow\nscan\n\
    scan\nscan\nscan\nlaylow\nscan\nscan\nshop\ndeck\n";

fn transcript(extra: &[&str], script: &str) -> String {
    let mut args = vec!["--demo", "--seed", "5"];
    args.extend_from_slice(extra);
    let output = run_with_input(&args, script);
    assert!(output.status.success(), "{}", stderr(&output));
    stdout(&output)
}

#[test]
fn a_whole_game_won_reads_the_same_in_english() {
    insta::assert_snapshot!(transcript(&[], WINNING_GAME));
}

#[test]
fn a_whole_game_won_reads_the_same_in_french() {
    insta::assert_snapshot!(transcript(&["--lang", "fr"], WINNING_GAME));
}

#[test]
fn a_whole_game_won_with_the_screen_reader_and_ascii_modes_together() {
    let text = transcript(
        &["--lang", "fr", "--screen-reader", "--ascii"],
        WINNING_GAME,
    );
    assert!(text.is_ascii());
    insta::assert_snapshot!(text);
}

#[test]
fn a_whole_game_lost_to_reckless_scanning() {
    let script = format!("{PROLOGUE}{}", "scan\n".repeat(30));
    let text = transcript(&[], &script);
    assert!(text.contains("Security traced you."), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn an_ending_keeps_the_last_save_so_the_game_resumes_just_before_it() {
    let dir = tempfile::tempdir().unwrap();
    let won = run_in_env(
        &["--demo", "--data-dir", dir_arg(&dir), "--seed", "5"],
        WINNING_GAME,
        &[],
    );
    assert!(won.status.success(), "{}", stderr(&won));
    assert!(
        stdout(&won).contains("Mission complete"),
        "{}",
        stdout(&won)
    );
    let resumed = play_in(dir.path(), &[], "shop\n0\nquit\ny\n");
    assert!(resumed.status.success(), "{}", stderr(&resumed));
    let text = stdout(&resumed);
    assert!(text.contains("Resuming your saved game."), "{text}");
    assert!(
        text.contains("\n[3] Deck upgrade (120 credits)\n"),
        "the deck is still for sale:\n{text}"
    );
    assert!(!text.contains("Mission complete"), "{text}");
}

// ---- The campaign ----------------------------------------------------------------------------

/// The text of a campaign session with the placeholder texts of the narrative (`TODO
/// quest.m01.obj.2`, written in lot R2.3 and later) replaced by `<draft>`, so that what is
/// compared is the interface and not the story that is not written yet.
fn undraft(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("TODO ") {
        out.push_str(&rest[..at]);
        let tail = &rest[at + "TODO ".len()..];
        let key_len = tail
            .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.'))
            .unwrap_or(tail.len());
        out.push_str("<draft>");
        rest = &tail[key_len..];
    }
    out.push_str(rest);
    out
}

fn campaign(extra: &[&str], script: &str) -> String {
    let output = run_with_input(extra, script);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
    undraft(&stdout(&output))
}

/// The opening of a new campaign on a pipe: the handle (the default), the two pages of the
/// prologue, and the guided tutorial declined.
const OPENING: &str = "\n\n\nn\n";

/// A first look at the hub: the commands, the situation, what is not open yet.
const FIRST_LOOK: &str =
    "\n\n\nn\nhelp\nstatus\nquests\ncontacts\nnet\nshop\nmessages\nbogus\nquit\ny\n";

#[test]
fn the_campaign_plays_on_a_pipe_from_the_handle_to_the_farewell() {
    let text = campaign(&["--seed", "1"], FIRST_LOOK);
    assert!(text.contains("Handle [Neon]: \n"), "{text}");
    assert!(text.contains("Welcome to the net, Neon."), "{text}");
    assert!(text.contains("> help\nCommands\n"), "{text}");
    assert!(text.contains("[Error] Not yet: find R4Z0R."), "{text}");
    assert!(text.contains("[Error] Unknown command: bogus."), "{text}");
    assert!(text.ends_with("Goodbye, Neon.\n"), "{text:?}");
    assert!(!text.contains('\u{1b}'));
}

#[test]
fn the_first_look_at_the_hub_reads_the_same_in_english() {
    insta::assert_snapshot!(campaign(&[], FIRST_LOOK));
}

#[test]
fn the_first_look_at_the_hub_reads_the_same_in_french() {
    let text = campaign(&["--lang", "fr"], FIRST_LOOK);
    assert!(text.contains("Pas encore : trouver R4Z0R."), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn the_first_look_with_the_screen_reader_and_ascii_modes() {
    let reader = campaign(&["--screen-reader"], FIRST_LOOK);
    assert!(
        reader
            .contains("1. id: m01; title: First Steps in the Shadows; status: active; chapter: 1"),
        "{reader}"
    );
    assert!(!reader.contains("N E O N"), "no decoration");
    insta::assert_snapshot!(reader);
    let ascii = campaign(&["--ascii", "--lang", "fr"], FIRST_LOOK);
    assert!(ascii.is_ascii(), "{ascii}");
    insta::assert_snapshot!(ascii);
    let both = campaign(&["--screen-reader", "--ascii", "--lang", "fr"], FIRST_LOOK);
    assert!(both.is_ascii(), "{both}");
    insta::assert_snapshot!(both);
}

#[test]
fn the_campaign_difficulty_is_chosen_for_a_new_game_only() {
    let text = campaign(
        &["--difficulty", "hardcore"],
        &format!("{OPENING}status\nhint\nquit\ny\n"),
    );
    assert!(text.contains("Difficulty: Hardcore"), "{text}");
    assert!(text.contains("[Error] No hint left"), "{text}");
    let output = run(&["--demo", "--difficulty", "story"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "a difficulty is for the campaign"
    );
}

#[test]
fn the_two_games_are_never_played_together_and_the_game_options_work_for_either() {
    assert_eq!(run(&["--demo", "--campaign"]).status.code(), Some(2));
    // They work for the campaign (the default), the demo, and the campaign named.
    for game in [&[][..], &["--demo"], &["--campaign"]] {
        let mut listing = game.to_vec();
        listing.push("--list-saves");
        let output = run(&listing);
        assert!(output.status.success(), "{game:?}: {}", stderr(&output));
        let mut fresh = game.to_vec();
        fresh.extend(["--new", "--seed", "3"]);
        let output = run_with_input(&fresh, "");
        assert!(output.status.success(), "{game:?}: {}", stderr(&output));
    }
}

/// Runs the campaign with its saves in `dir`.
fn campaign_in(dir: &std::path::Path, args: &[&str], input: &str) -> Output {
    let dir = dir.to_str().expect("a UTF-8 temporary path");
    let mut all = vec!["--data-dir", dir, "--seed", "7"];
    all.extend_from_slice(args);
    run_with_input(&all, input)
}

#[test]
fn a_campaign_is_resumed_where_it_was_left_without_the_welcome() {
    let dir = tempfile::tempdir().unwrap();
    let first = campaign_in(
        dir.path(),
        &[],
        "Zed\n\n\nn\nquests\nhelp\nstatus\nhack localhost\ny\nquit\ny\n",
    );
    assert!(first.status.success(), "{}", stderr(&first));
    assert!(dir.path().join("saves-campaign/auto.toml").exists());
    assert!(
        !dir.path().join("saves").exists(),
        "the demo's folder is not touched"
    );

    let second = campaign_in(dir.path(), &[], "status\nnet\nquit\ny\n");
    assert!(second.status.success(), "{}", stderr(&second));
    let text = undraft(&stdout(&second));
    assert!(text.contains("Resuming your saved game."), "{text}");
    assert!(
        text.contains("Back on the net, Zed: 1 active quest, 0 unread messages."),
        "{text}"
    );
    assert!(
        !text.contains("Welcome"),
        "the welcome is not replayed:\n{text}"
    );
    assert!(!text.contains("Handle ["), "{text}");
    assert!(text.contains("Handle: Zed"), "{text}");
    assert!(
        text.contains("Notoriety: 4/100 (Discreet)"),
        "the intrusion is remembered:\n{text}"
    );
    assert!(text.contains("localhost - pierced - 1"), "{text}");
}

#[test]
fn campaign_saves_are_listed_apart_from_the_demos_and_a_checkpoint_comes_back() {
    let dir = tempfile::tempdir().unwrap();
    campaign_in(dir.path(), &[], "Zed\n\n\nn\nsave 3\nbuy 1\nquit\ny\n");
    // The demo plays in the same data folder and sees none of it.
    let demo = play_in(dir.path(), &["--list-saves"], "");
    assert!(
        stdout(&demo).ends_with("No saved game.\n"),
        "{}",
        stdout(&demo)
    );
    let listing = stdout(&campaign_in(dir.path(), &["--list-saves"], ""));
    assert!(listing.contains("saves-campaign"), "{listing}");
    assert!(listing.contains("auto: Zed, 0 actions"), "{listing}");
    assert!(
        listing.contains("checkpoint-1: Zed, 0 actions"),
        "{listing}"
    );
    assert!(listing.contains("slot-3: Zed, 0 actions"), "{listing}");
    // Loading a named save works for the campaign too, and a demo save is not a campaign.
    let output = campaign_in(dir.path(), &["--load", "slot-3"], "status\nquit\ny\n");
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("Handle: Zed"));
    let output = campaign_in(dir.path(), &["--load", "slot-4"], "");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("There is no save called slot-4."),
        "{}",
        stderr(&output)
    );
    play_in(dir.path(), &[], &format!("{PROLOGUE}save 1\nquit\ny\n"));
    let output = campaign_in(dir.path(), &["--new"], "Zed\n\n\nn\nquit\ny\n");
    assert!(output.status.success());
    assert!(dir.path().join("saves-campaign/auto.toml.bak").exists());
}

#[test]
fn the_first_three_quests_play_to_the_end_through_the_real_binary() {
    let script = "Neon\n\n\nn\nquests\nhelp\nnet\nstatus\ntalk echo7\n1\nhack localhost\ny\nlaylow\n\
        hack corp-server-01\ny\ntalk r4z0r\n1\nshop\nbuy stealth\ny\nhack localhost\ny\n\
        hack corp-server-01\ny\nhack underground-market\ny\narchives\ndecrypt doc_phase2\n\
        quests\nstatus\nquit\ny\n";
    let text = campaign(&["--seed", "9"], script);
    assert_eq!(text.matches("Quest completed: ").count(), 3, "{text}");
    assert!(text.contains("[Reward] Reached level 3."), "{text}");
    assert!(text.contains("Credits: 555"), "{text}");
    assert!(text.contains("Notoriety: 36/100 (Watched)"), "{text}");
    assert!(text.contains("[ALERT] Notoriety up: Watched."), "{text}");
    assert!(!text.contains("[Error]"), "{text}");
    assert!(text.ends_with("Goodbye, Neon.\n"), "{text:?}");
}

#[test]
fn closing_the_input_ends_the_campaign_instead_of_looping() {
    let output = run_with_input(&[], "");
    assert!(output.status.success());
    assert!(stdout(&output).ends_with("Input closed: ending the session.\n"));
    let output = run_with_input(&[], "Neon\nhack localhost\n");
    assert!(output.status.success());
    assert!(stdout(&output).ends_with("Input closed: ending the session.\n"));
}

// ---- The prologue and the guided tutorial ----------------------------------------------------

/// A new campaign through the prologue (the default handle, two pages, the tutorial accepted by
/// the default answer) and the first steps of the tutorial, in the order it teaches them.
const GUIDED_START: &str = "\n\n\n\nhelp\nstatus\nquests\ntalk echo7\n0\nnet\nquit\ny\n";

#[test]
fn a_new_campaign_plays_the_prologue_and_the_first_tutorial_steps_in_english() {
    let text = campaign(&[], GUIDED_START);
    assert!(text.contains("Neo-Tokyo, 2087."), "{text}");
    assert!(
        text.contains("Press Enter to continue, or type `skip`"),
        "{text}"
    );
    assert!(
        text.contains("Encrypted channel #7: incoming connection."),
        "{text}"
    );
    assert!(text.contains("Start the guided tutorial?"), "{text}");
    // Each hint is said once, right after the command that earns it, and never inside the
    // conversation menu.
    for step in [
        "Tutorial, step 1 of 10: Type `help`",
        "Tutorial, step 2 of 10: Type `status`",
        "Tutorial, step 3 of 10: Type `quests`",
        "Tutorial, step 4 of 10: Type `talk echo7`",
        "Tutorial, step 5 of 10: Type `net`",
        "Tutorial, step 6 of 10: Type `hack localhost`",
    ] {
        assert_eq!(text.matches(step).count(), 1, "{step}:\n{text}");
    }
    assert!(!text.contains("[Error]"), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn a_new_campaign_plays_the_prologue_and_the_first_tutorial_steps_in_french() {
    let text = campaign(&["--lang", "fr"], GUIDED_START);
    assert!(text.contains("Neo-Tokyo, 2087."), "{text}");
    assert!(text.contains("Lancer le tutoriel guidé ?"), "{text}");
    assert!(
        text.contains("Tutoriel, étape 2 sur 10 : Taper `status`"),
        "{text}"
    );
    assert!(!text.contains("[Erreur]"), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn the_prologue_reads_the_same_with_the_screen_reader_and_ascii_modes() {
    let script = "\n\n\n\nhelp\nquit\ny\n";
    let reader = campaign(&["--screen-reader"], script);
    assert!(
        reader.contains("Encrypted channel number 7: incoming connection."),
        "{reader}"
    );
    assert!(!reader.contains("N E O N"), "no decoration");
    insta::assert_snapshot!(reader);
    let ascii = campaign(&["--ascii", "--lang", "fr"], script);
    assert!(ascii.is_ascii(), "{ascii}");
    assert!(
        ascii.contains("Canal chiffre #7 : connexion entrante."),
        "{ascii}"
    );
    insta::assert_snapshot!(ascii);
    let both = campaign(&["--screen-reader", "--ascii", "--lang", "fr"], script);
    assert!(both.is_ascii(), "{both}");
    assert!(
        both.contains("Canal chiffre numero 7 : connexion entrante."),
        "{both}"
    );
    insta::assert_snapshot!(both);
}

#[test]
fn the_prologue_and_the_tutorial_can_be_skipped() {
    // `skip` at the first page goes to the contact with ECHO-7; declining ends the offer.
    let text = campaign(&[], "\nskip\nn\nstatus\ntutorial\nquit\ny\n");
    assert!(!text.contains("Nexus Corp runs the city"), "{text}");
    assert!(text.contains("ECHO-7 » Finally."), "{text}");
    assert!(
        text.contains("Tutorial skipped. `tutorial restart` starts it later."),
        "{text}"
    );
    assert!(text.contains("The tutorial is off."), "{text}");
    assert!(!text.contains("Tutorial, step"), "{text}");
    // Skipped later, from the command line.
    let text = campaign(&[], "\n\n\n\nhelp\ntutorial skip\nstatus\nquit\ny\n");
    assert_eq!(text.matches("Tutorial, step").count(), 2, "{text}");
    assert!(text.contains("Tutorial skipped."), "{text}");
}

#[test]
fn a_tutorial_in_progress_is_resumed_with_its_step_and_not_replayed() {
    let dir = tempfile::tempdir().unwrap();
    let first = campaign_in(dir.path(), &[], "Zed\n\n\n\nhelp\nstatus\nquit\ny\n");
    assert!(first.status.success(), "{}", stderr(&first));
    let second = campaign_in(dir.path(), &[], "net\nquests\nquit\ny\n");
    let text = undraft(&stdout(&second));
    assert!(text.contains("Resuming your saved game."), "{text}");
    // The instruction of the step it was on, once; ECHO-7's line is not said again.
    assert_eq!(
        text.matches("Tutorial, step 3 of 10: Type `quests`")
            .count(),
        1,
        "{text}"
    );
    assert!(!text.contains("Neo-Tokyo, 2087."), "{text}");
    assert!(!text.contains("ECHO-7 » Then the work."), "{text}");
    // The command that is not the current step says nothing; the current one moves on.
    assert!(
        text.contains("Tutorial, step 4 of 10: Type `talk echo7`"),
        "{text}"
    );
}

#[test]
fn a_campaign_closed_in_the_middle_of_the_prologue_comes_back_to_the_same_page() {
    let dir = tempfile::tempdir().unwrap();
    // The input closes at the second page: the session ends and the autosave holds the page.
    campaign_in(dir.path(), &[], "Zed\n\n");
    let second = campaign_in(dir.path(), &[], "\n\nquit\ny\n");
    let text = stdout(&second);
    assert!(text.contains("Nexus Corp runs the city"), "{text}");
    assert!(text.contains("Start the guided tutorial?"), "{text}");
}

// ---- The two games keep their saves apart ----------------------------------------------------

#[test]
fn a_save_of_the_other_game_is_refused_with_a_clear_message_and_left_alone() {
    let made = tempfile::tempdir().unwrap();
    campaign_in(made.path(), &[], "Zed\n\n\nn\nquit\ny\n");
    play_in(made.path(), &[], &format!("{PROLOGUE}scan\nquit\ny\n"));
    let campaign_save =
        std::fs::read_to_string(made.path().join("saves-campaign/auto.toml")).unwrap();
    let demo_save = std::fs::read_to_string(made.path().join("saves/auto.toml")).unwrap();

    // Each save is put in the folder of the other game, as a player moving files could.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("saves")).unwrap();
    std::fs::create_dir_all(dir.path().join("saves-campaign")).unwrap();
    std::fs::write(dir.path().join("saves/auto.toml"), &campaign_save).unwrap();
    std::fs::write(dir.path().join("saves-campaign/auto.toml"), &demo_save).unwrap();

    let demo = play_in(dir.path(), &[], "quit\ny\n");
    assert_eq!(demo.status.code(), Some(1));
    let message = stderr(&demo);
    assert!(
        message.contains(
            "This is a save of the campaign, not of the demo. To play the campaign, run `neon-hack`"
        ),
        "{message}"
    );
    assert!(message.contains("--new"), "{message}");
    assert!(!message.contains("damaged"), "{message}");

    let campaign = campaign_in(dir.path(), &[], "quit\ny\n");
    assert_eq!(campaign.status.code(), Some(1));
    let message = stderr(&campaign);
    assert!(
        message.contains("This is a save of the demo, not of the campaign. To play the demo, run `neon-hack --demo`"),
        "{message}"
    );

    // In French too, for a named save as well.
    let french = campaign_in(dir.path(), &["--lang", "fr", "--load", "auto"], "");
    assert_eq!(french.status.code(), Some(1));
    assert!(
        stderr(&french).contains("C'est une sauvegarde de la démo, pas de la campagne."),
        "{}",
        stderr(&french)
    );

    // Nothing was touched, and starting over works (the foreign save becomes the backup).
    assert_eq!(
        std::fs::read_to_string(dir.path().join("saves/auto.toml")).unwrap(),
        campaign_save
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("saves-campaign/auto.toml")).unwrap(),
        demo_save
    );
    let fresh = campaign_in(dir.path(), &["--new"], "Zed\n\n\nn\nquit\ny\n");
    assert!(fresh.status.success(), "{}", stderr(&fresh));
    assert!(dir.path().join("saves-campaign/auto.toml.bak").exists());

    // A damaged file is still told as damaged, not as another game's.
    std::fs::write(dir.path().join("saves/auto.toml"), "garbage").unwrap();
    let damaged = play_in(dir.path(), &[], "");
    assert!(stderr(&damaged).contains("damaged"), "{}", stderr(&damaged));
}

// ---- The whole campaign, to the epilogue ------------------------------------------------------

/// The lines of the optimistic player of the engine (`crates/neon-engine/tests/campaign`)
/// for three plans, one for each of three endings. A test of the engine checks that they are
/// what the walk types, so they cannot go stale; they are the opening (the handle, the prologue
/// skipped, no tutorial) then every command and menu answer up to the farewell.
const WALK_E1A: &str = include_str!("scripts/walk-e1a.txt");
const WALK_E3: &str = include_str!("scripts/walk-e3.txt");
const WALK_E4: &str = include_str!("scripts/walk-e4.txt");

/// What the whole walk is asserted to say, for a plan: its ending, and the epilogue line of
/// each of the nine contacts.
struct Finale {
    ending: &'static str,
    epilogue: [&'static str; 9],
}

const FINALE_E1A: Finale = Finale {
    ending: "e1a",
    epilogue: [
        "echo7_lives",
        "r4z0r_allied",
        "phoenix_free",
        "aura_free",
        "broker_neutral",
        "angel_free",
        "insider_",
        "ghost_stays",
        "miner_archives",
    ],
};

const FINALE_E3: Finale = Finale {
    ending: "e3",
    epilogue: [
        "echo7_absent",
        "r4z0r_hostile",
        "phoenix_lost",
        "aura_hosted",
        "broker_neutral",
        "angel_silenced",
        "insider_",
        "ghost_stays",
        "miner_archives",
    ],
};

const FINALE_E4: Finale = Finale {
    ending: "e4",
    epilogue: [
        "echo7_absent",
        "r4z0r_allied",
        "phoenix_turned",
        "aura_possessed",
        "broker_hostile",
        "angel_free",
        "insider_",
        "ghost_stays",
        "miner_archives",
    ],
};

/// Plays a walk through the real binary on a pipe, from a new game, and checks everything
/// the plan must have said: the campaign is finished, with its ending and the nine epilogue
/// lines, in the order the contacts are listed, once each, and no error anywhere.
fn whole_campaign(dir: &std::path::Path, extra: &[&str], script: &str, finale: &Finale) -> String {
    // The saves are written for real, in a folder of the test's own: the script saves a slot.
    let mut args = vec!["--data-dir", dir.to_str().expect("a UTF-8 temporary path")];
    args.extend_from_slice(extra);
    let output = run_with_input(&args, script);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
    let text = stdout(&output);
    for forbidden in ["[Error]", "[Erreur]", "<missing:", "<?", "\u{1b}"] {
        assert!(!text.contains(forbidden), "`{forbidden}` in the transcript");
    }
    // The ending screen and the epilogue are the drafts of chapter 6 for now: what shows
    // which ending and which lines were reached is their key.
    let titles: Vec<&str> = text.matches("TODO ending.").collect();
    assert_eq!(
        titles.len(),
        1 + 6 + usize::from(finale.ending == "e1a" || finale.ending == "e1b"),
        "{text}"
    );
    assert_eq!(
        text.matches(&format!("TODO ending.{}.title", finale.ending))
            .count(),
        1
    );
    assert_eq!(text.matches("TODO epilogue.").count(), 9, "{text}");
    // The lines come in the order of the contacts. The Insider's depends on how the walk
    // played a contract that can fail (a prefix says so), the others on the decisions.
    let mut at = text.rfind("TODO ending.").unwrap();
    for line in finale.epilogue {
        let needle = if line.ends_with('_') {
            format!("TODO epilogue.{line}")
        } else {
            format!("TODO epilogue.{line}\n")
        };
        let found = text[at..].find(&needle);
        assert!(
            found.is_some(),
            "epilogue.{line} is missing or out of order:\n{text}"
        );
        at += found.unwrap();
    }
    text
}

/// The last screens: from the conversation that puts the last decision to the farewell.
fn final_screens(text: &str) -> &str {
    let start = text.rfind("> talk aura\n").unwrap();
    &text[start..]
}

#[test]
fn the_whole_campaign_reaches_the_epilogue_in_english() {
    let dir = tempfile::tempdir().unwrap();
    let text = whole_campaign(dir.path(), &[], WALK_E1A, &FINALE_E1A);
    assert_eq!(text.matches("Quest completed: ").count(), 14, "{text}");
    assert!(!text.contains("Quest failed"), "{text}");
    assert!(text.contains("Welcome to the net, Walker."), "{text}");
    assert!(text.ends_with("Goodbye, Walker.\n"), "{text:?}");
    insta::assert_snapshot!(final_screens(&text));

    // The finished game is saved like any other: it comes back with nothing left to do, and
    // the slot the walk saved on the way is there.
    let saves = campaign_in(dir.path(), &["--list-saves"], "");
    assert!(
        stdout(&saves).contains("slot-3: Walker, "),
        "{}",
        stdout(&saves)
    );
    let again = campaign_in(dir.path(), &[], "status\nquests\nquit\ny\n");
    let again = stdout(&again);
    assert!(again.contains("Resuming your saved game."), "{again}");
    assert!(
        again.contains("Back on the net, Walker: 0 active quests"),
        "{again}"
    );
    assert!(again.contains("Quests in progress: 0"), "{again}");
}

#[test]
fn the_whole_campaign_reaches_the_epilogue_in_french() {
    let dir = tempfile::tempdir().unwrap();
    let text = whole_campaign(dir.path(), &["--lang", "fr"], WALK_E3, &FINALE_E3);
    assert_eq!(text.matches("Quête terminée : ").count(), 14, "{text}");
    assert!(text.contains("Bienvenue sur le réseau, Walker."), "{text}");
    assert!(text.ends_with("À bientôt, Walker.\n"), "{text:?}");
    insta::assert_snapshot!(final_screens(&text));
}

#[test]
fn the_whole_campaign_reaches_the_epilogue_with_the_screen_reader_and_ascii_modes() {
    let dir = tempfile::tempdir().unwrap();
    let text = whole_campaign(
        dir.path(),
        &["--screen-reader", "--ascii", "--lang", "fr"],
        WALK_E4,
        &FINALE_E4,
    );
    assert!(text.is_ascii(), "{text}");
    assert_eq!(text.matches("Quete terminee : ").count(), 14, "{text}");
    assert!(!text.contains("N E O N"), "no decoration");
    assert!(text.ends_with("A bientot, Walker.\n"), "{text:?}");
    insta::assert_snapshot!(final_screens(&text));
}
