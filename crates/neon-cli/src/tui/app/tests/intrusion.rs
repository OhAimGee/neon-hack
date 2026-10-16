//! The panel of an intrusion in the full-screen interface: the sections the game lists, the
//! status bar, the drawer of the compact layout, in both languages and in the modes.

use neon_engine::campaign::Difficulty;
use neon_engine::intrusion::IntrusionGame;

use super::snapshots::picture;
use super::*;

fn game() -> IntrusionGame {
    IntrusionGame::new("corp-server-01", Difficulty::Normal).unwrap()
}

fn intrusion_app<'a>(
    game: &'a mut IntrusionGame,
    catalog: &'a Catalog,
    mode: RenderMode,
) -> App<'a> {
    let first = game.start();
    App::new(
        game,
        Renderer {
            catalog,
            mode,
            verbosity: Verbosity::Normal,
            colors: None,
        },
        first,
        Persistence::disabled(),
    )
}

/// An intrusion with a few turns played: a move, a breach, a utility, the end of a turn.
fn play(app: &mut App<'_>) {
    type_line(app, "move 3");
    type_line(app, "breach 4 brute");
    type_line(app, "use cloak");
}

#[test]
fn the_full_panel_lists_the_nodes_the_patrols_the_forecast_and_the_deck() {
    let (catalog, mut game) = (catalog_en(), game());
    let mut app = intrusion_app(&mut game, &catalog, RenderMode::FULL);
    play(&mut app);
    let rows = screen(&app, 100, 28);
    for heading in ["▸ Nodes", "▸ Patrols", "▸ Forecast", "▸ Deck"] {
        assert!(has(&rows, heading), "{heading}: {rows:#?}");
    }
    // The player's node is marked, the others say where they stand: the state is a word or the
    // defense, never a colour.
    assert!(has(&rows, "▶3 Archives here"), "{rows:#?}");
    assert!(
        has(&rows, "Network 2/4"),
        "the progress of the Vault: {rows:#?}"
    );
    assert!(has(&rows, "Patrol 1: 5 then 3"), "{rows:#?}");
    assert!(has(&rows, "Cloak is active."), "{rows:#?}");
    assert!(has(&rows, "End of turn: Trace +2"), "{rows:#?}");
    assert!(has(&rows, "5 Cloak (1)"), "{rows:#?}");
    assert!(rows.iter().all(|row| row.width() <= 100));
}

#[test]
fn the_status_bar_carries_the_trace_the_turn_and_the_cycles_and_follows_the_game() {
    let (catalog, mut game) = (catalog_en(), game());
    let mut app = intrusion_app(&mut game, &catalog, RenderMode::FULL);
    assert_eq!(
        screen(&app, 100, 28)[0],
        " corp-server-01 │ Trace 0/100 (calm) │ Turn 1 │ Cycles 3/3"
    );
    type_line(&mut app, "breach 2 brute");
    assert_eq!(
        screen(&app, 100, 28)[0],
        " corp-server-01 │ Trace 5/100 (calm) │ Turn 1 │ Cycles 2/3"
    );
    type_line(&mut app, "end");
    assert!(screen(&app, 100, 28)[0].contains("Turn 2 │ Cycles 3/3"));
}

#[test]
fn the_panel_is_drawn_from_the_game_again_after_each_command() {
    let (catalog, mut game) = (catalog_en(), game());
    let mut app = intrusion_app(&mut game, &catalog, RenderMode::FULL);
    assert!(has(&screen(&app, 100, 28), "End of turn: Trace +7"));
    type_line(&mut app, "use cloak");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "End of turn: Trace +2"), "{rows:#?}");
    assert!(has(&rows, "Patrol 1: cancelled"), "{rows:#?}");
}

#[test]
fn the_compact_layout_has_the_panel_as_a_drawer() {
    let (catalog, mut game) = (catalog_en(), game());
    let mut app = intrusion_app(&mut game, &catalog, RenderMode::FULL);
    play(&mut app);
    let closed = screen(&app, 64, 20);
    assert!(!has(&closed, "▸ Nodes"), "{closed:#?}");
    // The bar still says the three numbers.
    assert!(
        has(&closed, "Trace 5/100 (calm) │ Turn 1 │ Cycles 0/3"),
        "{closed:#?}"
    );
    press(&mut app, KeyCode::F(2));
    let open = screen(&app, 64, 20);
    assert!(has(&open, "▸ Nodes"), "{open:#?}");
    press(&mut app, KeyCode::Esc);
    assert!(!has(&screen(&app, 64, 20), "▸ Nodes"));
}

#[test]
fn the_panel_in_the_wide_tier_in_english() {
    let (catalog, mut game) = (catalog_en(), game());
    let mut app = intrusion_app(&mut game, &catalog, RenderMode::FULL);
    play(&mut app);
    insta::assert_snapshot!(picture(&app, 100, 28));
}

#[test]
fn the_panel_in_the_wide_tier_in_french() {
    let (catalog, mut game) = (catalog_fr(), game());
    let mut app = intrusion_app(&mut game, &catalog, RenderMode::FULL);
    play(&mut app);
    insta::assert_snapshot!(picture(&app, 100, 28));
}

#[test]
fn the_drawer_in_the_compact_tier_in_english() {
    let (catalog, mut game) = (catalog_en(), game());
    let mut app = intrusion_app(&mut game, &catalog, RenderMode::FULL);
    play(&mut app);
    press(&mut app, KeyCode::F(2));
    insta::assert_snapshot!(picture(&app, 64, 20));
}

#[test]
fn the_panel_in_ascii_is_seven_bits_and_keeps_its_words() {
    let (catalog, mut game) = (catalog_fr(), game());
    let mut app = intrusion_app(&mut game, &catalog, RenderMode::ASCII);
    play(&mut app);
    let rows = screen(&app, 100, 28);
    assert!(rows.iter().all(|row| row.is_ascii()), "{rows:#?}");
    assert!(has(&rows, ">3 Archives ici"), "{rows:#?}");
    insta::assert_snapshot!(picture(&app, 100, 28));
}

#[test]
fn every_size_language_and_mode_stays_in_the_cells_of_the_terminal() {
    for (catalog, lang) in [(catalog_en(), "en"), (catalog_fr(), "fr")] {
        for mode in [
            RenderMode::FULL,
            RenderMode::ASCII,
            RenderMode::SCREEN_READER,
        ] {
            let mut game = game();
            let mut app = intrusion_app(&mut game, &catalog, mode);
            play(&mut app);
            type_line(&mut app, "end");
            for (width, height) in [(100, 28), (64, 20), (63, 19), (40, 12), (200, 60)] {
                for drawer in [false, true] {
                    let rows = screen(&app, width, height);
                    assert_eq!(rows.len(), usize::from(height));
                    assert!(
                        rows.iter().all(|row| row.width() <= usize::from(width)),
                        "{lang} {width}x{height}: {rows:#?}"
                    );
                    if drawer {
                        press(&mut app, KeyCode::F(2));
                    }
                }
            }
        }
    }
}
