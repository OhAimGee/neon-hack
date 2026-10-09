use neon_engine::demo::DemoGame;
use neon_engine::text::{Catalog, RenderMode};
use proptest::prelude::*;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};

use unicode_width::UnicodeWidthStr;

use super::draw::to_style;
use super::*;
use crate::palette::{Palette, PaletteChoice};
use crate::persist::Persistence;
use crate::render::{LineKind, Verbosity};
use crate::test_support::{catalog_en, catalog_fr};

mod snapshots;

fn app_with<'a>(game: &'a mut DemoGame, catalog: &'a Catalog, mode: RenderMode) -> App<'a> {
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

/// What the screen shows, one string per row, trailing spaces removed.
fn screen(app: &App<'_>, width: u16, height: u16) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| app.draw(frame)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            // A wide character covers the cell after its own.
            let mut row = String::new();
            let mut x = 0;
            while x < width {
                let symbol = buffer[(x, y)].symbol();
                row.push_str(symbol);
                x += u16::try_from(symbol.width().max(1)).unwrap();
            }
            row.trim_end().to_owned()
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

/// The input line: the row above the help bar, the last row.
fn input_row(rows: &[String]) -> &str {
    &rows[rows.len() - 2]
}

/// The input line of the screen at 100x28.
fn input_line(app: &App<'_>) -> String {
    input_row(&screen(app, 100, 28)).to_owned()
}

fn key(app: &mut App<'_>, code: KeyCode, modifiers: KeyModifiers) {
    app.on_key(KeyEvent::new(code, modifiers));
}

fn ctrl(app: &mut App<'_>, letter: char) {
    key(app, KeyCode::Char(letter), KeyModifiers::CONTROL);
}

fn type_text(app: &mut App<'_>, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c));
    }
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
    assert!(has(&rows, "Keep your trace under 100."));
    assert_eq!(
        input_row(&rows),
        ">",
        "the input line is above the help bar"
    );
    assert!(
        rows[27].contains("Tab complete") && rows[27].contains("Ctrl+C quit"),
        "the help bar is the last row: {:?}",
        rows[27]
    );
    assert!(
        rows[1].contains("Terminal") && rows[1].starts_with('╭'),
        "the log is boxed in the full layout: {:?}",
        rows[1]
    );
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
    // From nothing at all to a very large terminal, in a menu with a long line typed.
    type_text(&mut app, &"é日 ".repeat(40));
    for width in [0, 1, 2, 3, 10, 30, 63, 64, 65, 99, 100, 150, 300] {
        for height in [0, 1, 2, 3, 8, 19, 20, 21, 27, 28, 40, 100] {
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
    assert_eq!(
        input_row(&rows),
        "?",
        "the menu marker is on the input line"
    );
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
    assert_eq!(input_line(&app), "? x");
    press(&mut app, KeyCode::Esc);
    let rows = screen(&app, 100, 28);
    assert_eq!(
        input_row(&rows),
        "?",
        "the first Escape only clears the line"
    );
    assert!(has(&rows, "[0] Leave the stall"), "the menu is still open");
    press(&mut app, KeyCode::Esc);
    assert_eq!(input_line(&app), ">", "the second one leaves the menu");
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
fn control_c_asks_to_quit_like_the_quit_command_and_never_kills_the_process() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    ctrl(&mut app, 'a');
    assert_eq!(input_line(&app), ">", "Ctrl+A types nothing");
    ctrl(&mut app, 'c');
    assert!(!app.should_quit(), "a question first, like `quit`");
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "> quit"), "{rows:#?}");
    assert!(has(&rows, "[Y/n]") || has(&rows, "[y/N]"), "{rows:#?}");
    // Answering ends the game and the next key leaves.
    type_line(&mut app, "y");
    assert!(has(&screen(&app, 100, 28), "Press any key to leave."));
    press(&mut app, KeyCode::Char(' '));
    assert!(app.should_quit());
}

#[test]
fn control_c_in_a_menu_or_a_question_closes_the_input() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_line(&mut app, "shop");
    ctrl(&mut app, 'c');
    assert!(has(
        &screen(&app, 100, 28),
        "Input closed: ending the session."
    ));
    // Twice in a row also leaves the quit question of the command line.
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    ctrl(&mut app, 'c');
    ctrl(&mut app, 'c');
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
    assert_eq!(input_line(&app), "> @");
    app.on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT));
    assert_eq!(input_line(&app), "> @", "Alt alone is a shortcut, not text");
}

#[test]
fn tab_completes_like_a_shell() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    press(&mut app, KeyCode::Char('s'));
    press(&mut app, KeyCode::Tab);
    let rows = screen(&app, 100, 28);
    assert_eq!(
        input_row(&rows),
        "> s",
        "scan, shop and status share only `s`"
    );
    // What is still ambiguous is listed under the log.
    let listed = &rows[rows.len() - 3];
    assert!(
        listed.contains("scan") && listed.contains("shop") && listed.contains("status"),
        "{listed:?}"
    );
    press(&mut app, KeyCode::Char('h'));
    assert!(
        !has(&screen(&app, 100, 28), "status"),
        "the list goes with the next key"
    );
    press(&mut app, KeyCode::Tab);
    assert_eq!(input_line(&app), "> shop", "one candidate: completed");
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(input_line(&app), "> shop x", "and spaced");
    press(&mut app, KeyCode::Backspace);
    press(&mut app, KeyCode::Backspace);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('z'));
    press(&mut app, KeyCode::Tab);
    assert_eq!(input_line(&app), "> z", "no candidate: nothing changes");
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
        colors: None,
    };
    let app = App::new(&mut game, renderer, attach, Persistence::disabled());
    let rows = screen(&app, 100, 28);
    assert!(
        !has(&rows, "NEON HACK") && !has(&rows, "N E O N"),
        "no logo"
    );
    assert!(!has(&rows, "Neo-Tokyo"), "no intro");
    assert_eq!(input_row(&rows), ">", "straight to the command line");
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
    assert_eq!(
        input_line(&app),
        "Votre handle [Case] : Zoe",
        "while typing"
    );
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

// ---- Colour --------------------------------------------------------------------------------

fn app_colored<'a>(
    game: &'a mut DemoGame,
    catalog: &'a Catalog,
    colors: Option<Palette>,
) -> App<'a> {
    let first = game.start();
    App::new(
        game,
        Renderer {
            catalog,
            mode: RenderMode::FULL,
            verbosity: Verbosity::Normal,
            colors,
        },
        first,
        Persistence::disabled(),
    )
}

/// Every cell of the screen at a size.
fn cells(app: &App<'_>, width: u16, height: u16) -> Vec<ratatui::buffer::Cell> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| app.draw(frame)).unwrap();
    terminal.backend().buffer().content().to_vec()
}

const SIZES: [(u16, u16); 4] = [(100, 28), (80, 24), (64, 20), (40, 10)];

/// A game with enough history to show every kind of line: dialogue, rewards, gauges,
/// alerts and an error.
fn busy(app: &mut App<'_>) {
    reach_command_line(app);
    for _ in 0..8 {
        type_line(app, "scan");
    }
    type_line(app, "nonsense");
}

#[test]
fn without_colour_only_attributes_are_used_at_every_size() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(3));
    let mut app = app_colored(&mut game, &catalog, None);
    busy(&mut app);
    for (width, height) in SIZES {
        for cell in cells(&app, width, height) {
            assert_eq!(cell.fg, Color::Reset, "{width}x{height}: {cell:?}");
            assert_eq!(cell.bg, Color::Reset, "{width}x{height}: {cell:?}");
        }
    }
}

#[test]
fn nothing_ever_blinks_in_any_palette() {
    for choice in [
        PaletteChoice::Default,
        PaletteChoice::HighContrast,
        PaletteChoice::Cvd,
        PaletteChoice::Mono,
    ] {
        for truecolor in [false, true] {
            let (catalog, mut game) = (catalog_en(), DemoGame::new(3));
            let mut app = app_colored(&mut game, &catalog, Some(Palette::new(choice, truecolor)));
            busy(&mut app);
            for (width, height) in SIZES {
                for cell in cells(&app, width, height) {
                    let modifier = cell.modifier;
                    assert!(
                        !modifier.contains(Modifier::SLOW_BLINK)
                            && !modifier.contains(Modifier::RAPID_BLINK),
                        "{choice:?} {width}x{height}"
                    );
                }
            }
        }
    }
}

#[test]
fn an_opaque_palette_paints_its_background_under_every_cell() {
    for (choice, exact, plain) in [
        (
            PaletteChoice::HighContrast,
            Color::Rgb(0, 0, 0),
            Color::Black,
        ),
        (
            PaletteChoice::Cvd,
            Color::Rgb(0x10, 0x10, 0x10),
            Color::Black,
        ),
    ] {
        for truecolor in [true, false] {
            let (catalog, mut game) = (catalog_en(), DemoGame::new(3));
            let mut app = app_colored(&mut game, &catalog, Some(Palette::new(choice, truecolor)));
            busy(&mut app);
            let expected = if truecolor { exact } else { plain };
            for (width, height) in SIZES {
                for cell in cells(&app, width, height) {
                    // Reverse video swaps the colours on purpose (the status bar, danger).
                    if cell.modifier.contains(Modifier::REVERSED) {
                        continue;
                    }
                    assert_eq!(
                        cell.bg, expected,
                        "{choice:?} truecolor={truecolor} {width}x{height}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_default_palette_is_transparent_and_uses_the_terminals_named_colours() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(3));
    let colors = Palette::new(PaletteChoice::Default, true);
    let mut app = app_colored(&mut game, &catalog, Some(colors));
    busy(&mut app);
    let all = cells(&app, 100, 28);
    assert!(
        all.iter().all(|cell| cell.bg == Color::Reset),
        "the theme shows through"
    );
    let colours: Vec<Color> = all
        .iter()
        .map(|cell| cell.fg)
        .filter(|c| *c != Color::Reset)
        .collect();
    assert!(!colours.is_empty(), "something is coloured");
    assert!(
        colours.iter().all(|c| !matches!(c, Color::Rgb(..))),
        "even on a 24-bit terminal, the default palette stays with named colours"
    );
}

#[test]
fn the_cvd_palette_uses_exact_colours_only_on_a_24_bit_terminal() {
    let colours = |truecolor: bool| {
        let (catalog, mut game) = (catalog_en(), DemoGame::new(3));
        let mut app = app_colored(
            &mut game,
            &catalog,
            Some(Palette::new(PaletteChoice::Cvd, truecolor)),
        );
        busy(&mut app);
        cells(&app, 100, 28)
            .into_iter()
            .map(|cell| cell.fg)
            .collect::<Vec<_>>()
    };
    assert!(
        colours(true)
            .iter()
            .any(|c| matches!(c, Color::Rgb(0xD5, 0x5E, 0x00)))
    );
    assert!(colours(false).iter().all(|c| !matches!(c, Color::Rgb(..))));
}

#[test]
fn danger_is_reverse_video_and_errors_are_underlined_whatever_the_palette() {
    for choice in [
        PaletteChoice::Default,
        PaletteChoice::HighContrast,
        PaletteChoice::Cvd,
        PaletteChoice::Mono,
    ] {
        let palette = Palette::new(choice, true);
        let danger = to_style(
            palette.style(LineKind::Alert(neon_engine::event::Severity::Danger)),
            &palette,
        );
        assert!(
            danger
                .add_modifier
                .contains(Modifier::REVERSED | Modifier::BOLD),
            "{choice:?}"
        );
        let error = to_style(palette.style(LineKind::Error), &palette);
        assert!(
            error.add_modifier.contains(Modifier::UNDERLINED),
            "{choice:?}"
        );
    }
}

// ---- The campaign in the full-screen interface -----------------------------------------------

/// The campaign runs in the same model: the notoriety is a gauge of the status bar and the
/// panel, named and numbered with its band, and the active quest is listed in the panel.
#[test]
fn the_campaign_shows_the_notoriety_and_the_quest_in_the_status_bar_and_the_panel() {
    use neon_engine::campaign::{CampaignGame, Difficulty};

    let catalog = catalog_en();
    let mut game = CampaignGame::new(Difficulty::Normal, 1).unwrap();
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
    );
    type_line(&mut app, "Neon");
    // The prologue: two pages to continue, then the offer of the tutorial, declined.
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    type_line(&mut app, "n");
    type_line(&mut app, "hack localhost");
    type_line(&mut app, "y");
    let rows = screen(&app, 100, 28);
    // The status bar says it in words and numbers, never by colour alone.
    assert!(has(&rows[..1], "Neon"), "{rows:#?}");
    assert!(has(&rows[..1], "Notoriety 4/100 (Discreet)"), "{rows:#?}");
    // The panel repeats it with a bar, and lists what is left to do in the quest.
    assert!(has(&rows, "Notoriety 4/100"), "{rows:#?}");
    assert!(has(&rows, "[█░░░░░░░░░] Discreet"), "{rows:#?}");
    assert!(has(&rows, "First Steps"), "{rows:#?}");
    // The log carries the line of the gauge that moved.
    assert!(has(&rows, "Notoriety +4"), "{rows:#?}");
    // The compact layout keeps the figure in the status bar.
    let compact = screen(&app, 80, 24);
    assert!(
        has(&compact[..1], "Notoriety 4/100 (Discreet)"),
        "{compact:#?}"
    );
}

// ---- The log: scrollback, long lines, wide characters ----------------------------------------

/// A demo game with a long log: the prologue and `count` times the help screen.
fn scanned(app: &mut App<'_>, count: usize) {
    reach_command_line(app);
    for _ in 0..count {
        type_line(app, "help");
    }
}

#[test]
fn the_log_follows_the_game_and_shows_the_newest_events() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    scanned(&mut app, 40);
    let rows = screen(&app, 100, 28);
    assert!(
        !has(&rows, "NEON HACK") && !has(&rows, "Neo-Tokyo"),
        "the start has scrolled away"
    );
    let echoes = rows.iter().filter(|row| row.contains("> help")).count();
    assert!(echoes >= 2, "the last commands are in view: {rows:#?}");
    assert!(!has(&rows, "Scrolled back"), "no indicator while following");
}

#[test]
fn page_up_and_page_down_scroll_the_log_and_say_so() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    scanned(&mut app, 40);
    let bottom = screen(&app, 100, 28);
    press(&mut app, KeyCode::PageUp);
    let up = screen(&app, 100, 28);
    assert_ne!(up, bottom);
    let indicator = up.iter().find(|row| row.contains("Scrolled back"));
    assert!(
        indicator.is_some_and(|row| row.contains("lines below") && row.contains("End or Esc")),
        "{up:#?}"
    );
    // A page is the height of the log area less one row, so one line of context stays.
    press(&mut app, KeyCode::PageDown);
    assert_eq!(screen(&app, 100, 28), bottom, "back where it started");
    // PageDown at the bottom does nothing.
    press(&mut app, KeyCode::PageDown);
    assert_eq!(screen(&app, 100, 28), bottom);
}

#[test]
fn scrolling_never_goes_past_the_start_of_the_log() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    scanned(&mut app, 40);
    // Page after page (the size of a page is known once the screen has been drawn).
    for _ in 0..200 {
        press(&mut app, KeyCode::PageUp);
        let _ = screen(&app, 100, 28);
    }
    let top = screen(&app, 100, 28);
    assert!(has(&top, "Scrolled back"), "{top:#?}");
    // The very first line of the game is on screen, and one PageDown moves off it by a page.
    assert!(has(&top, "N E O N"), "{top:#?}");
    press(&mut app, KeyCode::PageDown);
    assert_ne!(screen(&app, 100, 28), top);
    // Home and End jump to both ends when nothing is typed.
    press(&mut app, KeyCode::End);
    assert!(!has(&screen(&app, 100, 28), "Scrolled back"));
    press(&mut app, KeyCode::Home);
    assert_eq!(screen(&app, 100, 28), top);
}

#[test]
fn escape_comes_back_down_before_it_clears_anything() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    scanned(&mut app, 40);
    type_text(&mut app, "net");
    press(&mut app, KeyCode::PageUp);
    assert!(has(&screen(&app, 100, 28), "Scrolled back"));
    press(&mut app, KeyCode::Esc);
    let rows = screen(&app, 100, 28);
    assert!(!has(&rows, "Scrolled back"));
    assert_eq!(input_row(&rows), "> net", "the line is still there");
    press(&mut app, KeyCode::Esc);
    assert_eq!(input_line(&app), ">", "the next one clears it");
}

#[test]
fn acting_brings_the_log_back_to_the_newest_events() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    scanned(&mut app, 40);
    press(&mut app, KeyCode::PageUp);
    press(&mut app, KeyCode::PageUp);
    type_line(&mut app, "status");
    assert!(!has(&screen(&app, 100, 28), "Scrolled back"));
}

#[test]
fn home_and_end_move_the_cursor_when_there_is_a_line_to_edit() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    scanned(&mut app, 40);
    type_text(&mut app, "bc");
    press(&mut app, KeyCode::Home);
    press(&mut app, KeyCode::Char('a'));
    assert_eq!(input_line(&app), "> abc");
    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Char('d'));
    assert_eq!(input_line(&app), "> abcd");
    assert!(
        !has(&screen(&app, 100, 28), "Scrolled back"),
        "the log did not move"
    );
}

#[test]
fn the_oldest_entries_are_forgotten_beyond_the_capacity_of_the_log() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    for _ in 0..(LOG_CAPACITY / 2 + 50) {
        type_line(&mut app, "help");
    }
    assert_eq!(app.log.len(), LOG_CAPACITY);
    let rows = screen(&app, 100, 28);
    assert!(has(&rows, "> help"), "the newest is kept");
    // The start of the game has been forgotten: scrolled to the top, it is not there.
    press(&mut app, KeyCode::Home);
    assert!(!has(&screen(&app, 100, 28), "N E O N"));
}

#[test]
fn long_lines_wrap_inside_the_log_and_never_spill_outside_it() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_line(&mut app, &"x".repeat(300));
    for (width, height) in [(100, 28), (64, 20), (80, 24)] {
        let rows = screen(&app, width, height);
        for row in &rows {
            assert!(row.width() <= usize::from(width), "{row:?}");
        }
        let full = rows
            .iter()
            .filter(|row| row.contains(&"x".repeat(30)))
            .count();
        assert!(
            full >= 3,
            "{width}x{height}: the echo wraps over several rows: {rows:#?}"
        );
    }
}

#[test]
fn wide_and_combining_characters_are_laid_out_by_their_display_width() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    press(&mut app, KeyCode::Enter);
    type_line(&mut app, "霓虹ハッカー");
    type_line(&mut app, "y");
    let rows = screen(&app, 64, 20);
    assert!(rows[0].contains("霓虹ハッカー"), "{:?}", rows[0]);
    assert!(has(&rows, "霓虹ハッカー."), "{rows:#?}");
    for row in &rows {
        assert!(row.width() <= 64, "{row:?}");
    }
    // The cursor sits after the typed text, counted in columns, not characters.
    type_text(&mut app, "e\u{301}日");
    let mut terminal = Terminal::new(TestBackend::new(64, 20)).unwrap();
    terminal.draw(|frame| app.draw(frame)).unwrap();
    let cursor = terminal.get_cursor_position().unwrap();
    assert_eq!((cursor.x, cursor.y), (2 + 1 + 2, 18));
}

#[test]
fn a_line_longer_than_the_screen_scrolls_sideways_and_keeps_the_cursor_in_sight() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_text(&mut app, &"abcdefghij".repeat(10));
    let mut terminal = Terminal::new(TestBackend::new(64, 20)).unwrap();
    terminal.draw(|frame| app.draw(frame)).unwrap();
    let cursor = terminal.get_cursor_position().unwrap();
    assert!(cursor.x < 64, "{cursor:?}");
    let rows = screen(&app, 64, 20);
    assert!(
        input_row(&rows).ends_with("ghij"),
        "the end of the line is shown"
    );
    assert!(
        !input_row(&rows).starts_with('>'),
        "the start has scrolled away"
    );
    press(&mut app, KeyCode::Home);
    let rows = screen(&app, 64, 20);
    assert!(
        input_row(&rows).starts_with("> abcdef"),
        "{:?}",
        input_row(&rows)
    );
    terminal.draw(|frame| app.draw(frame)).unwrap();
    assert_eq!(terminal.get_cursor_position().unwrap().x, 2);
}

// ---- The input line ----------------------------------------------------------------------------

#[test]
fn the_cursor_is_where_the_text_will_be_typed() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_text(&mut app, "hack");
    let at = |app: &App<'_>| {
        let mut terminal = Terminal::new(TestBackend::new(100, 28)).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        let cursor = terminal.get_cursor_position().unwrap();
        (cursor.x, cursor.y)
    };
    assert_eq!(at(&app), (2 + 4, 26));
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Left);
    assert_eq!(at(&app), (2 + 2, 26));
    ctrl(&mut app, 'a');
    assert_eq!(at(&app), (2, 26));
    ctrl(&mut app, 'e');
    assert_eq!(at(&app), (2 + 4, 26));
}

#[test]
fn control_keys_edit_the_line_in_the_interface() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_text(&mut app, "hack corp gateway");
    ctrl(&mut app, 'w');
    assert_eq!(input_line(&app), "> hack corp");
    ctrl(&mut app, 'a');
    ctrl(&mut app, 'k');
    assert_eq!(input_line(&app), ">");
    type_text(&mut app, "net");
    ctrl(&mut app, 'u');
    assert_eq!(input_line(&app), ">");
    type_text(&mut app, "abc");
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Delete);
    assert_eq!(input_line(&app), "> ab");
    // Ctrl-D with text is Delete; it closes the input only on an empty line.
    ctrl(&mut app, 'a');
    ctrl(&mut app, 'd');
    assert_eq!(input_line(&app), "> b");
    assert!(!has(&screen(&app, 100, 28), "Input closed"));
}

#[test]
fn history_recalls_commands_with_up_and_down_and_skips_menus() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_line(&mut app, "help");
    type_line(&mut app, "help");
    type_line(&mut app, "status");
    type_text(&mut app, "ne");
    press(&mut app, KeyCode::Up);
    assert_eq!(input_line(&app), "> status");
    press(&mut app, KeyCode::Up);
    assert_eq!(input_line(&app), "> help");
    press(&mut app, KeyCode::Up);
    assert_eq!(
        input_line(&app),
        "> help",
        "the repeat was not stored twice"
    );
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    assert_eq!(input_line(&app), "> ne", "the unfinished line comes back");
    press(&mut app, KeyCode::Esc);
    // A menu answer is not a command: it is not recorded.
    type_line(&mut app, "shop");
    type_line(&mut app, "1");
    press(&mut app, KeyCode::Up);
    assert_eq!(input_line(&app), "?", "no history in a menu");
    while input_line(&app).starts_with('?') {
        press(&mut app, KeyCode::Esc);
    }
    press(&mut app, KeyCode::Up);
    assert_eq!(
        input_line(&app),
        "> shop",
        "the menu answers are not in the history"
    );
}

#[test]
fn a_paste_is_one_line_and_runs_nothing() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    let before = screen(&app, 100, 28);
    app.on_paste("scan\nscan\nstatus\n");
    let rows = screen(&app, 100, 28);
    assert_eq!(input_row(&rows), "> scan scan status");
    assert_eq!(
        rows.iter().filter(|row| row.contains("scan")).count(),
        1,
        "the only `scan` on screen is the input line"
    );
    assert_eq!(
        rows[..rows.len() - 2],
        before[..before.len() - 2],
        "the log did not change"
    );
}

#[test]
fn the_text_of_a_prompt_is_limited_to_what_it_asks_for() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, &"n".repeat(200));
    assert!(
        input_line(&app).chars().count() < 80,
        "{}",
        input_line(&app)
    );
    // After the handle is given, the command line takes long lines again.
    press(&mut app, KeyCode::Esc);
    type_line(&mut app, "Neon");
    type_line(&mut app, "y");
    type_text(&mut app, &"n".repeat(200));
    assert_eq!(app.editor.text().chars().count(), 200);
}

// ---- Completion ----------------------------------------------------------------------------------

fn opened_campaign<'a>(
    game: &'a mut neon_engine::campaign::CampaignGame,
    catalog: &'a Catalog,
    mode: RenderMode,
) -> App<'a> {
    let first = game.start();
    let mut app = App::new(
        game,
        Renderer {
            catalog,
            mode,
            verbosity: Verbosity::Normal,
            colors: None,
        },
        first,
        Persistence::disabled(),
    );
    type_line(&mut app, "Neon");
    // The prologue: two pages to continue, then the offer of the tutorial, declined.
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    type_line(&mut app, "n");
    app
}

#[test]
fn tab_completes_the_arguments_of_a_command_from_the_game() {
    use neon_engine::campaign::{CampaignGame, Difficulty};

    let catalog = catalog_en();
    let mut game = CampaignGame::new(Difficulty::Normal, 1).unwrap();
    let mut app = opened_campaign(&mut game, &catalog, RenderMode::FULL);
    type_text(&mut app, "talk ec");
    press(&mut app, KeyCode::Tab);
    assert_eq!(
        input_line(&app),
        "> talk echo7",
        "the word is completed, not the line"
    );
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(input_line(&app), "> talk echo7 x", "and spaced");
    // The word before the cursor is the one completed, and what follows it stays.
    press(&mut app, KeyCode::Esc);
    type_text(&mut app, "talk ec");
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Tab);
    assert_eq!(input_line(&app), "> talk echo7 ec");
}

#[test]
fn tab_lists_what_it_cannot_decide() {
    use neon_engine::campaign::{CampaignGame, Difficulty};

    let catalog = catalog_en();
    let mut game = CampaignGame::new(Difficulty::Normal, 1).unwrap();
    let mut app = opened_campaign(&mut game, &catalog, RenderMode::FULL);
    type_text(&mut app, "s");
    press(&mut app, KeyCode::Tab);
    let rows = screen(&app, 100, 28);
    assert!(
        rows[rows.len() - 3].contains("status"),
        "the candidates are listed: {rows:#?}"
    );
    assert!(!app.candidates.is_empty());
    press(&mut app, KeyCode::Char('t'));
    assert!(
        app.candidates.is_empty(),
        "the list is gone at the next key"
    );
}

// ---- Size ------------------------------------------------------------------------------------------

#[test]
fn a_terminal_too_small_shows_one_centred_sentence_and_ignores_the_keys() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_text(&mut app, "sca");
    let before = screen(&app, 100, 28);

    let small = screen(&app, 56, 16);
    let lines: Vec<&String> = small.iter().filter(|row| !row.is_empty()).collect();
    assert!(!lines.is_empty() && lines.len() <= 3, "{small:#?}");
    assert!(
        lines[0].contains("Terminal too small (56x16)"),
        "{small:#?}"
    );
    let first = small.iter().position(|row| !row.is_empty()).unwrap();
    assert!(
        (6..=9).contains(&first),
        "centred vertically: starts on row {first}: {small:#?}"
    );
    assert!(
        lines[0].starts_with(' '),
        "centred horizontally: {:?}",
        lines[0]
    );

    // Nothing typed while it is too small reaches the game or the line.
    type_text(&mut app, "xyz");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    assert_eq!(screen(&app, 100, 28), before, "the state is untouched");
    // Bigger again: everything is back.
    assert_eq!(screen(&app, 100, 28), before);
    assert_eq!(screen(&app, 56, 16), small);
}

#[test]
fn the_too_small_sentence_follows_the_language_and_the_screen_reader_wording() {
    let (catalog, mut game) = (catalog_fr(), DemoGame::new(1));
    let app = app_with(&mut game, &catalog, RenderMode::FULL);
    assert!(has(&screen(&app, 63, 19), "Terminal trop petit (63x19)"));
    let reader = RenderMode {
        screen_reader: true,
        ascii: false,
    };
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let app = app_with(&mut game, &catalog, reader);
    let rows = screen(&app, 40, 10);
    let said = rows
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        said.contains("40 columns by 10 lines") && !said.contains("40x10"),
        "{rows:#?}"
    );
}

#[test]
fn control_c_still_leaves_when_the_screen_is_too_small_to_see_a_question() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    let _ = screen(&app, 50, 15);
    type_text(&mut app, "x");
    ctrl(&mut app, 'c');
    assert!(
        has(&screen(&app, 100, 28), "Input closed"),
        "the input closed, with no invisible question"
    );
}

#[test]
fn the_panel_is_empty_of_nothing_but_the_gauges_and_objectives_at_the_full_tier() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    let rows = screen(&app, 100, 28);
    // The panel is 30 columns wide, boxed, and titled.
    assert!(
        rows[1].ends_with("╮") && rows[1].contains("Panel"),
        "{:?}",
        rows[1]
    );
    let panel_start = rows[1].find("╭Panel").unwrap();
    assert_eq!(rows[1][..panel_start].width(), 70, "{:?}", rows[1]);
}

#[test]
fn key_releases_focus_and_the_mouse_do_nothing() {
    use ratatui::crossterm::event::{
        Event as TerminalEvent, KeyEventKind, MouseEvent, MouseEventKind,
    };

    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    let press_event =
        KeyEvent::new_with_kind(KeyCode::Char('a'), KeyModifiers::NONE, KeyEventKind::Press);
    let release = KeyEvent {
        kind: KeyEventKind::Release,
        ..press_event
    };
    let repeat = KeyEvent {
        kind: KeyEventKind::Repeat,
        ..press_event
    };
    assert!(super::super::handle(
        &mut app,
        TerminalEvent::Key(press_event)
    ));
    assert!(!super::super::handle(&mut app, TerminalEvent::Key(release)));
    assert!(super::super::handle(&mut app, TerminalEvent::Key(repeat)));
    assert_eq!(input_line(&app), "> aa", "a release is not a second key");
    assert!(!super::super::handle(&mut app, TerminalEvent::FocusLost));
    assert!(!super::super::handle(&mut app, TerminalEvent::FocusGained));
    let wheel = MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 1,
        row: 1,
        modifiers: KeyModifiers::NONE,
    };
    assert!(!super::super::handle(&mut app, TerminalEvent::Mouse(wheel)));
    assert!(super::super::handle(
        &mut app,
        TerminalEvent::Resize(80, 24)
    ));
    assert!(super::super::handle(
        &mut app,
        TerminalEvent::Paste("b".to_owned())
    ));
    assert_eq!(input_line(&app), "> aab");
}

// ---- Whatever the player does ------------------------------------------------------------------------

fn any_key() -> impl Strategy<Value = KeyEvent> {
    let code = prop_oneof![
        any::<char>().prop_map(KeyCode::Char),
        Just(KeyCode::Char('a')),
        Just(KeyCode::Char('c')),
        Just(KeyCode::Char('d')),
        Just(KeyCode::Char('é')),
        Just(KeyCode::Enter),
        Just(KeyCode::Esc),
        Just(KeyCode::Tab),
        Just(KeyCode::Backspace),
        Just(KeyCode::Delete),
        Just(KeyCode::Left),
        Just(KeyCode::Right),
        Just(KeyCode::Up),
        Just(KeyCode::Down),
        Just(KeyCode::Home),
        Just(KeyCode::End),
        Just(KeyCode::PageUp),
        Just(KeyCode::PageDown),
        Just(KeyCode::F(2)),
    ];
    let modifiers = prop_oneof![
        Just(KeyModifiers::NONE),
        Just(KeyModifiers::NONE),
        Just(KeyModifiers::SHIFT),
        Just(KeyModifiers::CONTROL),
        Just(KeyModifiers::ALT),
        Just(KeyModifiers::CONTROL | KeyModifiers::ALT),
    ];
    (code, modifiers).prop_map(|(code, modifiers)| KeyEvent::new(code, modifiers))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn no_sequence_of_keys_pastes_and_sizes_can_panic_or_lose_the_cursor(
        steps in proptest::collection::vec(
            prop_oneof![
                any_key().prop_map(|key| (Some(key), None, 0usize)),
                "[a-z \\n\u{301}日]{0,20}".prop_map(|text| (None, Some(text), 0)),
                (0usize..6).prop_map(|size| (None, None, size)),
            ],
            0..60,
        ),
    ) {
        const SIZES: [(u16, u16); 6] = [(100, 28), (64, 20), (63, 20), (30, 8), (200, 60), (1, 1)];
        let (catalog, mut game) = (catalog_en(), DemoGame::new(7));
        let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
        let mut size = SIZES[0];
        for (key, paste, resize) in steps {
            if let Some(key) = key {
                app.on_key(key);
            }
            if let Some(text) = paste {
                app.on_paste(&text);
            }
            if resize > 0 {
                size = SIZES[resize];
            }
            let mut terminal = Terminal::new(TestBackend::new(size.0, size.1)).unwrap();
            terminal.draw(|frame| app.draw(frame)).unwrap();
            if !app.should_quit() && app.shown_tier.get() != Tier::TooSmall && !app.finished {
                let cursor = terminal.get_cursor_position().unwrap();
                prop_assert!(cursor.x < size.0 && cursor.y < size.1, "{cursor:?} in {size:?}");
            }
            if app.should_quit() {
                break;
            }
        }
    }
}

#[test]
fn control_c_drops_what_was_being_typed_so_it_cannot_answer_the_question() {
    let (catalog, mut game) = (catalog_en(), DemoGame::new(1));
    let mut app = app_with(&mut game, &catalog, RenderMode::FULL);
    reach_command_line(&mut app);
    type_text(&mut app, "sta");
    ctrl(&mut app, 'c');
    assert!(has(&screen(&app, 100, 28), "> quit"));
    assert_eq!(app.editor.text(), "", "the half-typed command is gone");
    type_line(&mut app, "y");
    assert!(has(&screen(&app, 100, 28), "Press any key to leave."));
}
