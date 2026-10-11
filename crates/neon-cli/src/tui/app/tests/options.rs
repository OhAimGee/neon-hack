//! The presentation commands in the full-screen interface: they act on the screen at once,
//! on the whole history, and the game never hears of them.

use std::fs;

use super::staging::StageGame;
use super::*;
use crate::config::{LangChoice, VerbosityChoice};
use crate::options::{Catalogs, Fixed, Host, Live, Target};

fn host<'a>(en: &'a Catalog, fr: &'a Catalog, target: Target) -> Host<'a> {
    Host {
        catalogs: Catalogs { en, fr },
        live: Live {
            lang: LangChoice::En,
            verbosity: VerbosityChoice::Normal,
            ascii: false,
            palette: PaletteChoice::Default,
            reduce_motion: false,
        },
        fixed: Fixed {
            screen_reader: false,
            color: false,
            truecolor: false,
        },
        target,
        changed: Vec::new(),
    }
}

fn demo_app<'a>(
    game: &'a mut DemoGame,
    en: &'a Catalog,
    fr: &'a Catalog,
    target: Target,
) -> App<'a> {
    let mut app = app_with(game, en, RenderMode::FULL).with_options(host(en, fr, target));
    reach_command_line(&mut app);
    app
}

#[test]
fn options_is_shown_in_the_log_and_the_game_is_not_told() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut game = DemoGame::new(3);
    let mut app = demo_app(&mut game, &en, &fr, Target::Off);
    type_line(&mut app, "options");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "== Settings =="), "{rows:?}");
    assert!(
        has(&rows, "reduce_motion") && has(&rows, "screen_reader"),
        "{rows:?}"
    );
    assert!(!has(&rows, "Unknown command"), "{rows:?}");
    assert_eq!(app.prompt, Prompt::Command);
}

#[test]
fn without_the_commands_a_line_goes_to_the_game_as_before() {
    let (en, mut game) = (catalog_en(), DemoGame::new(3));
    let mut app = app_with(&mut game, &en, RenderMode::FULL);
    reach_command_line(&mut app);
    type_line(&mut app, "options");
    assert!(has(&screen(&app, 100, 28), "Unknown command"));
}

#[test]
fn the_language_changes_for_the_whole_history_and_the_chrome() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut game = DemoGame::new(3);
    let mut app = demo_app(&mut game, &en, &fr, Target::Off);
    type_line(&mut app, "help");
    let before = screen(&app, 100, 28);
    assert!(
        has(&before, "Commands") && has(&before, "Tab complete"),
        "{before:?}"
    );
    type_line(&mut app, "options lang fr");
    let after = screen(&app, 100, 28);
    // What was said before is said again in French, and so are the bars around it.
    assert!(
        has(&after, "Commandes") && !has(&after, "Commands"),
        "{after:?}"
    );
    assert!(has(&after, "Tab compléter"), "{after:?}");
    assert!(has(&after, "Réglage changé : lang = fr."), "{after:?}");
    assert!(has(&after, "Panneau"), "{after:?}");
    type_line(&mut app, "options lang en");
    assert!(has(&screen(&app, 100, 28), "Tab complete"));
}

#[test]
fn ascii_redraws_the_borders_and_everything_else_in_seven_bits() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut game = DemoGame::new(3);
    let mut app = demo_app(&mut game, &en, &fr, Target::Off);
    // A line typed before, with an accent: it is drawn again in seven bits.
    type_line(&mut app, "zoë");
    let before = screen(&app, 100, 28);
    assert!(
        before.iter().any(|row| !row.is_ascii()),
        "rounded borders, accents"
    );
    type_line(&mut app, "options ascii on");
    for row in screen(&app, 100, 28) {
        assert!(row.is_ascii(), "{row:?}");
    }
}

#[test]
fn the_verbosity_is_reported_and_changed() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut game = DemoGame::new(3);
    let mut app = demo_app(&mut game, &en, &fr, Target::Off);
    type_line(&mut app, "verbosity");
    assert!(has(&screen(&app, 100, 28), "Verbosity: normal."));
    type_line(&mut app, "verbosity brief");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "verbosity = brief"), "{rows:?}");
    type_line(&mut app, "verbosity");
    assert!(has(&screen(&app, 100, 28), "Verbosity: brief."));
}

#[test]
fn a_mistake_is_an_error_line_and_the_screen_keeps_working() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut game = DemoGame::new(3);
    let mut app = demo_app(&mut game, &en, &fr, Target::Off);
    type_line(&mut app, "options speed fast");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "Unknown setting: speed."), "{rows:?}");
    assert_eq!(input_row(&rows), ">");
}

#[test]
fn the_lines_are_remembered_like_any_command() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut game = DemoGame::new(3);
    let mut app = demo_app(&mut game, &en, &fr, Target::Off);
    type_line(&mut app, "options verbosity full");
    press(&mut app, KeyCode::Up);
    assert_eq!(
        input_row(&screen(&app, 100, 28)),
        "> options verbosity full"
    );
}

#[test]
fn a_question_is_answered_not_parsed_for_commands() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut game = DemoGame::new(3);
    let mut app = demo_app(&mut game, &en, &fr, Target::Off);
    // Ctrl-C asks to quit: the prompt is now a question, and `options` would be an answer.
    app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_ne!(app.prompt, Prompt::Command);
    type_line(&mut app, "options");
    assert!(!has(&screen(&app, 100, 28), "== Settings =="));
}

#[test]
fn reduced_motion_can_be_turned_on_and_off_in_the_game() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut game = StageGame;
    let first = game.start();
    let mut app = App::new(
        &mut game,
        Renderer {
            catalog: &en,
            mode: RenderMode::FULL,
            verbosity: Verbosity::Normal,
            colors: None,
        },
        first,
        Persistence::disabled(),
    )
    .with_motion(true)
    .with_options(host(&en, &fr, Target::Off));
    type_line(&mut app, "hack");
    assert!(app.is_animating());
    type_line(&mut app, "options reduce_motion on");
    type_line(&mut app, "hack");
    assert!(!app.is_animating(), "no scene once motion is reduced");
    type_line(&mut app, "options reduce_motion off");
    type_line(&mut app, "hack");
    assert!(app.is_animating());
}

#[test]
fn saving_writes_the_file_and_says_where() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("settings.toml");
    let mut game = DemoGame::new(3);
    let mut app = demo_app(&mut game, &en, &fr, Target::File(path.clone()));
    type_line(&mut app, "options lang fr");
    type_line(&mut app, "options save");
    assert!(has(&screen(&app, 100, 28), "Réglages gardés dans"));
    let written = fs::read_to_string(&path).unwrap();
    assert!(
        written.lines().any(|line| line == "lang = \"fr\""),
        "{written}"
    );
}
