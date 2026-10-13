//! `load` in the full-screen interface: the saves are listed in the log, a name asks a real
//! question in the place of the prompt, and a yes replaces the game.

use neon_engine::campaign::CampaignGame;
use neon_engine::save::SaveRequest;
use tempfile::TempDir;

use super::options::host;
use super::snapshots::campaign;
use super::*;
use crate::options::Target;
use crate::store::Store;

/// The save of a game that Zoe has played through the opening.
fn zoe_save() -> String {
    let mut game = campaign();
    game.start();
    game.handle(Input::Line("Zoe".to_owned()));
    game.handle(Input::Continue);
    game.handle(Input::Continue);
    game.handle(Input::Confirm(false));
    game.snapshot().unwrap()
}

/// A saves folder with Zoe's game in slot 1.
fn saves() -> (TempDir, Store) {
    let folder = TempDir::new().unwrap();
    let store = Store::new(folder.path().to_path_buf());
    store.write(SaveRequest::Slot(1), &zoe_save()).unwrap();
    (folder, store)
}

/// Neon's game at the command line, with the commands of the campaign and `store` for saves.
fn app_with_saves<'a>(
    game: &'a mut CampaignGame,
    catalog: &'a Catalog,
    fr: &'a Catalog,
    store: Store,
) -> App<'a> {
    let first = game.start();
    let mut app = App::new(
        game,
        Renderer {
            catalog,
            mode: RenderMode::FULL,
            verbosity: Verbosity::Normal,
            colors: None,
        },
        first,
        Persistence::new(store),
    )
    .with_options(host(catalog, fr, Target::Off));
    reach_campaign_command_line(&mut app);
    app
}

fn reach_campaign_command_line(app: &mut App<'_>) {
    type_line(app, "Neon");
    press(app, KeyCode::Enter);
    press(app, KeyCode::Enter);
    type_line(app, "n");
}

#[test]
fn load_lists_the_saves_in_the_log() {
    let (_folder, store) = saves();
    let (en, fr, mut game) = (catalog_en(), catalog_fr(), campaign());
    let mut app = app_with_saves(&mut game, &en, &fr, store);
    type_line(&mut app, "load");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "== Saves =="), "{rows:#?}");
    assert!(has(&rows, "slot-1 - Zoe - "), "{rows:#?}");
    assert!(!app.loading.is_pending());
    assert!(input_row(&rows).starts_with('>'));
}

#[test]
fn a_name_asks_a_real_question_and_no_leaves_the_game_alone() {
    let (_folder, store) = saves();
    let (en, fr, mut game) = (catalog_en(), catalog_fr(), campaign());
    let mut app = app_with_saves(&mut game, &en, &fr, store);
    type_line(&mut app, "load slot-1");
    let rows = screen(&app, 100, 28);
    // The question is the prompt: asked above the cursor, with the safe answer by default.
    let question = input_row(&rows);
    assert!(
        question.contains("Load slot-1 (Zoe,") && question.contains("[y/N]"),
        "{question}"
    );
    assert!(
        has(&rows, "is lost"),
        "the warning is in the log: {rows:#?}"
    );
    // Typing the word of another command does not run it: it is not an answer.
    type_line(&mut app, "status");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "Please answer yes or no."), "{rows:#?}");
    assert!(input_row(&rows).contains("[y/N]"), "the question stands");
    type_line(&mut app, "n");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "Nothing was loaded."), "{rows:#?}");
    assert_eq!(input_row(&rows), ">", "the game's prompt is back");
    assert!(rows[0].contains("Neon"), "still Neon's game: {rows:#?}");
}

#[test]
fn yes_replaces_the_game_and_the_log_says_so() {
    let (_folder, store) = saves();
    let (en, fr, mut game) = (catalog_en(), catalog_fr(), campaign());
    let mut app = app_with_saves(&mut game, &en, &fr, store);
    // A list was open: it belonged to the game that goes.
    type_line(&mut app, "quests");
    assert!(app.modal.is_some());
    type_line(&mut app, "load slot-1");
    type_line(&mut app, "y");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "Loaded slot-1."), "{rows:#?}");
    assert!(
        rows[0].contains("Zoe"),
        "the status bar follows the new game: {rows:#?}"
    );
    assert_eq!(input_row(&rows), ">");
    assert!(app.modal.is_none());
    // The history is still there, and the game answers as Zoe's.
    assert!(has(&rows, "> load slot-1"), "{rows:#?}");
    type_line(&mut app, "status");
    assert!(has(&screen(&app, 100, 28), "Handle: Zoe"));
}

#[test]
fn escape_is_no() {
    let (_folder, store) = saves();
    let (en, fr, mut game) = (catalog_en(), catalog_fr(), campaign());
    let mut app = app_with_saves(&mut game, &en, &fr, store);
    type_line(&mut app, "load slot-1");
    press(&mut app, KeyCode::Esc);
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "Nothing was loaded."), "{rows:#?}");
    assert!(rows[0].contains("Neon"));
}

#[test]
fn closing_the_input_while_asked_still_ends_the_game() {
    let (_folder, store) = saves();
    let (en, fr, mut game) = (catalog_en(), catalog_fr(), campaign());
    let mut app = app_with_saves(&mut game, &en, &fr, store);
    type_line(&mut app, "load slot-1");
    ctrl(&mut app, 'd');
    assert!(app.finished, "the game was told the input closed");
    assert!(!app.loading.is_pending());
}

#[test]
fn a_game_without_the_commands_of_the_campaign_does_not_take_load() {
    let (en, mut game) = (catalog_en(), DemoGame::new(3));
    let mut app = app_with(&mut game, &en, RenderMode::FULL);
    reach_command_line(&mut app);
    type_line(&mut app, "load");
    assert!(has(&screen(&app, 100, 28), "Unknown command"));
}

#[test]
fn with_no_save_folder_load_says_saving_is_off() {
    let (en, fr, mut game) = (catalog_en(), catalog_fr(), campaign());
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
    .with_options(host(&en, &fr, Target::Off));
    reach_campaign_command_line(&mut app);
    type_line(&mut app, "load");
    assert!(has(&screen(&app, 100, 28), "Saving is turned off"));
}
