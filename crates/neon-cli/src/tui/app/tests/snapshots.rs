//! Snapshots of the screen, as text and as styles.
//!
//! The text is what a player reads; the style runs are what a snapshot of text cannot see
//! (reverse video, underline, the palettes). Both are checked in, so a change of the layout,
//! of a wording, of `ratatui` or of `unicode-width` shows up as a diff to review.

use neon_engine::campaign::{CampaignGame, Difficulty};

use super::*;

/// The campaign, past the opening (handle, prologue, tutorial declined), under `colors`.
fn campaign_app<'a>(
    game: &'a mut CampaignGame,
    catalog: &'a Catalog,
    mode: RenderMode,
    colors: Option<Palette>,
    handle: &str,
) -> App<'a> {
    let first = game.start();
    let mut app = App::new(
        game,
        Renderer {
            catalog,
            mode,
            verbosity: Verbosity::Normal,
            colors,
        },
        first,
        Persistence::disabled(),
    );
    type_line(&mut app, handle);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    type_line(&mut app, "n");
    app
}

fn campaign() -> CampaignGame {
    CampaignGame::new(Difficulty::Normal, 1).unwrap()
}

/// The screen, one row per line, under a header that says the size.
fn picture(app: &App<'_>, width: u16, height: u16) -> String {
    let mut text = format!("{width}x{height}\n");
    text.push_str(&screen(app, width, height).join("\n"));
    text
}

/// A few turns of play: a rejected command, a first intrusion (reward, the gauge moves).
fn play(app: &mut App<'_>) {
    type_line(app, "nonsense");
    type_line(app, "hack localhost");
    type_line(app, "y");
}

#[test]
fn the_wide_tier_in_english() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, None, "Neon");
    play(&mut app);
    insta::assert_snapshot!(picture(&app, 100, 28));
}

#[test]
fn the_compact_tier_in_english() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, None, "Neon");
    play(&mut app);
    insta::assert_snapshot!(picture(&app, 64, 20));
}

#[test]
fn the_wide_tier_in_french() {
    let (catalog, mut game) = (catalog_fr(), campaign());
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, None, "Néon");
    play(&mut app);
    insta::assert_snapshot!(picture(&app, 100, 28));
}

#[test]
fn the_compact_tier_in_french() {
    let (catalog, mut game) = (catalog_fr(), campaign());
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, None, "Néon");
    play(&mut app);
    insta::assert_snapshot!(picture(&app, 64, 20));
}

#[test]
fn ascii_mode_in_french_at_both_tiers() {
    let (catalog, mut game) = (catalog_fr(), campaign());
    let mut app = campaign_app(&mut game, &catalog, RenderMode::ASCII, None, "Néon");
    play(&mut app);
    let wide = picture(&app, 100, 28);
    let compact = picture(&app, 64, 20);
    for row in wide.lines().chain(compact.lines()) {
        assert!(row.is_ascii(), "not ASCII: {row:?}");
    }
    insta::assert_snapshot!("ascii_wide", wide);
    insta::assert_snapshot!("ascii_compact", compact);
}

#[test]
fn a_menu_above_the_input_line_in_the_compact_tier() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, None, "Neon");
    type_line(&mut app, "talk echo7");
    insta::assert_snapshot!(picture(&app, 64, 20));
}

#[test]
fn candidates_listed_under_the_log() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, None, "Neon");
    type_text(&mut app, "s");
    press(&mut app, KeyCode::Tab);
    insta::assert_snapshot!(picture(&app, 64, 20));
}

#[test]
fn scrollback_one_page_up() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, None, "Neon");
    for _ in 0..4 {
        type_line(&mut app, "help");
    }
    // The first drawing tells how big a page is.
    let _ = screen(&app, 100, 28);
    press(&mut app, KeyCode::PageUp);
    insta::assert_snapshot!(picture(&app, 100, 28));
}

#[test]
fn long_lines_and_wide_characters() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, None, "霓虹ハッカー");
    type_line(
        &mut app,
        "ネオン・ハッカー wanders the 日本語 market looking for a very long stretch of text that has to wrap",
    );
    type_text(
        &mut app,
        "e\u{301}e\u{301}e\u{301} 日本語日本語日本語日本語日本語日本語日本語日本語日本語日本語日本語日本語日本語日本語日本語日本語",
    );
    insta::assert_snapshot!(picture(&app, 64, 20));
}

#[test]
fn too_small_in_each_language_and_for_a_screen_reader() {
    let (catalog_english, mut english) = (catalog_en(), campaign());
    let app = campaign_app(
        &mut english,
        &catalog_english,
        RenderMode::FULL,
        None,
        "Neon",
    );
    let (catalog_french, mut french) = (catalog_fr(), campaign());
    let in_french = campaign_app(&mut french, &catalog_french, RenderMode::FULL, None, "Neon");
    let (catalog_reader, mut reader) = (catalog_en(), campaign());
    let reading = campaign_app(
        &mut reader,
        &catalog_reader,
        RenderMode {
            screen_reader: true,
            ascii: false,
        },
        None,
        "Neon",
    );
    let text = [
        picture(&app, 50, 15),
        picture(&in_french, 50, 15),
        picture(&reading, 50, 15),
    ]
    .join("\n\n");
    insta::assert_snapshot!(text);
}

// ---- Styles ------------------------------------------------------------------------------------

/// Where the screen is not plain: for each row, the runs of cells that share a style that is
/// not the terminal's default.
fn style_runs(app: &App<'_>, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| app.draw(frame)).unwrap();
    let buffer = terminal.backend().buffer();
    let mut lines = vec![format!("{width}x{height}")];
    for y in 0..height {
        let mut run: Option<(u16, String)> = None;
        for x in 0..=width {
            let key = (x < width)
                .then(|| &buffer[(x, y)])
                .filter(|cell| {
                    cell.fg != Color::Reset || cell.bg != Color::Reset || !cell.modifier.is_empty()
                })
                .map(|cell| format!("fg={:?} bg={:?} {:?}", cell.fg, cell.bg, cell.modifier));
            match (&run, &key) {
                (Some((_, open)), Some(next)) if open == next => {}
                _ => {
                    if let Some((start, open)) = run.take() {
                        lines.push(format!("{y:>2} {start:>3}-{:<3} {open}", x - 1));
                    }
                    run = key.map(|key| (x, key));
                }
            }
        }
    }
    lines.join("\n")
}

fn styled(choice: PaletteChoice, truecolor: bool) -> String {
    let (catalog, mut game) = (catalog_en(), campaign());
    let colors = Some(Palette::new(choice, truecolor));
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, colors, "Neon");
    play(&mut app);
    // Something to carry a danger style: the gauge cannot reach it here, so the error line
    // and the reward stand for the roles.
    style_runs(&app, 64, 20)
}

#[test]
fn styles_without_colour_are_attributes_only() {
    let (catalog, mut game) = (catalog_en(), campaign());
    let mut app = campaign_app(&mut game, &catalog, RenderMode::FULL, None, "Neon");
    play(&mut app);
    insta::assert_snapshot!(style_runs(&app, 64, 20));
}

#[test]
fn styles_of_the_default_palette() {
    insta::assert_snapshot!(styled(PaletteChoice::Default, true));
}

#[test]
fn styles_of_the_high_contrast_palette() {
    insta::assert_snapshot!(styled(PaletteChoice::HighContrast, true));
}

#[test]
fn styles_of_the_cvd_palette_on_a_24_bit_terminal() {
    insta::assert_snapshot!(styled(PaletteChoice::Cvd, true));
}

#[test]
fn styles_of_the_cvd_palette_on_a_16_colour_terminal() {
    insta::assert_snapshot!(styled(PaletteChoice::Cvd, false));
}

#[test]
fn styles_of_the_mono_palette_chosen_by_hand() {
    insta::assert_snapshot!(styled(PaletteChoice::Mono, true));
}
