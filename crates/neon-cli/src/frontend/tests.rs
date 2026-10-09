use std::io::Cursor;

use neon_engine::text::Lang;

use super::*;
use crate::config::DisplayChoice;
use crate::test_support::{catalog_en, catalog_fr};

/// A terminal that is big enough, with nothing asked.
fn roomy() -> Facts {
    Facts {
        display: DisplayChoice::Auto,
        asked_on_command_line: false,
        screen_reader: false,
        stdin_is_terminal: true,
        stdout_is_terminal: true,
        dumb: false,
        size: Some((100, 28)),
        tui_built: true,
    }
}

fn decide(change: impl FnOnce(&mut Facts)) -> Decision {
    let mut facts = roomy();
    change(&mut facts);
    choose(&facts)
}

#[test]
fn a_big_enough_terminal_gets_the_full_screen_interface() {
    let decision = choose(&roomy());
    assert_eq!(decision.frontend, Frontend::Tui);
    assert_eq!(decision.notice, None);
    // The compact size is the smallest that does.
    assert_eq!(
        decide(|f| f.size = Some(MIN_SIZE)).frontend,
        Frontend::Tui,
        "{MIN_SIZE:?}"
    );
}

#[test]
fn a_pipe_on_either_side_gets_the_plain_interface_without_a_word() {
    for (stdin, stdout) in [(false, true), (true, false), (false, false)] {
        let decision = decide(|f| {
            f.stdin_is_terminal = stdin;
            f.stdout_is_terminal = stdout;
        });
        assert_eq!(decision.frontend, Frontend::Plain, "{stdin} {stdout}");
        assert_eq!(decision.notice, None, "routine: no excuse needed");
    }
}

#[test]
fn asking_for_the_full_screen_on_a_pipe_is_answered() {
    let decision = decide(|f| {
        f.asked_on_command_line = true;
        f.display = DisplayChoice::Tui;
        f.stdout_is_terminal = false;
    });
    assert_eq!(decision.frontend, Frontend::Plain);
    assert_eq!(decision.notice, Some(Notice::NoTerminal));
    let decision = decide(|f| {
        f.asked_on_command_line = true;
        f.dumb = true;
    });
    assert_eq!(decision.notice, Some(Notice::Dumb));
    let decision = decide(|f| {
        f.asked_on_command_line = true;
        f.tui_built = false;
    });
    assert_eq!(decision.notice, Some(Notice::NoTui));
}

#[test]
fn a_dumb_terminal_or_a_build_without_the_tui_gets_the_plain_interface() {
    assert_eq!(decide(|f| f.dumb = true).frontend, Frontend::Plain);
    assert_eq!(decide(|f| f.tui_built = false).frontend, Frontend::Plain);
    assert_eq!(decide(|f| f.size = None).frontend, Frontend::Plain);
}

#[test]
fn a_terminal_that_is_too_small_falls_back_and_says_why() {
    for size in [(63, 20), (64, 19), (50, 15), (0, 0), (200, 10)] {
        let decision = decide(|f| f.size = Some(size));
        assert_eq!(decision.frontend, Frontend::Plain, "{size:?}");
        assert_eq!(
            decision.notice,
            Some(Notice::TooSmall {
                width: size.0,
                height: size.1
            })
        );
    }
}

#[test]
fn the_player_can_forbid_the_full_screen_and_a_screen_reader_never_gets_it() {
    let plain = decide(|f| {
        f.display = DisplayChoice::Plain;
        f.asked_on_command_line = true;
    });
    assert_eq!((plain.frontend, plain.notice), (Frontend::Plain, None));
    let reader = decide(|f| {
        f.screen_reader = true;
        f.display = DisplayChoice::Tui;
    });
    assert_eq!((reader.frontend, reader.notice), (Frontend::Plain, None));
    // Too small and plain asked: nothing to excuse.
    let both = decide(|f| {
        f.display = DisplayChoice::Plain;
        f.size = Some((10, 5));
    });
    assert_eq!(both.notice, None);
}

#[test]
fn a_saved_wish_for_the_full_screen_is_not_nagged_about_on_a_pipe() {
    let decision = decide(|f| {
        f.display = DisplayChoice::Tui;
        f.stdin_is_terminal = false;
    });
    assert_eq!(
        (decision.frontend, decision.notice),
        (Frontend::Plain, None)
    );
}

#[test]
fn every_notice_has_a_text_in_both_languages() {
    for notice in [
        Notice::TooSmall {
            width: 50,
            height: 15,
        },
        Notice::NoTerminal,
        Notice::Dumb,
        Notice::NoTui,
    ] {
        for catalog in [catalog_en(), catalog_fr()] {
            let text = render(&notice_text(notice), &catalog, RenderMode::FULL);
            assert!(!text.contains("<missing"), "{notice:?}: {text}");
        }
    }
    let said = render(
        &notice_text(Notice::TooSmall {
            width: 50,
            height: 15,
        }),
        &catalog_en(),
        RenderMode::FULL,
    );
    assert!(said.contains("50x15") && said.contains("64x20"), "{said}");
}

// ---- The first question ------------------------------------------------------------------

#[test]
fn answers_are_numbers_or_words_in_either_language_and_empty_is_the_first() {
    for (line, expected) in [
        ("", Some(ModeAnswer::Full)),
        ("1\n", Some(ModeAnswer::Full)),
        (" Full ", Some(ModeAnswer::Full)),
        ("plein", Some(ModeAnswer::Full)),
        ("2", Some(ModeAnswer::Plain)),
        ("LIGNE", Some(ModeAnswer::Plain)),
        ("3", Some(ModeAnswer::ScreenReader)),
        ("screen-reader", Some(ModeAnswer::ScreenReader)),
        ("lecteur", Some(ModeAnswer::ScreenReader)),
        ("4", None),
        ("yes", None),
        ("0", None),
    ] {
        assert_eq!(parse_answer(line), expected, "{line:?}");
    }
}

#[test]
fn the_question_is_asked_only_the_first_time_on_a_terminal_when_nothing_else_decides() {
    assert!(question_due(true, true, false, false, false));
    assert!(
        !question_due(false, true, false, false, false),
        "a file exists"
    );
    assert!(
        !question_due(true, false, false, false, false),
        "not a terminal"
    );
    assert!(
        !question_due(true, true, true, false, false),
        "a flag decides"
    );
    assert!(
        !question_due(true, true, false, true, false),
        "screen reader"
    );
    assert!(
        !question_due(true, true, false, false, true),
        "--no-save writes nothing"
    );
}

fn ask(input: &str, lang: Lang) -> (Option<ModeAnswer>, String) {
    let catalog = match lang {
        Lang::En => catalog_en(),
        Lang::Fr => catalog_fr(),
    };
    let mut out = Vec::new();
    let answer = ask_mode(
        &catalog,
        RenderMode::FULL,
        &mut Cursor::new(input.as_bytes().to_vec()),
        &mut out,
    )
    .unwrap();
    (answer, String::from_utf8(out).unwrap())
}

#[test]
fn the_question_lists_the_three_modes_and_takes_the_answer() {
    let (answer, shown) = ask("3\n", Lang::En);
    assert_eq!(answer, Some(ModeAnswer::ScreenReader));
    assert!(shown.contains("How do you want to play?"), "{shown}");
    for entry in ["[1] Full screen", "[2] Line by line", "[3] Screen reader"] {
        assert!(shown.contains(entry), "{entry}: {shown}");
    }
    assert!(shown.ends_with("Your choice [1]: "), "{shown:?}");
    let (answer, shown) = ask("\n", Lang::Fr);
    assert_eq!(answer, Some(ModeAnswer::Full));
    assert!(shown.contains("[1] Plein écran"), "{shown}");
}

#[test]
fn a_bad_answer_is_asked_again_and_a_closed_input_decides_nothing() {
    let (answer, shown) = ask("maybe\n9\n2\n", Lang::En);
    assert_eq!(answer, Some(ModeAnswer::Plain));
    assert_eq!(shown.matches("Answer 1, 2 or 3.").count(), 2, "{shown}");
    assert_eq!(shown.matches("Your choice [1]: ").count(), 3);
    let (answer, _) = ask("", Lang::En);
    assert_eq!(answer, None, "nothing is decided, so nothing is written");
    let (answer, _) = ask("nope", Lang::En);
    assert_eq!(answer, None);
}

#[test]
fn the_answer_creates_the_settings_file_and_never_replaces_one() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("data").join("settings.toml");
    save_choice(&path, ModeAnswer::ScreenReader).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("display = \"plain\"\nscreen_reader = true\n"),
        "{text}"
    );
    // The file reads back as settings.
    let (settings, state) = crate::config::load_file(&path);
    assert_eq!(state, crate::config::FileState::Loaded);
    let _ = settings;
    // A second answer does not touch it.
    assert!(save_choice(&path, ModeAnswer::Full).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    // Neither does it touch a damaged file.
    let damaged = folder.path().join("damaged.toml");
    std::fs::write(&damaged, "this is = not [toml").unwrap();
    assert!(save_choice(&damaged, ModeAnswer::Plain).is_err());
    assert_eq!(
        std::fs::read_to_string(&damaged).unwrap(),
        "this is = not [toml"
    );
}

#[test]
fn each_answer_writes_the_wish_it_stands_for() {
    for (answer, display, screen_reader) in [
        (ModeAnswer::Full, DisplayChoice::Tui, None),
        (ModeAnswer::Plain, DisplayChoice::Plain, None),
        (ModeAnswer::ScreenReader, DisplayChoice::Plain, Some(true)),
    ] {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("settings.toml");
        save_choice(&path, answer).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let parsed: toml::Table = text.parse().unwrap();
        assert_eq!(
            parsed.get("display").and_then(toml::Value::as_str),
            Some(display.name()),
            "{answer:?}"
        );
        assert_eq!(
            parsed.get("screen_reader").and_then(toml::Value::as_bool),
            screen_reader,
            "{answer:?}"
        );
    }
}
