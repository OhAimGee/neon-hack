use std::io::Cursor;

use neon_engine::demo::DemoGame;
use neon_engine::text::{Catalog, RenderMode};
use neon_engine::{Game, Input, Prompt, Step, View};

use super::*;
use crate::persist::Persistence;
use crate::render::Verbosity;
use crate::test_support::{catalog_en, catalog_fr};

fn play(catalog: &Catalog, mode: RenderMode, seed: u64, input: &[u8], echo_input: bool) -> String {
    let renderer = Renderer {
        catalog,
        mode,
        verbosity: Verbosity::Normal,
    };
    let mut out = Vec::new();
    let mut game = DemoGame::new(seed);
    let first = game.start();
    run(
        &mut game,
        first,
        &renderer,
        &mut Cursor::new(input),
        &mut out,
        echo_input,
        &mut Persistence::disabled(),
    )
    .unwrap();
    String::from_utf8(out).unwrap()
}

fn english(input: &[u8]) -> String {
    play(&catalog_en(), RenderMode::FULL, 1, input, true)
}

#[test]
fn a_short_session_reads_like_a_transcript() {
    let output = english(b"\nNeon\ny\nhelp\nquit\ny\n");
    // Pure atmosphere is hidden at normal verbosity; each line read is echoed, as on a pipe.
    let expected = "\
== N E O N   H A C K ==
On the screen, a single line: ENCRYPTED CHANNEL #7 - INCOMING CONNECTION.
[Enter] to continue

Your handle [Case]: Neon
Carve \"Neon\" into the net? [Y/n] y
ECHO-7 » Finally. I was starting to think you would never wake up, Neon.
ECHO-7 » Type 'help' to see what you can do.
> help
Commands
[1] help - List the commands
[2] quit - Leave the net
[3] save - Save the game (save 1 to 9)
[4] scan - Scan the network
[5] shop - Visit the stall
[6] status - Show your situation
> quit
Leave the net? [y/N] y
Goodbye, Neon.
";
    assert_eq!(output, expected);
}

#[test]
fn nothing_in_the_output_depends_on_a_terminal() {
    let output = english(b"\nNeon\ny\nscan\nshop\n1\n0\nquit\ny\n");
    assert!(
        !output.contains('\u{1b}'),
        "no escape sequence in plain output"
    );
    assert!(output.is_char_boundary(output.len()));
    assert!(output.ends_with("Goodbye, Neon.\n"), "{output:?}");
}

#[test]
fn echo_is_for_pipes_only() {
    let piped = english(b"\nNeon\ny\nstatus\nquit\ny\n");
    assert!(piped.contains("> status\n"));
    let terminal = play(
        &catalog_en(),
        RenderMode::FULL,
        1,
        b"\nNeon\ny\nstatus\nquit\ny\n",
        false,
    );
    assert!(
        !terminal.contains("> status\n"),
        "the terminal echoes by itself"
    );
    assert!(terminal.contains("Handle: Neon"));
}

#[test]
fn closing_the_input_ends_the_game_wherever_it_happens() {
    let at_the_start = english(b"");
    assert!(at_the_start.ends_with("Input closed: ending the session.\n"));
    let in_a_menu = english(b"\nNeon\ny\nshop\n");
    assert!(
        in_a_menu.ends_with("Input closed: ending the session.\n"),
        "{in_a_menu}"
    );
    // The prompt left without a newline is closed before the farewell.
    assert!(in_a_menu.contains("? \nInput closed"), "{in_a_menu:?}");
}

#[test]
fn a_bad_yes_or_no_is_asked_again_without_bothering_the_engine() {
    let output = english(b"\nNeon\nperhaps\nyes\nquit\nn\nquit\ny\n");
    assert!(
        output.contains("[Error] Please answer yes or no.\n"),
        "{output}"
    );
    // The refused answer did not confirm anything; "yes" afterwards did.
    assert!(output.contains("ECHO-7 » Finally"));
    // Declining to quit goes back to the command line.
    assert_eq!(output.matches("Leave the net? [y/N] ").count(), 2);
}

#[test]
fn invalid_utf8_does_not_end_the_game() {
    let output = english(b"\nNeon\ny\n\xff\xfe\nquit\ny\n");
    assert!(
        output.contains("[Error] Unknown command: \u{fffd}"),
        "{output}"
    );
    assert!(output.ends_with("Goodbye, Neon.\n"));
}

#[test]
fn the_same_seed_gives_the_same_bytes() {
    let script = b"\nNeon\ny\nscan\nscan\nscan\nquit\ny\n";
    let run_seed = |seed| play(&catalog_en(), RenderMode::FULL, seed, script, true);
    assert_eq!(run_seed(5), run_seed(5));
    assert_ne!(run_seed(5), run_seed(6));
}

#[test]
fn french_and_screen_reader_modes_change_the_words_not_the_structure() {
    let script = b"\nNeon\ny\nscan\nquit\nn\nquit\ny\n";
    let french = play(&catalog_fr(), RenderMode::FULL, 1, script, true);
    assert!(french.contains("[Gain] +"));
    assert!(french.contains("Quitter le réseau ? [o/N] "));
    let reader = play(&catalog_en(), RenderMode::SCREEN_READER, 1, script, true);
    assert!(reader.contains("Trace up by "), "{reader}");
    assert!(reader.contains("ECHO-7: Finally"));
    assert!(
        !reader.contains("NEON HACK"),
        "decoration is dropped for screen readers"
    );
}

/// A broken engine that ignores the end of the input.
struct Stubborn;

impl Game for Stubborn {
    fn start(&mut self) -> Step {
        self.handle(Input::Cancel)
    }
    fn handle(&mut self, _input: Input) -> Step {
        Step {
            events: Vec::new(),
            prompt: Prompt::Command,
            save: None,
        }
    }
    fn prompt(&self) -> Prompt {
        Prompt::Command
    }
    fn snapshot(&self) -> Result<String, neon_engine::save::SaveError> {
        Ok(String::new())
    }
    fn view(&self) -> View {
        View {
            player: String::new(),
            gauges: Vec::new(),
            objectives: Vec::new(),
        }
    }
    fn complete(&self, _line: &str) -> Vec<String> {
        Vec::new()
    }
}

#[test]
fn an_engine_that_will_not_end_cannot_trap_the_frontend_in_a_loop() {
    let catalog = catalog_en();
    let renderer = Renderer {
        catalog: &catalog,
        mode: RenderMode::FULL,
        verbosity: Verbosity::Normal,
    };
    let mut game = Stubborn;
    let first = game.start();
    let error = run(
        &mut game,
        first,
        &renderer,
        &mut Cursor::new(&b"one\ntwo\n"[..]),
        &mut Vec::new(),
        true,
        &mut Persistence::disabled(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
}

#[test]
fn a_frontend_attached_to_a_game_under_way_continues_without_replaying_the_start() {
    let catalog = catalog_en();
    let renderer = Renderer {
        catalog: &catalog,
        mode: RenderMode::FULL,
        verbosity: Verbosity::Full,
    };
    // Play the prologue elsewhere (another frontend, or a load), then attach this one.
    let mut game = DemoGame::new(1);
    game.start();
    for input in [
        Input::Continue,
        Input::Line("Neon".to_owned()),
        Input::Confirm(true),
    ] {
        game.handle(input);
    }
    let attach = game.resume();
    let mut out = Vec::new();
    run(
        &mut game,
        attach,
        &renderer,
        &mut Cursor::new(&b"status\nquit\ny\n"[..]),
        &mut out,
        true,
        &mut Persistence::disabled(),
    )
    .unwrap();
    let output = String::from_utf8(out).unwrap();
    assert!(output.starts_with("> status\n"), "{output}");
    assert!(!output.contains("NEON HACK"), "the logo is not shown again");
    assert!(!output.contains("Neo-Tokyo"), "the intro is not replayed");
    assert!(
        !output.contains("Your handle"),
        "the prologue is not asked again"
    );
    assert!(output.contains("Handle: Neon"), "the game state is kept");
}

#[test]
fn ascii_mode_keeps_the_echo_of_typed_lines_seven_bit_too() {
    let script = "\nZoë\no\nscan\nquit\no\n".as_bytes();
    let ascii = play(&catalog_fr(), RenderMode::ASCII, 1, script, true);
    assert!(ascii.is_ascii(), "{ascii}");
    assert!(ascii.contains("Votre pseudo [Case] : Zoe\n"), "{ascii}");
    assert!(ascii.contains("> scan\n"));
    // The same session keeps its accents everywhere else.
    let full = play(&catalog_fr(), RenderMode::FULL, 1, script, true);
    assert!(full.contains("Votre pseudo [Case] : Zoë\n"), "{full}");
}

#[test]
fn saves_are_written_as_the_game_goes_and_manual_ones_are_acknowledged() {
    let dir = tempfile::tempdir().unwrap();
    let saves = dir.path().join("saves");
    let renderer = Renderer {
        catalog: &catalog_en(),
        mode: RenderMode::FULL,
        verbosity: Verbosity::Normal,
    };
    let mut persistence = Persistence::new(crate::store::Store::new(saves.clone()));
    let mut game = DemoGame::new(7);
    let first = game.start();
    let mut out = Vec::new();
    run(
        &mut game,
        first,
        &renderer,
        &mut Cursor::new(&b"\nNeon\ny\nscan\nsave 2\nsave 12\nshop\n0\nquit\ny\n"[..]),
        &mut out,
        true,
        &mut persistence,
    )
    .unwrap();
    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("Game saved in slot 2.\n"), "{output}");
    assert!(output.contains("Slots go from 1 to 9."), "{output}");
    for file in ["auto.toml", "slot-2.toml", "checkpoint-1.toml"] {
        assert!(saves.join(file).exists(), "{file} is missing");
    }
    // The autosave of the last action (quitting) holds the game at the command line.
    let text = std::fs::read_to_string(saves.join("auto.toml")).unwrap();
    assert_eq!(
        DemoGame::from_save(&text).unwrap().prompt(),
        Prompt::Command
    );
}
