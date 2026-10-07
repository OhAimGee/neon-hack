use neon_engine::demo::DemoGame;
use neon_engine::text::{Catalog, RenderMode};
use proptest::prelude::*;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::persist::Persistence;
use crate::render::Verbosity;
use crate::test_support::{catalog_en, catalog_fr};

fn app_with<'a>(game: &'a mut DemoGame, catalog: &'a Catalog, mode: RenderMode) -> App<'a> {
    let first = game.start();
    App::new(
        game,
        Renderer {
            catalog,
            mode,
            verbosity: Verbosity::Normal,
        },
        first,
        Persistence::disabled(),
    )
}

/// What the screen shows, one string per row, trailing spaces removed.
fn screen(app: &App<'_>, width: u16, height: u16) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| app.draw(frame)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

fn press(app: &mut App<'_>, code: KeyCode) {
    app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn type_line(app: &mut App<'_>, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c));
    }
    press(app, KeyCode::Enter);
}

/// Goes through the prologue: continue, a handle, confirmation.
fn reach_command_line(app: &mut App<'_>) {
    press(app, KeyCode::Enter);
    type_line(app, "Neon");
    type_line(app, "y");
}

fn has(rows: &[String], needle: &str) -> bool {
    rows.iter().any(|row| row.contains(needle))
}

#[test]
fn the_terminal_size_decides_how_much_of_the_interface_is_shown() {
    let tier_of = |width, height| tier(Rect::new(0, 0, width, height));
    assert_eq!(tier_of(100, 28), Tier::Full);
    assert_eq!(tier_of(200, 60), Tier::Full);
    assert_eq!(tier_of(99, 28), Tier::Compact);
    assert_eq!(tier_of(100, 27), Tier::Compact);
    assert_eq!(tier_of(64, 20), Tier::Compact);
    assert_eq!(tier_of(63, 20), Tier::TooSmall);
    assert_eq!(tier_of(64, 19), Tier::TooSmall);
}

#[test]
fn the_full_layout_has_a_status_bar_a_log_a_panel_and_an_input_line() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_line(&mut app, "scan");
    let rows = screen(&app, 100, 28);

    assert!(
        rows[0].contains("Neon") && rows[0].contains("Trace"),
        "{:?}",
        rows[0]
    );
    assert!(
        rows[0].contains("/100 ("),
        "the level is said in words: {:?}",
        rows[0]
    );
    assert!(has(&rows, "> scan"), "the typed line is echoed in the log");
    assert!(has(&rows, "[Reward] +"));
    assert!(has(&rows, "Trace +"), "the gauge change is in the log too");
    // Side panel: the gauge with its number, a bar and the level, then the objective.
    assert!(has(&rows, "Trace ") && has(&rows, "/100"));
    assert!(has(&rows, "[█") || has(&rows, "[░"), "a bar is drawn");
    assert!(has(&rows, "Keep your trace under 70."));
    assert_eq!(rows[27], ">", "the input line is the last row");
}

#[test]
fn the_compact_layout_drops_the_panel_but_keeps_the_gauge_in_words() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    let rows = screen(&app, 64, 20);
    assert!(rows[0].contains("Trace 0/100 (calm)"), "{:?}", rows[0]);
    assert!(
        !has(&rows, "Keep your trace under"),
        "no panel in the compact layout"
    );
}

#[test]
fn a_terminal_that_is_too_small_says_so_in_the_current_language() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let app = app_with(&mut game, &catalog, RenderMode::FULL);
    let rows = screen(&app, 63, 20);
    assert!(
        has(&rows, "Terminal too small (63x20): at least 64x20 needed."),
        "{rows:?}"
    );
    let (catalog, mut game) = (catalog_fr(), DemoGame::new(1));
    let app = app_with(&mut game, &catalog, RenderMode::FULL);
    assert!(has(&screen(&app, 63, 20), "Terminal trop petit (63x20)"));
}

#[test]
fn no_terminal_size_can_make_the_interface_panic() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_line(&mut app, "shop");
    for width in [1, 2, 3, 10, 30, 63, 64, 65, 99, 100, 150] {
        for height in [1, 2, 3, 8, 19, 20, 21, 27, 28, 40] {
            let _ = screen(&app, width, height);
        }
    }
}

#[test]
fn a_menu_is_listed_above_the_input_with_its_way_out() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_line(&mut app, "shop");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "[1] Proxy chain (30 credits)"));
    assert!(has(
        &rows,
        "[2] Cloak module (60 credits) (10 credits short.)"
    ));
    assert!(has(&rows, "[0] Leave the stall"));
    assert_eq!(rows[27], "?", "the menu marker is on the input line");
}

#[test]
fn playing_by_keyboard_goes_through_the_whole_prologue() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    assert!(has(&screen(&app, 100, 28), "[Enter] to continue"));
    press(&mut app, KeyCode::Enter);
    assert!(has(&screen(&app, 100, 28), "Your handle [Case]:"));
    type_line(&mut app, "Neon");
    assert!(has(
        &screen(&app, 100, 28),
        "Carve \"Neon\" into the net? [Y/n]"
    ));
    type_line(&mut app, "y");
    let rows = screen(&app, 100, 28);
    // The log is 68 columns wide here, so the sentence wraps: check its beginning and its end.
    assert!(has(
        &rows,
        "ECHO-7 » Finally. I was starting to think you would never wake up,"
    ));
    assert!(has(&rows, "Neon."));
    assert!(has(&rows, "ECHO-7 » Type 'help' to see what you can do."));
}

#[test]
fn a_bad_yes_or_no_is_refused_on_the_spot() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    press(&mut app, KeyCode::Enter);
    type_line(&mut app, "Neon");
    type_line(&mut app, "perhaps");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "[Error] Please answer yes or no."));
    assert!(
        has(&rows, "Carve \"Neon\" into the net?"),
        "still waiting for the answer"
    );
}

#[test]
fn escape_clears_a_typed_line_then_backs_out_of_a_menu() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_line(&mut app, "shop");
    for c in "xy".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Backspace);
    assert_eq!(screen(&app, 100, 28)[27], "? x");
    press(&mut app, KeyCode::Esc);
    let rows = screen(&app, 100, 28);
    assert_eq!(rows[27], "?", "the first Escape only clears the line");
    assert!(has(&rows, "[0] Leave the stall"), "the menu is still open");
    press(&mut app, KeyCode::Esc);
    assert_eq!(
        screen(&app, 100, 28)[27],
        ">",
        "the second one leaves the menu"
    );
}

#[test]
fn control_d_ends_the_game_and_the_next_key_leaves() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    app.on_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
    assert!(!app.should_quit());
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "Input closed: ending the session."));
    assert!(has(&rows, "Press any key to leave."));
    press(&mut app, KeyCode::Char(' '));
    assert!(app.should_quit());
}

#[test]
fn control_c_also_ends_cleanly_and_other_control_keys_are_not_text() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    app.on_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    assert_eq!(screen(&app, 100, 28)[27], ">", "Ctrl+A types nothing");
    app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert!(has(&screen(&app, 100, 28), "Press any key to leave."));
}

#[test]
fn altgr_characters_are_text() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    // Windows reports AltGr as Control+Alt.
    app.on_key(KeyEvent::new(
        KeyCode::Char('@'),
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    ));
    assert_eq!(screen(&app, 100, 28)[27], "> @");
    app.on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT));
    assert_eq!(
        screen(&app, 100, 28)[27],
        "> @",
        "Alt alone is a shortcut, not text"
    );
}

#[test]
fn tab_completes_like_a_shell() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    press(&mut app, KeyCode::Char('s'));
    press(&mut app, KeyCode::Tab);
    assert_eq!(
        screen(&app, 100, 28)[27],
        "> s",
        "scan, shop and status share only `s`"
    );
    press(&mut app, KeyCode::Char('h'));
    press(&mut app, KeyCode::Tab);
    assert_eq!(
        screen(&app, 100, 28)[27],
        "> shop",
        "one candidate: completed and spaced"
    );
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('z'));
    press(&mut app, KeyCode::Tab);
    assert_eq!(
        screen(&app, 100, 28)[27],
        "> z",
        "no candidate: nothing changes"
    );
}

#[test]
fn ascii_mode_draws_the_bar_and_separators_with_ascii_only() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::ASCII);
    reach_command_line(&mut app);
    type_line(&mut app, "scan");
    let rows = screen(&app, 100, 28);
    assert!(rows[0].contains(" | "), "{:?}", rows[0]);
    for row in &rows {
        assert!(row.is_ascii(), "not ASCII: {row:?}");
    }
    assert!(has(&rows, "[#") || has(&rows, "[."));
}

#[test]
fn wrapping_cuts_at_spaces_and_inside_words_only_when_it_must() {
    assert_eq!(wrap("one two three", 7), ["one two", "three"]);
    assert_eq!(wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
    assert_eq!(wrap("", 10), [""]);
    assert_eq!(
        wrap("日本語", 4),
        ["日本", "語"],
        "wide characters take two columns"
    );
    assert_eq!(
        wrap("a b", 0),
        ["a", "b"],
        "a zero width still makes progress"
    );
}

proptest! {
    #[test]
    fn wrapped_rows_fit_and_lose_nothing(text in "[a-z 日本語]{0,60}", width in 2usize..40) {
        let rows = wrap(&text, width);
        for row in &rows {
            prop_assert!(row.width() <= width, "{row:?} wider than {width}");
        }
        let kept: String = rows.concat().chars().filter(|c| *c != ' ').collect();
        let original: String = text.chars().filter(|c| *c != ' ').collect();
        prop_assert_eq!(kept, original);
    }
}

#[test]
fn an_interface_attached_to_a_game_under_way_does_not_replay_the_start() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    game.start();
    for input in [
        Input::Continue,
        Input::Line("Neon".to_owned()),
        Input::Confirm(true),
    ] {
        game.handle(input);
    }
    let attach = game.resume();
    let renderer = Renderer {
        catalog: &catalog,
        mode: RenderMode::FULL,
        verbosity: Verbosity::Full,
    };
    let app = App::new(&mut game, renderer, attach, Persistence::disabled());
    let rows = screen(&app, 100, 28);
    assert!(
        !has(&rows, "NEON HACK") && !has(&rows, "N E O N"),
        "no logo"
    );
    assert!(!has(&rows, "Neo-Tokyo"), "no intro");
    assert_eq!(rows[27], ">", "straight to the command line");
    assert!(
        rows[0].contains("Neon"),
        "the player's state is shown: {:?}",
        rows[0]
    );
}

#[test]
fn ascii_mode_shows_what_is_typed_as_ascii_and_keeps_every_cell_ascii() {
    let (catalog, mut game) = (catalog_fr(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::ASCII);
    press(&mut app, KeyCode::Enter);
    for c in "Zoë".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    let rows = screen(&app, 100, 28);
    assert_eq!(rows[27], "Votre pseudo [Case] : Zoe", "while typing");
    press(&mut app, KeyCode::Enter);
    type_line(&mut app, "o");
    let rows = screen(&app, 100, 28);
    assert!(
        has(&rows, "Zoe"),
        "the handle is shown in the log: {rows:#?}"
    );
    for row in &rows {
        assert!(row.is_ascii(), "not ASCII: {row:?}");
    }
}
