//! Saves written by published versions of the game must keep loading.
//!
//! `fixtures/demo_save_v1.toml` is a real version-1 save, copied once and never edited. If a
//! change makes this test fail, the format changed: raise `SAVE_VERSION`, add a migration,
//! and freeze a new file for the new version, keeping this one.

use neon_engine::campaign::CampaignGame;
use neon_engine::demo::DemoGame;
use neon_engine::game::Game;
use neon_engine::prompt::Prompt;
use neon_engine::save::{SAVE_VERSION, peek};

const V1: &str = include_str!("fixtures/demo_save_v1.toml");
const CAMPAIGN_V1: &str = include_str!("fixtures/campaign_save_v1.toml");

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

/// `fixtures/campaign_save_v1.toml` was written by the campaign at the end of lot R2.2b (three
/// quests in, a conversation held, a hint used, the Stealth Module bought) and is never edited.
/// The campaign had no published save before it, so version 1 starts here: if a change makes
/// these tests fail, the campaign format changed, and it needs a migration.
#[test]
fn the_frozen_campaign_save_still_loads_with_the_state_it_had() {
    let meta = peek(CAMPAIGN_V1).unwrap();
    assert_eq!((meta.player.as_str(), meta.turn), ("Neon", 7));

    let game = CampaignGame::from_save(CAMPAIGN_V1).unwrap();
    assert_eq!(game.prompt(), Prompt::Command);
    assert_eq!(game.credits(), 405 - 150);
    assert_eq!(game.notoriety(), 12);
    assert_eq!(game.clock(), 7);
    let state = game.state();
    assert_eq!(state.handle(), "Neon");
    assert_eq!(state.difficulty(), neon_engine::campaign::Difficulty::Normal);
    // The resumed game says where it stands, without the welcome.
    let step = game.resume();
    assert_eq!(step.prompt, Prompt::Command);
    assert!(!step.events.is_empty());
}

#[test]
fn writing_the_frozen_campaign_save_again_gives_the_same_text() {
    let game = CampaignGame::from_save(CAMPAIGN_V1).unwrap();
    assert_eq!(game.snapshot().unwrap(), CAMPAIGN_V1);
}

#[test]
fn the_two_games_do_not_read_each_others_saves() {
    assert!(CampaignGame::from_save(V1).is_err());
    assert!(DemoGame::from_save(CAMPAIGN_V1).is_err());
}
