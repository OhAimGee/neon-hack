//! Saves written by published versions of the game must keep loading.
//!
//! `fixtures/demo_save_v1.toml` is a real version-1 save, copied once and never edited. If a
//! change makes this test fail, the format changed: raise `SAVE_VERSION`, add a migration,
//! and freeze a new file for the new version, keeping this one.

use neon_engine::demo::DemoGame;
use neon_engine::game::Game;
use neon_engine::prompt::Prompt;
use neon_engine::save::{SAVE_VERSION, peek};

const V1: &str = include_str!("fixtures/demo_save_v1.toml");

#[test]
fn the_frozen_version_1_save_still_loads_with_the_state_it_had() {
    let meta = peek(V1).unwrap();
    assert_eq!((meta.player.as_str(), meta.turn), ("Neon", 3));

    let game = DemoGame::from_save(V1).unwrap();
    let view = game.view();
    assert_eq!(view.player, "Neon");
    assert_eq!((view.gauges[0].value, view.gauges[0].max), (30, 100));
    // It was saved inside the stall, with the proxy bought: the menu is back, entry 1 owned.
    let Prompt::Choice(choice) = game.prompt() else {
        panic!("expected the stall menu, got {:?}", game.prompt());
    };
    assert!(choice.options[0].available.is_err(), "the proxy is owned");
}

#[test]
fn writing_the_frozen_save_again_gives_the_same_text() {
    let game = DemoGame::from_save(V1).unwrap();
    assert_eq!(game.snapshot().unwrap(), V1);
}

#[test]
fn a_frozen_file_exists_for_every_version_up_to_the_current_one() {
    // Versions are published one by one: v1 is frozen above, and the current version must
    // not run ahead of the frozen files without a migration (see the unit test on MIGRATIONS).
    assert_eq!(SAVE_VERSION, 1);
}
