//! The side panel: what it lists, how it is hidden and shown (F2, `panel`), the drawer of the
//! compact layout and the order Escape closes things in.

use neon_engine::command::Capabilities;

use super::snapshots::{campaign, campaign_app, picture};
use super::*;

/// The campaign past the opening, run by the full-screen interface (so `panel` is open).
fn panel_app<'a>(game: &'a mut CampaignGame, catalog: &'a Catalog, mode: RenderMode) -> App<'a> {
    game.set_capabilities(Capabilities::TUI);
    campaign_app(game, catalog, mode, None, "Neon").with_panel_command()
}

use neon_engine::campaign::CampaignGame;

#[test]
fn the_full_panel_lists_quests_net_and_inventory_under_their_headings() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = panel_app(&mut game, &catalog, RenderMode::FULL);
    let rows = screen(&app, 100, 28);
    for heading in ["▸ Quests", "▸ The net", "▸ Inventory"] {
        assert!(has(&rows, heading), "{heading}: {rows:#?}");
    }
    assert!(has(&rows, "Credits: "), "the credits are in the inventory");
    // Nothing is held and no site is known yet: the lists say so.
    assert!(has(&rows, "No item yet."), "{rows:#?}");
    // The panel takes what the log leaves, never more than its width.
    assert!(rows.iter().all(|row| row.width() <= 100));
    press(&mut app, KeyCode::F(2));
    assert!(!has(&screen(&app, 100, 28), "▸ Quests"));
}

#[test]
fn the_panel_is_in_french_too_and_in_ascii() {
    let (catalog, mut game) = (catalog_fr(), campaign());
    let app = panel_app(&mut game, &catalog, RenderMode::FULL);
    let rows = screen(&app, 100, 28);
    for heading in ["▸ Quêtes", "▸ Le réseau", "▸ Inventaire"] {
        assert!(has(&rows, heading), "{heading}: {rows:#?}");
    }
    let (catalog, mut game) = (catalog_en(), campaign());
    let app = panel_app(&mut game, &catalog, RenderMode::ASCII);
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "> Quests") && !has(&rows, "▸"), "{rows:#?}");
}

#[test]
fn f2_hides_the_permanent_panel_and_shows_it_again() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = panel_app(&mut game, &catalog, RenderMode::FULL);
    assert!(has(&screen(&app, 100, 28), "Panel"));
    press(&mut app, KeyCode::F(2));
    let hidden = screen(&app, 100, 28);
    assert!(
        !has(&hidden, "Panel") && !has(&hidden, "Inventory"),
        "{hidden:#?}"
    );
    // The log takes the room: its frame reaches the right edge.
    assert!(hidden[1].ends_with('╮'), "{:?}", hidden[1]);
    press(&mut app, KeyCode::F(2));
    assert!(has(&screen(&app, 100, 28), "Inventory"));
}

#[test]
fn the_compact_layout_has_no_panel_until_the_drawer_is_opened() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = panel_app(&mut game, &catalog, RenderMode::FULL);
    assert!(!has(&screen(&app, 64, 20), "Inventory"));
    // F2 means what the last drawing showed: the screen is drawn before every key.
    press(&mut app, KeyCode::F(2));
    let rows = screen(&app, 64, 20);
    assert!(has(&rows, "Inventory") && has(&rows, "Panel"), "{rows:#?}");
    // The drawer is over the log: the input line and the help are still there.
    assert!(input_row(&rows).starts_with('>'));
    assert!(rows.last().is_some_and(|row| row.contains("F2 panel")));
    press(&mut app, KeyCode::F(2));
    assert!(!has(&screen(&app, 64, 20), "Inventory"));
}

#[test]
fn escape_closes_the_drawer_before_anything_else() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = panel_app(&mut game, &catalog, RenderMode::FULL);
    for _ in 0..4 {
        type_line(&mut app, "help");
    }
    let _ = screen(&app, 64, 20);
    press(&mut app, KeyCode::PageUp);
    type_text(&mut app, "sta");
    press(&mut app, KeyCode::F(2));
    let _ = screen(&app, 64, 20);
    assert!(app.panel.drawer);
    // 1: the drawer, with the line and the scroll kept.
    press(&mut app, KeyCode::Esc);
    let _ = screen(&app, 64, 20);
    assert!(!app.panel.drawer && app.scroll.get() > 0 && !app.editor.is_empty());
    // 2: the scroll. 3: the line.
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.scroll.get(), 0);
    assert!(!app.editor.is_empty());
    press(&mut app, KeyCode::Esc);
    assert!(app.editor.is_empty());
}

#[test]
fn the_panel_command_shows_and_hides_the_panel_and_says_so() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = panel_app(&mut game, &catalog, RenderMode::FULL);
    let _ = screen(&app, 100, 28);
    type_line(&mut app, "panel");
    let rows = screen(&app, 100, 28);
    assert!(
        has(&rows, "Panel hidden.") && !has(&rows, "Inventory"),
        "{rows:#?}"
    );
    type_line(&mut app, "PANEL");
    let rows = screen(&app, 100, 28);
    assert!(
        has(&rows, "Panel shown.") && has(&rows, "Inventory"),
        "{rows:#?}"
    );
    // At the compact layout it is the drawer.
    let _ = screen(&app, 64, 20);
    type_line(&mut app, "panel");
    let rows = screen(&app, 64, 20);
    assert!(has(&rows, "Inventory"), "{rows:#?}");
}

#[test]
fn the_panel_command_is_the_games_when_the_interface_does_not_run_it() {
    let (catalog, mut game) = (catalog_en(), campaign());
    // Not told that the interface is the full-screen one: the game says the command is not for
    // this interface, and the panel is untouched.
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, None, "Neon");
    let _ = screen(&app, 100, 28);
    type_line(&mut app, "panel");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "Only in full screen."), "{rows:#?}");
    assert!(has(&rows, "Inventory"));
}

#[test]
fn a_panel_too_short_for_its_rows_says_how_many_it_leaves_out() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let app = panel_app(&mut game, &catalog, RenderMode::FULL);
    let rows = screen(&app, 100, 28);
    assert!(!has(&rows, "more rows"), "{rows:#?}");
    // The drawer at the smallest size has less room than the campaign has rows.
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = panel_app(&mut game, &catalog, RenderMode::FULL);
    press(&mut app, KeyCode::F(2));
    let rows = screen(&app, 64, 20);
    assert!(
        has(&rows, "Inventory") || has(&rows, "more row"),
        "{rows:#?}"
    );
}

#[test]
fn the_panel_in_pictures() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = panel_app(&mut game, &catalog, RenderMode::FULL);
    insta::assert_snapshot!("full_en", picture(&app, 100, 28));
    press(&mut app, KeyCode::F(2));
    insta::assert_snapshot!("hidden_en", picture(&app, 100, 28));
    let _ = screen(&app, 64, 20);
    press(&mut app, KeyCode::F(2));
    insta::assert_snapshot!("drawer_en", picture(&app, 64, 20));
    let (catalog, mut game) = (catalog_fr(), campaign());
    let app = panel_app(&mut game, &catalog, RenderMode::ASCII);
    insta::assert_snapshot!("full_fr_ascii", picture(&app, 100, 28));
}
