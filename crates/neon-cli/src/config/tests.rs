use super::*;
use crate::palette::{ColorChoice, PaletteChoice};

fn env(vars: &[(&str, &str)]) -> Env {
    let mut env = Env::default();
    for (name, value) in vars {
        let value = Some((*value).to_owned());
        match *name {
            "LC_ALL" => env.lc_all = value,
            "LC_MESSAGES" => env.lc_messages = value,
            "LC_CTYPE" => env.lc_ctype = value,
            "LANG" => env.lang = value,
            "NEON_HACK_LANG" => env.neon_lang = value,
            other => panic!("unknown variable {other}"),
        }
    }
    env
}

fn file(text: &str) -> FileSettings {
    parse_file(text).unwrap()
}

fn lang(cli: Option<LangChoice>, env: &Env, file: FileSettings) -> (LangChoice, Source) {
    let cli = CliPresentation {
        lang: cli,
        ..CliPresentation::default()
    };
    let resolved = resolve(cli, env, file).lang;
    (resolved.value, resolved.source)
}

#[test]
fn the_language_comes_from_the_most_specific_source() {
    let nothing = FileSettings::default();
    let all = env(&[("NEON_HACK_LANG", "fr"), ("LANG", "en_US.UTF-8")]);
    let settings = file("lang = \"en\"");

    // command line > NEON_HACK_LANG > settings.toml > locale > English
    assert_eq!(
        lang(Some(LangChoice::En), &all, settings),
        (LangChoice::En, Source::Cli)
    );
    assert_eq!(
        lang(None, &all, settings),
        (LangChoice::Fr, Source::EnvLang)
    );
    let no_neon = env(&[("LANG", "fr_FR.UTF-8")]);
    assert_eq!(
        lang(None, &no_neon, settings),
        (LangChoice::En, Source::File)
    );
    assert_eq!(
        lang(None, &no_neon, nothing),
        (LangChoice::Fr, Source::Locale)
    );
    assert_eq!(
        lang(None, &Env::default(), nothing),
        (LangChoice::En, Source::Default)
    );
}

#[test]
fn the_locale_follows_the_posix_order_and_unknown_languages_say_nothing() {
    let nothing = FileSettings::default();
    let both = env(&[("LC_ALL", "fr_CA.UTF-8"), ("LANG", "en_US.UTF-8")]);
    assert_eq!(lang(None, &both, nothing).0, LangChoice::Fr, "LC_ALL wins");
    let messages = env(&[("LC_MESSAGES", "fr"), ("LANG", "en")]);
    assert_eq!(
        lang(None, &messages, nothing).0,
        LangChoice::Fr,
        "LC_MESSAGES over LANG"
    );
    for unknown in ["de_DE.UTF-8", "C", "POSIX", "C.UTF-8", "", "xx@euro"] {
        let env = env(&[("LANG", unknown)]);
        assert_eq!(
            lang(None, &env, nothing),
            (LangChoice::En, Source::Default),
            "{unknown:?}"
        );
    }
    assert_eq!(
        lang(None, &env(&[("LANG", "FR_fr")]), nothing).0,
        LangChoice::Fr
    );
    assert_eq!(
        lang(None, &env(&[("LANG", "fr@euro")]), nothing).0,
        LangChoice::Fr
    );
}

#[test]
fn a_language_the_game_does_not_have_falls_through_to_the_next_source() {
    let settings = file("lang = \"fr\"");
    let env = env(&[("NEON_HACK_LANG", "de")]);
    assert_eq!(lang(None, &env, settings), (LangChoice::Fr, Source::File));
}

#[test]
fn verbosity_screen_reader_and_ascii_go_command_line_then_file_then_default() {
    let settings = file("verbosity = \"full\"\nascii = true\nscreen_reader = true");
    let none = CliPresentation::default();
    let from_file = resolve(none, &Env::default(), settings);
    assert_eq!(
        from_file.verbosity,
        sourced(VerbosityChoice::Full, Source::File)
    );
    assert_eq!(from_file.ascii, sourced(true, Source::File));
    assert_eq!(from_file.screen_reader, sourced(true, Source::File));

    let cli = CliPresentation {
        verbosity: Some(VerbosityChoice::Brief),
        ascii: true,
        screen_reader: true,
        ..none
    };
    let from_cli = resolve(cli, &Env::default(), FileSettings::default());
    assert_eq!(
        from_cli.verbosity,
        sourced(VerbosityChoice::Brief, Source::Cli)
    );
    assert_eq!(from_cli.ascii, sourced(true, Source::Cli));
    assert_eq!(from_cli.screen_reader, sourced(true, Source::Cli));

    let defaults = resolve(none, &Env::default(), FileSettings::default());
    assert_eq!(
        defaults.verbosity,
        sourced(VerbosityChoice::Normal, Source::Default)
    );
    assert_eq!(defaults.ascii, sourced(false, Source::Default));
    assert_eq!(defaults.screen_reader, sourced(false, Source::Default));

    // The file can turn a flag off for good only by saying so; the command line can only turn on.
    let off = resolve(none, &Env::default(), file("ascii = false"));
    assert_eq!(off.ascii, sourced(false, Source::File));
}

#[test]
fn a_locale_that_names_another_charset_turns_ascii_on_by_itself() {
    let none = CliPresentation::default();
    let ascii = |vars: &[(&str, &str)]| resolve(none, &env(vars), FileSettings::default()).ascii;
    assert_eq!(
        ascii(&[("LANG", "fr_FR.ISO-8859-1")]),
        sourced(true, Source::Locale)
    );
    assert_eq!(
        ascii(&[("LC_ALL", "en_US.latin1")]),
        sourced(true, Source::Locale)
    );
    for fine in [
        "fr_FR.UTF-8",
        "fr_FR.utf8",
        "fr_FR.UTF-8@euro",
        "fr_FR",
        "C",
        "POSIX",
        "C.UTF-8",
    ] {
        assert_eq!(
            ascii(&[("LANG", fine)]),
            sourced(false, Source::Default),
            "{fine}"
        );
    }
    // LC_CTYPE decides the charset, but LC_ALL beats it.
    assert!(ascii(&[("LC_CTYPE", "fr_FR.ISO-8859-1"), ("LANG", "fr_FR.UTF-8")]).value);
    assert!(!ascii(&[("LC_ALL", "fr_FR.UTF-8"), ("LC_CTYPE", "fr_FR.ISO-8859-1")]).value);
    // An explicit setting in the file wins over the guess.
    let settings = file("ascii = false");
    let guessed = resolve(none, &env(&[("LANG", "fr_FR.ISO-8859-1")]), settings);
    assert_eq!(guessed.ascii, sourced(false, Source::File));
}

#[test]
fn the_settings_file_ignores_unknown_keys_and_refuses_wrong_values() {
    assert_eq!(file(""), FileSettings::default());
    assert_eq!(file("# nothing here\n"), FileSettings::default());
    assert_eq!(
        file("future_option = 7\nlang = \"fr\"").lang,
        Some(LangChoice::Fr)
    );
    for bad in [
        "lang = \"de\"",
        "lang = 3",
        "verbosity = \"loud\"",
        "ascii = \"yes\"",
        "screen_reader = 1",
        "lang = ",
        "not toml ===",
    ] {
        assert!(parse_file(bad).is_err(), "{bad}");
    }
}

#[test]
fn reading_the_file_tells_missing_loaded_and_damaged_apart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    assert_eq!(
        load_file(&path),
        (FileSettings::default(), FileState::Missing)
    );

    fs::write(&path, "lang = \"fr\"\n").unwrap();
    let (settings, state) = load_file(&path);
    assert_eq!(
        (settings.lang, state),
        (Some(LangChoice::Fr), FileState::Loaded)
    );

    fs::write(&path, "lang = \"klingon\"\n").unwrap();
    let (settings, state) = load_file(&path);
    assert_eq!(
        settings,
        FileSettings::default(),
        "a damaged file gives the defaults"
    );
    assert!(matches!(state, FileState::Damaged(_)));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "lang = \"klingon\"\n",
        "and is never rewritten"
    );

    fs::write(
        &path,
        vec![b'#'; usize::try_from(MAX_SETTINGS_BYTES).unwrap() + 1],
    )
    .unwrap();
    assert!(
        matches!(load_file(&path).1, FileState::Damaged(reason) if reason.contains("too large"))
    );
    fs::write(&path, [0xff, 0xfe]).unwrap();
    assert!(matches!(load_file(&path).1, FileState::Damaged(_)));
}

#[test]
fn the_data_folder_comes_from_the_command_line_then_the_environment_then_the_system() {
    let from_env = Env {
        neon_data_dir: Some(PathBuf::from("/from/env")),
        ..Env::default()
    };
    assert_eq!(
        data_dir(Some(Path::new("/from/cli")), &from_env).unwrap(),
        (PathBuf::from("/from/cli"), Source::Cli)
    );
    assert_eq!(
        data_dir(None, &from_env).unwrap(),
        (PathBuf::from("/from/env"), Source::EnvDataDir)
    );
    let system = Env {
        system_data_dir: Some(PathBuf::from("/system/neon-hack")),
        ..Env::default()
    };
    assert_eq!(
        data_dir(None, &system).unwrap(),
        (PathBuf::from("/system/neon-hack"), Source::Default)
    );
    assert!(
        data_dir(None, &Env::default()).is_err(),
        "no folder anywhere is an error that suggests --data-dir"
    );
}

fn terminal_env(vars: &[(&str, &str)], tty: bool) -> Env {
    let mut env = Env {
        stdout_is_terminal: tty,
        ..Env::default()
    };
    for (name, value) in vars {
        let value = Some((*value).to_owned());
        match *name {
            "TERM" => env.term = value,
            "COLORTERM" => env.colorterm = value,
            "NEON_HACK_PALETTE" => env.neon_palette = value,
            "NO_COLOR" => env.no_color = true,
            other => panic!("unknown variable {other}"),
        }
    }
    env
}

#[test]
fn the_terminal_is_read_from_term_and_colorterm() {
    let terminal = |vars: &[(&str, &str)]| terminal_env(vars, true).terminal();
    assert!(!terminal(&[]).dumb && !terminal(&[]).truecolor);
    assert!(terminal(&[("TERM", "dumb")]).dumb);
    assert!(!terminal(&[("TERM", "xterm-256color")]).dumb);
    assert!(terminal(&[("COLORTERM", "truecolor")]).truecolor);
    assert!(terminal(&[("COLORTERM", "24bit")]).truecolor);
    assert!(!terminal(&[("COLORTERM", "yes")]).truecolor);
    assert!(terminal_env(&[], true).terminal().is_tty);
    assert!(!terminal_env(&[], false).terminal().is_tty);
}

#[test]
fn colour_and_palette_follow_the_precedence_and_mono_follows_no_colour() {
    let present = |cli: CliPresentation, env: &Env, file: FileSettings| resolve(cli, env, file);
    let none = CliPresentation::default();
    let on_tty = terminal_env(&[], true);

    // Detected colour, default palette.
    let detected = present(none, &on_tty, FileSettings::default());
    assert!(detected.color.0);
    assert_eq!(
        detected.palette,
        sourced(PaletteChoice::Default, Source::Default)
    );
    // A pipe has none, so the palette is mono.
    let piped = present(none, &terminal_env(&[], false), FileSettings::default());
    assert!(!piped.color.0);
    assert_eq!(piped.palette, sourced(PaletteChoice::Mono, Source::NoColor));

    // NO_COLOR forces mono, even over the file and a chosen palette; --color overrides it.
    let no_color = terminal_env(&[("NO_COLOR", "1"), ("NEON_HACK_PALETTE", "cvd")], true);
    let settings = file("color = \"always\"\npalette = \"high-contrast\"");
    let silenced = present(none, &no_color, settings);
    assert!(!silenced.color.0);
    assert_eq!(silenced.palette.value, PaletteChoice::Mono);
    let forced = CliPresentation {
        color: Some(ColorChoice::Always),
        ..none
    };
    let colour_again = present(forced, &no_color, settings);
    assert!(colour_again.color.0);
    assert_eq!(
        colour_again.palette,
        sourced(PaletteChoice::Cvd, Source::EnvPalette)
    );

    // Palette: command line > NEON_HACK_PALETTE > settings > default; an unknown name falls through.
    let chosen = CliPresentation {
        palette: Some(PaletteChoice::Cvd),
        ..none
    };
    let with_env = terminal_env(&[("NEON_HACK_PALETTE", "high-contrast")], true);
    assert_eq!(
        present(chosen, &with_env, settings).palette,
        sourced(PaletteChoice::Cvd, Source::Cli)
    );
    assert_eq!(
        present(none, &with_env, settings).palette,
        sourced(PaletteChoice::HighContrast, Source::EnvPalette)
    );
    let from_file = file("palette = \"cvd\"");
    assert_eq!(
        present(none, &on_tty, from_file).palette,
        sourced(PaletteChoice::Cvd, Source::File)
    );
    let bad_env = terminal_env(&[("NEON_HACK_PALETTE", "neon")], true);
    assert_eq!(
        present(none, &bad_env, from_file).palette.source,
        Source::File
    );

    // A screen reader has no colour whatever else is asked.
    let reader = CliPresentation {
        screen_reader: true,
        color: Some(ColorChoice::Always),
        ..none
    };
    let silent = present(reader, &on_tty, settings);
    assert!(!silent.color.0);
    assert_eq!(silent.palette.value, PaletteChoice::Mono);
}

#[test]
fn the_settings_file_accepts_colour_and_palette_and_refuses_unknown_ones() {
    let settings = file("color = \"never\"\npalette = \"high-contrast\"");
    assert_eq!(settings.color, Some(ColorChoice::Never));
    assert_eq!(settings.palette, Some(PaletteChoice::HighContrast));
    for bad in [
        "color = \"sometimes\"",
        "palette = \"neon\"",
        "color = true",
    ] {
        assert!(parse_file(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_display_wish_goes_command_line_then_file_then_auto() {
    let display = |cli: Option<DisplayChoice>, text: &str| {
        let cli = CliPresentation {
            display: cli,
            ..CliPresentation::default()
        };
        let resolved = resolve(cli, &Env::default(), file(text)).display;
        (resolved.value, resolved.source)
    };
    assert_eq!(display(None, ""), (DisplayChoice::Auto, Source::Default));
    assert_eq!(
        display(None, "display = \"plain\""),
        (DisplayChoice::Plain, Source::File)
    );
    assert_eq!(
        display(None, "display = \"tui\""),
        (DisplayChoice::Tui, Source::File)
    );
    assert_eq!(
        display(Some(DisplayChoice::Tui), "display = \"plain\""),
        (DisplayChoice::Tui, Source::Cli)
    );
    assert_eq!(
        display(Some(DisplayChoice::Plain), "display = \"tui\""),
        (DisplayChoice::Plain, Source::Cli)
    );
    // A value the game does not know makes the file unusable, as for every other setting.
    assert!(parse_file("display = \"holographic\"").is_err());
}
