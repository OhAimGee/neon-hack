//! Replacing the game under way by a save: nothing is changed unless the save loads, and the
//! interface keeps the commands it runs.

use super::support::*;
use crate::campaign::CampaignGame;
use crate::command::Capabilities;
use crate::demo::DemoGame;
use crate::game::Game;
use crate::prompt::Prompt;
use crate::save::SaveError;

/// A game that has played the first quest, and its save.
fn played() -> (Driver, String) {
    let mut driver = Driver::new();
    driver.finish_m01();
    let text = driver.game.snapshot().unwrap();
    (driver, text)
}

#[test]
fn loading_puts_the_saved_game_in_place_of_the_one_under_way() {
    let (played, text) = played();
    let mut driver = Driver::new();
    assert_ne!(driver.game.state, played.game.state);
    let step = driver.game.load(&text).unwrap();
    assert_eq!(driver.game.state, played.game.state);
    // The step to carry on from says where the game is, and waits for a command.
    assert_eq!(step.prompt, Prompt::Command);
    let cat = catalog(crate::text::Lang::En);
    let said: String = step
        .events
        .iter()
        .map(|event| render_event(event, &cat))
        .collect();
    assert!(said.contains("Back on the net, Neon"), "{said}");
    assert_eq!(step.save, None, "loading is not a reason to save");
}

#[test]
fn a_save_that_does_not_load_changes_nothing() {
    let (_, text) = played();
    let mut driver = Driver::new();
    let before = driver.game.state.clone();
    for broken in [
        "not a save at all".to_owned(),
        text.replace("[end]", "[ended]"),
        String::new(),
    ] {
        assert!(driver.game.load(&broken).is_err(), "{broken:?}");
        assert_eq!(driver.game.state, before);
    }
    // The game goes on.
    assert!(driver.line("status").contains("Handle: Neon"));
}

#[test]
fn the_save_of_another_game_is_refused() {
    let demo = DemoGame::new(1).snapshot().unwrap();
    let mut driver = Driver::new();
    let before = driver.game.state.clone();
    assert!(matches!(
        driver.game.load(&demo),
        Err(SaveError::Schema(_) | SaveError::Invalid(_) | SaveError::Syntax(_))
    ));
    assert_eq!(driver.game.state, before);
}

#[test]
fn the_interface_keeps_the_commands_it_runs() {
    let (_, text) = played();
    let mut driver = Driver::new();
    driver.game.set_capabilities(Capabilities::TUI);
    driver.game.load(&text).unwrap();
    assert_eq!(driver.game.complete("pan"), ["panel"]);
    // A game that was never told anything stays a plain one.
    let mut plain = CampaignGame::from_save(&text).unwrap();
    plain.load(&text).unwrap();
    assert!(plain.complete("pan").is_empty());
}

#[test]
fn a_game_that_cannot_be_replaced_says_so() {
    // The default of the trait: the demo does not load saves while it runs.
    let mut demo = DemoGame::new(1);
    let (_, text) = played();
    assert!(demo.load(&text).is_err());
}
