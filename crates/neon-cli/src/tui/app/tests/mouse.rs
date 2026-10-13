//! The mouse wheel, when the mouse was asked for: it scrolls what the page keys scroll.

use super::snapshots::{campaign, campaign_app};
use super::*;

fn filled<'a>(game: &'a mut neon_engine::campaign::CampaignGame, catalog: &'a Catalog) -> App<'a> {
    let mut app = campaign_app(game, catalog, RenderMode::FULL, None, "Neon");
    for _ in 0..4 {
        type_line(&mut app, "help");
    }
    let _ = screen(&app, 100, 28);
    app
}

#[test]
fn the_wheel_goes_back_in_the_log_three_rows_a_notch_and_forward_again() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = filled(&mut game, &catalog);
    assert_eq!(app.scroll.get(), 0);
    app.on_wheel(true);
    assert_eq!(app.scroll.get(), 3);
    app.on_wheel(true);
    assert_eq!(app.scroll.get(), 6);
    assert!(has(&screen(&app, 100, 28), "Scrolled back: 6 lines below"));
    app.on_wheel(false);
    app.on_wheel(false);
    app.on_wheel(false);
    assert_eq!(app.scroll.get(), 0, "never below the end of the log");
    assert!(!has(&screen(&app, 100, 28), "Scrolled back"));
}

#[test]
fn the_wheel_never_scrolls_past_the_start_of_the_log() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = filled(&mut game, &catalog);
    for _ in 0..500 {
        app.on_wheel(true);
    }
    // The drawing clamps what was asked: the indicator is the top of the log, not the request.
    let rows = screen(&app, 100, 28);
    assert!(rows.iter().all(|row| row.width() <= 100));
    assert!(app.scroll.get() < 500 * 3, "{}", app.scroll.get());
}

#[test]
fn a_cut_screen_takes_the_wheel_before_the_log() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, None, "Neon");
    type_line(&mut app, "help");
    type_line(&mut app, "quests");
    // A short terminal cuts nothing of the journal: the wheel is the log's.
    let _ = screen(&app, 100, 28);
    app.on_wheel(true);
    assert_eq!(app.scroll.get(), 3);
    // Cut: the screen turns its own pages and the log stays where it was.
    app.scroll.set(0);
    let modal = app.modal.as_ref().unwrap();
    modal.overflow.set(true);
    modal.scroll.set(6);
    app.on_wheel(true);
    assert_eq!(app.modal.as_ref().unwrap().scroll.get(), 3);
    assert_eq!(app.scroll.get(), 0, "the log did not move");
    app.on_wheel(false);
    assert_eq!(app.modal.as_ref().unwrap().scroll.get(), 6);
}

#[test]
fn the_wheel_does_nothing_when_nothing_can_be_seen() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = filled(&mut game, &catalog);
    let _ = screen(&app, 50, 15);
    app.on_wheel(true);
    assert_eq!(app.scroll.get(), 0);
}
