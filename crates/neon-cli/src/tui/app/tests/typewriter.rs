//! The typewriter: the text of a step appears little by little, any key shows the rest, and
//! nothing about it changes what the game says.

use std::time::Duration;

use super::snapshots::{campaign, campaign_app};
use super::*;
use crate::config::TypewriterChoice;
use crate::tui::typing::MAX_DURATION;

fn typing_app<'a>(
    game: &'a mut neon_engine::campaign::CampaignGame,
    catalog: &'a Catalog,
    choice: TypewriterChoice,
) -> App<'a> {
    campaign_app(game, catalog, RenderMode::FULL, None, "Neon")
        .with_motion(true)
        .with_typewriter(choice)
}

/// Lets `seconds` of typing go by, in the frames the real loop draws.
fn wait(app: &mut App<'_>, millis: u64) {
    let mut left = millis;
    while left > 0 {
        let step = left.min(50);
        app.tick(Duration::from_millis(step));
        left -= step;
    }
}

#[test]
fn what_the_game_says_appears_little_by_little_then_whole() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = typing_app(&mut game, &catalog, TypewriterChoice::Normal);
    type_line(&mut app, "status");
    // The echo of the player's own line is there at once; the answer has not started.
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "> status"), "{rows:#?}");
    assert!(!has(&rows, "Handle: Neon"), "{rows:#?}");
    assert!(app.is_animating());
    // 120 columns a second: a tenth of a second is a dozen columns.
    wait(&mut app, 100);
    let rows = screen(&app, 100, 28);
    assert!(
        has(&rows, "Handle: Neo") || has(&rows, "Handle: N"),
        "{rows:#?}"
    );
    assert!(!has(&rows, "Difficulty"), "{rows:#?}");
    // Then all of it, and the interface is still again.
    wait(&mut app, 2000);
    let rows = screen(&app, 100, 28);
    assert!(
        has(&rows, "Hints left for the current quest: 3"),
        "{rows:#?}"
    );
    assert!(!app.is_animating());
}

#[test]
fn any_key_shows_the_rest_at_once_and_is_then_typed() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = typing_app(&mut game, &catalog, TypewriterChoice::Slow);
    type_line(&mut app, "status");
    assert!(app.is_animating());
    press(&mut app, KeyCode::Char('h'));
    assert!(!app.is_animating());
    let rows = screen(&app, 100, 28);
    assert!(
        has(&rows, "Hints left for the current quest: 3"),
        "{rows:#?}"
    );
    // The key was not lost.
    assert_eq!(input_line(&app), "> h");
}

#[test]
fn a_long_answer_is_typed_faster_rather_than_longer() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = typing_app(&mut game, &catalog, TypewriterChoice::Slow);
    type_line(&mut app, "help");
    // 40 columns a second would take most of a minute for this table.
    wait(&mut app, u64::try_from(MAX_DURATION.as_millis()).unwrap());
    assert!(!app.is_animating());
    let rows = screen(&app, 100, 28);
    assert!(
        has(&rows, "tutorial - "),
        "the last row of the table is there: {rows:#?}"
    );
}

#[test]
fn off_types_nothing_and_reduced_motion_stops_it() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = typing_app(&mut game, &catalog, TypewriterChoice::Off);
    type_line(&mut app, "status");
    assert!(!app.is_animating());
    assert!(has(&screen(&app, 100, 28), "Hints left"));

    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = typing_app(&mut game, &catalog, TypewriterChoice::Normal).with_motion(false);
    type_line(&mut app, "status");
    assert!(
        !app.is_animating(),
        "reduced motion wins over the typewriter"
    );
}

#[test]
fn an_interface_not_told_about_the_typewriter_shows_everything_at_once() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, None, "Neon");
    type_line(&mut app, "status");
    assert!(!app.is_animating());
    assert!(has(&screen(&app, 100, 28), "Hints left"));
}

#[test]
fn the_typing_is_cut_by_columns_not_by_bytes() {
    // Accents and wide letters count by the columns they take, so nothing is split.
    let (catalog, mut game) = (catalog_fr(), campaign());
    let mut app = typing_app(&mut game, &catalog, TypewriterChoice::Slow);
    type_line(&mut app, "status");
    for _ in 0..60 {
        wait(&mut app, 50);
        for row in screen(&app, 100, 28) {
            assert!(row.width() <= 100, "{row:?}");
        }
    }
    assert!(!app.is_animating());
}

#[test]
fn a_scene_goes_first_then_its_result_is_typed() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = typing_app(&mut game, &catalog, TypewriterChoice::Normal);
    type_line(&mut app, "hack localhost");
    type_line(&mut app, "y");
    // The scene is playing and the result is held back.
    assert!(app.staging.is_some());
    assert!(app.typing.is_none());
    // When it is over by itself, the held result starts to be typed.
    wait(&mut app, 2000);
    assert!(app.staging.is_none());
    assert!(app.typing.is_some(), "the result is typed after the scene");
    wait(&mut app, 2500);
    assert!(!app.is_animating());
    assert!(has(&screen(&app, 100, 28), "1 file taken"));
}

#[test]
fn a_key_during_the_scene_skips_it_and_the_typing() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = typing_app(&mut game, &catalog, TypewriterChoice::Slow);
    type_line(&mut app, "hack localhost");
    type_line(&mut app, "y");
    assert!(app.staging.is_some());
    press(&mut app, KeyCode::Char('x'));
    assert!(!app.is_animating());
    assert!(has(&screen(&app, 100, 28), "1 file taken"));
}
