//! The staged scenes: when they start, what they say, how they end, and that they never break
//! a rule of the screen (no blink, no colour without colour, nothing outside the screen).

use std::time::Duration;

use neon_engine::event::{Effect, Gauge, Role};
use neon_engine::save::SaveError;
use neon_engine::{Game, Input, Prompt, Step, View};

use super::*;
use crate::tui::motion::FRAME;

/// A game that answers `hack` and `laylow` with the scene and the words that go with it, so
/// that the interface can be tried without playing the campaign to the shop.
pub(super) struct StageGame;

fn raw(text: &str) -> Text {
    Text::new("ui.raw").with_str("value", text)
}

fn said(text: &str) -> Event {
    Event::message(
        Role::System,
        neon_engine::event::Importance::Essential,
        raw(text),
    )
}

/// The commands of the stub that stage a scene.
const SCENES: [&str; 4] = ["hack", "laylow", "decrypt", "buy"];

impl Game for StageGame {
    fn start(&mut self) -> Step {
        self.resume()
    }

    fn handle(&mut self, input: Input) -> Step {
        let events = match input {
            Input::Line(line) if line.trim() == "hack" => vec![
                Event::Effect(Effect::Hack {
                    site: "corp-server-01".to_owned(),
                    security: 4,
                    taken: 2,
                }),
                Event::Changed {
                    gauge: Gauge::Notoriety,
                    from: 10,
                    to: 20,
                    band: Text::new("band.calm"),
                },
                said("Intrusion over."),
            ],
            Input::Line(line) if line.trim() == "laylow" => vec![
                Event::Effect(Effect::Laylow { cooling: 10 }),
                Event::Changed {
                    gauge: Gauge::Notoriety,
                    from: 30,
                    to: 20,
                    band: Text::new("band.calm"),
                },
                said("Cover bought."),
            ],
            Input::Line(line) if line.trim() == "decrypt" => vec![
                Event::Effect(Effect::Decrypt {
                    document: raw("Personnel register"),
                    fragment: true,
                }),
                said("Document decrypted."),
            ],
            Input::Line(line) if line.trim() == "buy" => vec![
                Event::Effect(Effect::Buy {
                    item: raw("Ghost Protocol"),
                    price: 80,
                    balance: 420,
                }),
                said("Bought."),
            ],
            Input::Line(line) if line.trim() == "tier6" => vec![
                Event::Effect(Effect::Hack {
                    site: "sentinel-hub".to_owned(),
                    security: 9,
                    taken: 0,
                }),
                said("Beyond the scale."),
            ],
            _ => Vec::new(),
        };
        Step {
            events,
            prompt: Prompt::Command,
            save: None,
        }
    }

    fn prompt(&self) -> Prompt {
        Prompt::Command
    }

    fn snapshot(&self) -> Result<String, SaveError> {
        Ok(String::new())
    }

    fn view(&self) -> View {
        View {
            player: "Neon".to_owned(),
            gauges: Vec::new(),
            sections: Vec::new(),
            status: Vec::new(),
        }
    }

    fn complete(&self, _line: &str) -> Vec<String> {
        Vec::new()
    }
}

fn stage_app<'a>(
    game: &'a mut StageGame,
    catalog: &'a Catalog,
    mode: RenderMode,
    colors: Option<Palette>,
    motion: bool,
) -> App<'a> {
    let first = game.start();
    App::new(
        game,
        Renderer {
            catalog,
            mode,
            verbosity: Verbosity::Normal,
            colors,
        },
        first,
        Persistence::disabled(),
    )
    .with_motion(motion)
}

/// Moves the scene forward by `millis`, a frame at a time like the event loop does.
fn advance(app: &mut App<'_>, millis: u64) {
    let mut left = Duration::from_millis(millis);
    while left > Duration::ZERO && app.is_animating() {
        let step = left.min(FRAME);
        app.tick(step);
        left -= step;
    }
}

fn text_of(rows: &[String]) -> String {
    rows.join("\n")
}

#[test]
fn a_scene_is_staged_only_when_motion_is_allowed() {
    let catalog = catalog_en();
    for (motion, expected) in [(false, false), (true, true)] {
        let mut game = StageGame;
        let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, motion);
        assert!(!app.is_animating(), "nothing moves before the player acts");
        type_line(&mut app, "hack");
        assert_eq!(app.is_animating(), expected, "motion={motion}");
        // Nothing to stage: nothing moves.
        let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, motion);
        type_line(&mut app, "nonsense");
        assert!(!app.is_animating());
    }
}

#[test]
fn an_interface_that_is_not_told_about_motion_never_moves_by_itself() {
    let catalog = catalog_en();
    let mut game = StageGame;
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
    type_line(&mut app, "hack");
    assert!(!app.is_animating());
}

#[test]
fn the_hack_says_what_it_does_the_progress_and_the_defences_of_the_site() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "hack");
    let rows = screen(&app, 100, 28);
    let text = text_of(&rows);
    assert!(
        text.contains("Hack in progress, 0% done, site security: high"),
        "{text}"
    );
    assert!(text.contains("Run: corp-server-01"), "{text}");
    assert!(text.contains("Scanning ports"), "{text}");
    assert!(text.contains("ICE ■■■■□□"), "{text}");
    assert!(text.contains("Press any key to skip."), "{text}");
    // The window reveals the result: what the step said waits until the scene is over, but
    // what the player typed is already there.
    assert!(
        !text.contains("Intrusion over."),
        "the result is held back: {text}"
    );
    assert!(!text.contains("Notoriety +10"), "the gauge too: {text}");
    assert!(text.contains("> hack"), "{text}");

    advance(&mut app, 600);
    let middle = text_of(&screen(&app, 100, 28));
    let percent: u32 = middle
        .split("Hack in progress, ")
        .nth(1)
        .and_then(|rest| rest.split('%').next())
        .and_then(|number| number.parse().ok())
        .expect("a percentage");
    assert!(
        (10..90).contains(&percent),
        "{percent}% in the middle:\n{middle}"
    );
    assert!(middle.contains(&format!("{percent:>4}%")), "{middle}");
}

#[test]
fn the_french_wording_is_the_one_of_the_game() {
    let (catalog, mut game) = (catalog_fr(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "hack");
    let text = text_of(&screen(&app, 100, 28));
    assert!(
        text.contains("Hack en cours, progression 0 %, sécurité du site : élevée"),
        "{text}"
    );
    assert!(text.contains("Intrusion : corp-server-01"), "{text}");
    advance(&mut app, 1_700);
    let end = text_of(&screen(&app, 100, 28));
    assert!(end.contains("Accès obtenu, 2 fichiers pris."), "{end}");
}

#[test]
fn the_scene_ends_by_itself_within_two_seconds_and_the_result_stays() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "hack");
    let mut frames = 0;
    while app.is_animating() {
        app.tick(FRAME);
        frames += 1;
        assert!(frames <= 40, "still playing after {frames} frames");
    }
    let text = text_of(&screen(&app, 100, 28));
    assert!(text.contains("Intrusion over."), "{text}");
    assert!(!text.contains("Hack in progress"), "{text}");
}

#[test]
fn the_end_of_a_hack_holds_the_result_a_moment() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "hack");
    advance(&mut app, 1_600);
    assert!(app.is_animating(), "the last moments are held");
    let text = text_of(&screen(&app, 100, 28));
    assert!(
        text.contains("Hack complete, 100% done, site security: high"),
        "{text}"
    );
    assert!(text.contains("Access gained, 2 files taken."), "{text}");
}

#[test]
fn a_key_ends_the_scene_and_is_typed_as_usual() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "hack");
    advance(&mut app, 300);
    press(&mut app, KeyCode::Char('x'));
    assert!(!app.is_animating());
    let rows = screen(&app, 100, 28);
    assert!(input_row(&rows).ends_with('x'), "{rows:?}");
    assert!(has(&rows, "Intrusion over."), "{rows:?}");
}

#[test]
fn a_paste_ends_the_scene_and_lands_in_the_line() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "hack");
    app.on_paste("status");
    assert!(!app.is_animating());
    assert!(input_row(&screen(&app, 100, 28)).ends_with("status"));
}

#[test]
fn the_next_command_after_a_scene_may_start_another() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "hack");
    advance(&mut app, 5_000);
    assert!(!app.is_animating());
    type_line(&mut app, "laylow");
    assert!(app.is_animating());
    let text = text_of(&screen(&app, 100, 28));
    assert!(text.contains("Covering your tracks, 0% done"), "{text}");
    assert!(text.contains("Notoriety: 30"), "{text}");
}

#[test]
fn the_cover_drains_the_notoriety_and_says_so_at_the_end() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "laylow");
    advance(&mut app, 1_000);
    let middle = text_of(&screen(&app, 100, 28));
    let value: i32 = middle
        .split("Notoriety: ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|number| number.parse().ok())
        .expect("a value");
    assert!((20..30).contains(&value), "{middle}");
    advance(&mut app, 380);
    let end = text_of(&screen(&app, 100, 28));
    assert!(end.contains("Notoriety: 20"), "{end}");
    assert!(end.contains("Tracks covered, Notoriety -10."), "{end}");
}

#[test]
fn a_level_beyond_the_scale_shows_the_highest_word_and_all_pips() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "tier6");
    let text = text_of(&screen(&app, 100, 28));
    assert!(text.contains("site security: extreme"), "{text}");
    assert!(text.contains("ICE ■■■■■■"), "{text}");
    advance(&mut app, 1_700);
    assert!(text_of(&screen(&app, 100, 28)).contains("nothing new to take"));
}

#[test]
fn the_window_stays_inside_the_log_and_the_prompt_and_the_bar_stay_visible() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "hack");
    for (width, height) in [(100, 28), (64, 20)] {
        let rows = screen(&app, width, height);
        assert!(rows[0].contains("Neon"), "{width}x{height}: the status bar");
        assert!(
            rows[rows.len() - 2].starts_with('>'),
            "{width}x{height}: the input line"
        );
        assert!(
            rows[rows.len() - 1].contains("Tab complete"),
            "{width}x{height}: the help"
        );
        assert!(has(&rows, "Hack in progress"), "{width}x{height}");
        if width >= 100 {
            // The panel keeps its place beside the window.
            assert!(has(&rows, "Panel") || has(&rows, "panel"), "{rows:?}");
        }
    }
}

#[test]
fn a_room_too_small_for_the_window_draws_none_and_loses_nothing() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "hack");
    // Too short for the status and the bar, then too narrow: the log is simply shown.
    for (width, height) in [(64, 20), (64, 9), (64, 5), (31, 20), (12, 20), (64, 3)] {
        let _ = screen(&app, width, height);
    }
    let rows = screen(&app, 64, 6);
    assert!(!has(&rows, "Hack in progress"), "{rows:?}");
    advance(&mut app, 5_000);
    assert!(has(&screen(&app, 64, 20), "Intrusion over."));
}

#[test]
fn ascii_mode_stages_with_ascii_only() {
    let (catalog, mut game) = (catalog_fr(), StageGame);
    let mode = RenderMode {
        screen_reader: false,
        ascii: true,
    };
    let mut app = stage_app(&mut game, &catalog, mode, None, true);
    type_line(&mut app, "hack");
    for millis in [0, 400, 400, 400, 400, 400] {
        advance(&mut app, millis);
        for (width, height) in [(100, 28), (64, 20)] {
            for row in screen(&app, width, height) {
                assert!(row.is_ascii(), "{row:?}");
            }
        }
    }
}

#[test]
fn without_colour_a_scene_uses_attributes_only() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "laylow");
    for _ in 0..30 {
        for (width, height) in SIZES {
            for cell in cells(&app, width, height) {
                assert_eq!(cell.fg, Color::Reset, "{width}x{height}: {cell:?}");
                assert_eq!(cell.bg, Color::Reset, "{width}x{height}: {cell:?}");
            }
        }
        app.tick(FRAME);
    }
}

#[test]
fn nothing_blinks_while_a_scene_plays_in_any_palette() {
    for choice in [
        PaletteChoice::Default,
        PaletteChoice::HighContrast,
        PaletteChoice::Cvd,
        PaletteChoice::Mono,
    ] {
        for truecolor in [false, true] {
            let (catalog, mut game) = (catalog_en(), StageGame);
            let colors = Some(Palette::new(choice, truecolor));
            let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, colors, true);
            type_line(&mut app, "hack");
            for _ in 0..40 {
                for (width, height) in SIZES {
                    for cell in cells(&app, width, height) {
                        assert!(
                            !cell.modifier.contains(Modifier::SLOW_BLINK)
                                && !cell.modifier.contains(Modifier::RAPID_BLINK),
                            "{choice:?} {width}x{height}"
                        );
                    }
                }
                app.tick(FRAME);
            }
        }
    }
}

#[test]
fn an_opaque_palette_still_paints_every_cell_under_the_window() {
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
            let (catalog, mut game) = (catalog_en(), StageGame);
            let colors = Some(Palette::new(choice, truecolor));
            let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, colors, true);
            type_line(&mut app, "hack");
            let expected = if truecolor { exact } else { plain };
            for _ in 0..40 {
                for (width, height) in SIZES {
                    for cell in cells(&app, width, height) {
                        if cell.modifier.contains(Modifier::REVERSED) {
                            continue;
                        }
                        assert_eq!(cell.bg, expected, "{choice:?} {truecolor} {width}x{height}");
                    }
                }
                app.tick(FRAME);
            }
        }
    }
}

#[test]
fn the_bar_goes_from_magenta_to_cyan_where_the_terminal_does_24_bit_colour() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let colors = Some(Palette::new(PaletteChoice::HighContrast, true));
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, colors, true);
    type_line(&mut app, "hack");
    advance(&mut app, 1_700);
    let all = cells(&app, 100, 28);
    // The cells of the full bar are the █ of the window.
    let filled: Vec<_> = all.iter().filter(|cell| cell.symbol() == "█").collect();
    assert!(filled.len() >= 10, "{} cells", filled.len());
    let first = filled.first().unwrap().fg;
    let last = filled.last().unwrap().fg;
    let (Color::Rgb(r1, _, b1), Color::Rgb(r2, _, b2)) = (first, last) else {
        panic!("exact colours expected: {first:?} {last:?}");
    };
    // Magenta (red and blue) at one end, cyan (no red, blue) at the other.
    assert!(r1 > r2 && b1 >= 200 && b2 >= 200, "{first:?} -> {last:?}");
    for pair in filled.windows(2) {
        let (Color::Rgb(a, ..), Color::Rgb(b, ..)) = (pair[0].fg, pair[1].fg) else {
            panic!("exact colours expected");
        };
        assert!(a >= b, "the gradient only goes one way");
    }
}

#[test]
fn the_real_campaign_stages_its_first_intrusion() {
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
    )
    .with_motion(true);
    type_line(&mut app, "Neon");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    type_line(&mut app, "n");
    type_line(&mut app, "hack localhost");
    assert!(
        !app.is_animating(),
        "nothing is staged before the intrusion is confirmed"
    );
    type_line(&mut app, "y");
    // The scene is for the end of the intrusion, which is played turn by turn.
    assert!(!app.is_animating(), "nothing is staged while the intrusion is played");
    for line in LOCALHOST_RUN {
        type_line(&mut app, line);
    }
    assert!(app.is_animating());
    let text = text_of(&screen(&app, 100, 28));
    assert!(
        text.contains("Hack in progress, 0% done, site security: minimal"),
        "{text}"
    );
    assert!(text.contains("Run: localhost"), "{text}");
    advance(&mut app, 5_000);
    assert!(text_of(&screen(&app, 100, 28)).contains("localhost: 1 file taken."));
}

#[test]
fn a_session_of_keys_ticks_and_sizes_cannot_panic() {
    let (catalog, mut game) = (catalog_fr(), StageGame);
    let colors = Some(Palette::new(PaletteChoice::Cvd, true));
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, colors, true);
    for round in 0..6 {
        type_line(&mut app, if round % 2 == 0 { "hack" } else { "laylow" });
        for millis in [0, 50, 700, 1, 2_500] {
            app.tick(Duration::from_millis(millis));
            for (width, height) in [
                (0, 0),
                (1, 1),
                (30, 8),
                (63, 19),
                (64, 20),
                (100, 28),
                (300, 100),
            ] {
                let _ = cells(&app, width, height);
            }
        }
    }
}

#[test]
fn staged_screens_in_both_languages_and_sizes() {
    let picture = |app: &App<'_>, width: u16, height: u16| {
        format!(
            "{width}x{height}\n{}",
            screen(app, width, height).join("\n")
        )
    };

    let (catalog, mut game) = (catalog_fr(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "hack");
    advance(&mut app, 700);
    insta::assert_snapshot!("a_hack_in_french_on_the_wide_tier", picture(&app, 100, 28));

    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "laylow");
    advance(&mut app, 600);
    insta::assert_snapshot!(
        "a_cover_in_english_on_the_compact_tier",
        picture(&app, 64, 20)
    );

    let (catalog, mut game) = (catalog_en(), StageGame);
    let mode = RenderMode {
        screen_reader: false,
        ascii: true,
    };
    let mut app = stage_app(&mut game, &catalog, mode, None, true);
    type_line(&mut app, "hack");
    advance(&mut app, 1_700);
    insta::assert_snapshot!(
        "a_finished_hack_in_ascii_on_the_wide_tier",
        picture(&app, 100, 28)
    );
}

#[test]
fn a_decryption_names_the_document_and_the_fragment_it_yields() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "decrypt");
    let text = text_of(&screen(&app, 100, 28));
    assert!(text.contains("Decrypting: Personnel register"), "{text}");
    assert!(text.contains("Decryption in progress, 0% done"), "{text}");
    assert!(text.contains("Reading the header"), "{text}");
    advance(&mut app, 1_500);
    let end = text_of(&screen(&app, 100, 28));
    assert!(end.contains("Decryption complete, 100% done"), "{end}");
    assert!(
        end.contains("Document decrypted, new fragment in the archives."),
        "{end}"
    );
    advance(&mut app, 5_000);
    assert!(text_of(&screen(&app, 100, 28)).contains("Document decrypted."));
}

#[test]
fn a_purchase_shows_the_debit_and_a_balance_that_goes_down_to_what_is_left() {
    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "buy");
    let text = text_of(&screen(&app, 100, 28));
    assert!(text.contains("Purchase: Ghost Protocol"), "{text}");
    assert!(text.contains("Debit: 80 credits"), "{text}");
    assert!(text.contains("Balance: 500 credits"), "{text}");
    advance(&mut app, 600);
    let middle = text_of(&screen(&app, 100, 28));
    let balance: u32 = middle
        .split("Balance: ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|number| number.parse().ok())
        .expect("a balance");
    assert!((420..500).contains(&balance), "{middle}");
    // 1.15 s of 1.2 s: the bar is full and the result is held.
    advance(&mut app, 550);
    let end = text_of(&screen(&app, 100, 28));
    assert!(end.contains("Balance: 420 credits"), "{end}");
    assert!(end.contains("Bought: Ghost Protocol."), "{end}");
}

#[test]
fn every_scene_obeys_the_rules_of_the_screen() {
    for scene in SCENES {
        // No colour: attributes only; nothing blinks; nothing outside seven bits in ASCII.
        let catalog = catalog_fr();
        let mut game = StageGame;
        let mode = RenderMode {
            screen_reader: false,
            ascii: true,
        };
        let mut app = stage_app(&mut game, &catalog, mode, None, true);
        type_line(&mut app, scene);
        for _ in 0..40 {
            for (width, height) in SIZES {
                for row in screen(&app, width, height) {
                    assert!(row.is_ascii(), "{scene}: {row:?}");
                }
                for cell in cells(&app, width, height) {
                    assert_eq!(cell.fg, Color::Reset, "{scene} {width}x{height}: {cell:?}");
                    assert_eq!(cell.bg, Color::Reset, "{scene} {width}x{height}: {cell:?}");
                }
            }
            app.tick(FRAME);
        }
        for choice in [
            PaletteChoice::Default,
            PaletteChoice::HighContrast,
            PaletteChoice::Cvd,
            PaletteChoice::Mono,
        ] {
            for truecolor in [false, true] {
                let catalog = catalog_en();
                let mut game = StageGame;
                let colors = Some(Palette::new(choice, truecolor));
                let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, colors, true);
                type_line(&mut app, scene);
                for _ in 0..40 {
                    for (width, height) in SIZES {
                        for cell in cells(&app, width, height) {
                            assert!(
                                !cell.modifier.contains(Modifier::SLOW_BLINK)
                                    && !cell.modifier.contains(Modifier::RAPID_BLINK),
                                "{scene} {choice:?}"
                            );
                            if choice == PaletteChoice::Mono {
                                assert_eq!(cell.fg, Color::Reset, "{scene}: {cell:?}");
                            }
                        }
                    }
                    app.tick(FRAME);
                }
            }
        }
    }
}

#[test]
fn every_scene_ends_by_itself_within_two_seconds_and_leaves_its_result() {
    for scene in SCENES {
        let (catalog, mut game) = (catalog_en(), StageGame);
        let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
        type_line(&mut app, scene);
        let mut frames = 0;
        while app.is_animating() {
            app.tick(FRAME);
            frames += 1;
            assert!(frames <= 40, "{scene} still plays after {frames} frames");
        }
        let rows = screen(&app, 100, 28);
        assert!(
            has(&rows, "Intrusion over.")
                || has(&rows, "Cover bought.")
                || has(&rows, "Document decrypted.")
                || has(&rows, "Bought."),
            "{scene}: {rows:?}"
        );
    }
}

#[test]
fn the_other_scenes_in_two_more_pictures() {
    let picture = |app: &App<'_>, width: u16, height: u16| {
        format!(
            "{width}x{height}\n{}",
            screen(app, width, height).join("\n")
        )
    };

    let (catalog, mut game) = (catalog_fr(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "decrypt");
    advance(&mut app, 800);
    insta::assert_snapshot!(
        "a_decryption_in_french_on_the_wide_tier",
        picture(&app, 100, 28)
    );

    let (catalog, mut game) = (catalog_en(), StageGame);
    let mut app = stage_app(&mut game, &catalog, RenderMode::FULL, None, true);
    type_line(&mut app, "buy");
    advance(&mut app, 600);
    insta::assert_snapshot!(
        "a_purchase_in_english_on_the_compact_tier",
        picture(&app, 64, 20)
    );
}
