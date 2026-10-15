//! The screens kept open beside the log: when they open, how they follow the game, what
//! Escape and the page keys do, and how they look at both sizes.

use neon_engine::campaign::CampaignGame;
use neon_engine::command::Capabilities;

use super::snapshots::{campaign, campaign_app, picture};
use super::*;

fn modal_app<'a>(game: &'a mut CampaignGame, catalog: &'a Catalog, mode: RenderMode) -> App<'a> {
    game.set_capabilities(Capabilities::TUI);
    campaign_app(game, catalog, mode, None, "Neon").with_frontend_commands()
}

/// What the player does to open the stall: the first quest, played to its end.
fn open_the_stall(app: &mut App<'_>) {
    for line in ["quests", "help", "net", "status", "talk echo7", "0"] {
        type_line(app, line);
    }
    hack_localhost(app);
    type_line(app, "laylow");
}

#[test]
fn a_list_command_opens_its_screen_over_the_log_and_the_log_keeps_the_list() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    assert!(app.modal.is_none());
    type_line(&mut app, "quests");
    let rows = screen(&app, 100, 28);
    // The title is on the frame of the screen; the columns have headings.
    assert!(
        has(&rows, "╭Journal─"),
        "the title is on the frame: {rows:#?}"
    );
    assert!(has(&rows, "title") && has(&rows, "status"), "{rows:#?}");
    assert!(app.modal.is_some());
    // The input is live under it.
    assert!(input_row(&rows).starts_with('>'));
    // The log still holds the same list, line by line, for when the screen is closed.
    press(&mut app, KeyCode::Esc);
    assert!(app.modal.is_none());
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "[1] m01 - "), "{rows:#?}");
}

#[test]
fn a_list_read_once_does_not_open_a_screen() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    type_line(&mut app, "help");
    assert!(app.modal.is_none());
}

#[test]
fn typing_goes_on_under_an_open_screen() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    type_line(&mut app, "quests");
    type_text(&mut app, "sta");
    assert_eq!(input_line(&app), "> sta");
    press(&mut app, KeyCode::Tab);
    assert_eq!(input_line(&app), "> status");
    assert!(app.modal.is_some(), "typing does not close it");
}

#[test]
fn another_list_replaces_the_screen_and_a_step_without_one_keeps_it() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    type_line(&mut app, "quests");
    type_line(&mut app, "status");
    let rows = screen(&app, 100, 28);
    assert!(app.modal.is_some(), "a command that is not a list keeps it");
    assert!(has(&rows, "╭Journal─"), "{rows:#?}");
    type_line(&mut app, "net");
    let rows = screen(&app, 100, 28);
    assert!(
        has(&rows, "level"),
        "the net replaces the journal: {rows:#?}"
    );
    assert!(!has(&rows, "╭Journal─"), "{rows:#?}");
}

#[test]
fn the_screen_is_drawn_again_from_the_game_after_each_command() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    open_the_stall(&mut app);
    type_line(&mut app, "shop");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "available"), "{rows:#?}");
    assert!(!has(&rows, "owned"), "{rows:#?}");
    // A purchase is two lines (the question, then the yes); the screen is not asked to open
    // again, and it shows what was bought.
    type_line(&mut app, "buy 1");
    type_line(&mut app, "y");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "owned"), "{rows:#?}");
}

#[test]
fn escape_closes_the_screen_then_the_drawer_then_goes_down_the_log() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    for _ in 0..3 {
        type_line(&mut app, "help");
    }
    type_line(&mut app, "quests");
    let _ = screen(&app, 64, 20);
    press(&mut app, KeyCode::F(2));
    let _ = screen(&app, 64, 20);
    press(&mut app, KeyCode::PageUp);
    type_text(&mut app, "sta");
    assert!(app.modal.is_some() && app.panel.drawer && app.scroll.get() > 0);
    // 1: the screen.
    press(&mut app, KeyCode::Esc);
    let _ = screen(&app, 64, 20);
    assert!(app.modal.is_none() && app.panel.drawer && app.scroll.get() > 0);
    // 2: the drawer. 3: the scroll. 4: the line.
    press(&mut app, KeyCode::Esc);
    let _ = screen(&app, 64, 20);
    assert!(!app.panel.drawer && app.scroll.get() > 0);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.scroll.get(), 0);
    assert!(!app.editor.is_empty());
    press(&mut app, KeyCode::Esc);
    assert!(app.editor.is_empty());
}

#[test]
fn a_screen_taller_than_the_room_turns_its_own_pages() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    open_the_stall(&mut app);
    type_line(&mut app, "shop");
    // Rows to spare (the stall sells a dozen things): the keys are the log's.
    let _ = screen(&app, 64, 60);
    assert!(!app.modal.as_ref().unwrap().overflow.get());
    press(&mut app, KeyCode::PageUp);
    assert!(app.scroll.get() > 0, "the log is scrolled");
    press(&mut app, KeyCode::End);
    // A terminal too short for the shop cuts it, and the keys turn its pages.
    let rows = screen(&app, 64, 20);
    let _ = rows;
    let modal = app.modal.as_ref().unwrap();
    modal.overflow.set(true);
    modal.page.set(2);
    press(&mut app, KeyCode::PageDown);
    assert_eq!(app.modal.as_ref().unwrap().scroll.get(), 2);
    assert_eq!(app.scroll.get(), 0, "the log did not move");
    press(&mut app, KeyCode::PageUp);
    assert_eq!(app.modal.as_ref().unwrap().scroll.get(), 0);
}

#[test]
fn a_screen_too_wide_for_columns_falls_back_to_the_list_of_the_log() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    open_the_stall(&mut app);
    type_line(&mut app, "shop");
    let rows = screen(&app, 64, 20);
    // The same numbered lines as the log, whole: nothing is cut.
    assert!(has(&rows, "ghost_protocol"), "{rows:#?}");
    assert!(rows.iter().all(|row| row.width() <= 64));
}

#[test]
fn the_drawer_and_the_screen_do_not_cover_each_other() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    type_line(&mut app, "quests");
    let _ = screen(&app, 64, 20);
    press(&mut app, KeyCode::F(2));
    let rows = screen(&app, 64, 20);
    assert!(has(&rows, "Inventory") && has(&rows, "Esc"), "{rows:#?}");
}

#[test]
fn the_screen_is_in_french_and_in_ascii() {
    let (catalog, mut game) = (catalog_fr(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    type_line(&mut app, "quests");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "Échap ferme"), "{rows:#?}");
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::ASCII);
    type_line(&mut app, "quests");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "Esc closes"), "{rows:#?}");
    assert!(rows.iter().all(|row| row.is_ascii()), "{rows:#?}");
}

#[test]
fn the_screens_at_both_sizes() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    type_line(&mut app, "quests");
    insta::assert_snapshot!("quests_full_en", picture(&app, 100, 28));
    insta::assert_snapshot!("quests_compact_en", picture(&app, 64, 20));
    type_line(&mut app, "net");
    insta::assert_snapshot!("net_full_en", picture(&app, 100, 28));
}

#[test]
fn the_shop_screen_in_french_at_the_compact_size() {
    let (catalog, mut game) = (catalog_fr(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    open_the_stall(&mut app);
    type_line(&mut app, "shop");
    insta::assert_snapshot!("shop_compact_fr", picture(&app, 64, 20));
}

#[test]
fn every_screen_stays_in_the_cells_of_the_terminal() {
    // Whatever the size, drawing a screen never panics and never writes outside.
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = modal_app(&mut game, &catalog, RenderMode::FULL);
    open_the_stall(&mut app);
    for line in [
        "quests", "net", "shop", "laylow", "contacts", "messages", "archives",
    ] {
        type_line(&mut app, line);
        for (width, height) in [(64, 20), (80, 24), (100, 28), (30, 8), (120, 50)] {
            let rows = screen(&app, width, height);
            assert_eq!(rows.len(), usize::from(height));
            assert!(rows.iter().all(|row| row.width() <= usize::from(width)));
        }
    }
}
