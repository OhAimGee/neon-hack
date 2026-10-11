//! Handing the game over to the line-by-line display: `Ctrl+P`, the `plain` command, and what
//! the interface gives back.

use neon_engine::campaign::CampaignGame;
use neon_engine::command::Capabilities;

use super::snapshots::{campaign, campaign_app};
use super::*;

fn plain_app<'a>(game: &'a mut CampaignGame, catalog: &'a Catalog, mode: RenderMode) -> App<'a> {
    game.set_capabilities(Capabilities::TUI);
    campaign_app(game, catalog, mode, None, "Neon").with_frontend_commands()
}

#[test]
fn control_p_asks_for_the_plain_display_and_does_not_touch_the_game() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = plain_app(&mut game, &catalog, RenderMode::FULL);
    let _ = screen(&app, 100, 28);
    type_line(&mut app, "quests");
    assert_eq!(app.exit(), None);
    ctrl(&mut app, 'p');
    assert_eq!(app.exit(), Some(Exit::Plain));
}

#[test]
fn the_plain_command_asks_for_it_whatever_its_case() {
    for line in ["plain", "  PLAIN ", "Plain"] {
        let (catalog, mut game) = (catalog_en(), campaign());
        let mut app = plain_app(&mut game, &catalog, RenderMode::FULL);
        type_line(&mut app, line);
        assert_eq!(app.exit(), Some(Exit::Plain), "{line:?}");
    }
}

#[test]
fn plain_is_a_word_like_any_other_where_the_interface_does_not_run_it() {
    // The demo game has no `plain`, and an interface told nothing about it does not run it:
    // the line goes to the game, which says it does not know it.
    let mut game = DemoGame::new(1);
    let catalog = catalog_en();
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_line(&mut app, "plain");
    assert_eq!(app.exit(), None);
    assert!(has(&screen(&app, 100, 28), "> plain"));
}

#[test]
fn an_answer_that_happens_to_be_plain_is_an_answer() {
    // The handle prompt takes any text; only the command prompt knows `plain`.
    let (catalog, mut game) = (catalog_en(), campaign());
    game.set_capabilities(Capabilities::TUI);
    let first = game.start();
    let mut app = App::new(
        &mut game,
        Renderer {
            catalog: &catalog,
            mode: RenderMode::FULL,
            verbosity: Verbosity::Normal,
            colors: None,
        },
        first,
        Persistence::disabled(),
    )
    .with_frontend_commands();
    type_line(&mut app, "plain");
    assert_eq!(app.exit(), None);
    assert!(has(&screen(&app, 100, 28), "Welcome to the net, plain."));
}

#[test]
fn a_terminal_too_small_to_play_can_still_hand_over() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = plain_app(&mut game, &catalog, RenderMode::FULL);
    let _ = screen(&app, 50, 15);
    // Nothing is typed into the game from there, but the way out works.
    type_text(&mut app, "quests");
    assert_eq!(app.exit(), None);
    ctrl(&mut app, 'p');
    assert_eq!(app.exit(), Some(Exit::Plain));
}

#[test]
fn the_game_comes_out_as_it_went_in() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = plain_app(&mut game, &catalog, RenderMode::FULL);
    type_line(&mut app, "quests");
    type_line(&mut app, "net");
    ctrl(&mut app, 'p');
    let (game, renderer, options, _persistence) = app.into_parts();
    // The same player, the same quest, and the step to attach the next frontend says where
    // the game is without replaying anything.
    assert_eq!(game.view().player, "Neon");
    let step = game.resume();
    assert_eq!(step.prompt, Prompt::Command);
    assert!(!step.events.is_empty());
    assert!(options.is_none());
    assert_eq!(renderer.mode, RenderMode::FULL);
}

#[test]
fn after_the_end_every_key_leaves_for_good() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = plain_app(&mut game, &catalog, RenderMode::FULL);
    ctrl(&mut app, 'd');
    assert!(app.finished);
    ctrl(&mut app, 'p');
    assert_eq!(
        app.exit(),
        Some(Exit::Quit),
        "the game is over: nothing to hand over"
    );
}
