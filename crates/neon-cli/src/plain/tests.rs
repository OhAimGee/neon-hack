use std::io::Cursor;

use neon_engine::demo::{DemoGame, catalog_en, catalog_fr};
use neon_engine::text::{RenderMode, StaticCatalog};
use neon_engine::{Game, Input, Prompt, Step, View};

use super::*;
use crate::render::Verbosity;

fn play(
    catalog: StaticCatalog,
    mode: RenderMode,
    seed: u64,
    input: &[u8],
    echo_input: bool,
) -> String {
    let renderer = Renderer {
        catalog: &catalog,
        mode,
        verbosity: Verbosity::Normal,
    };
    let mut out = Vec::new();
    run(
        &mut DemoGame::new(seed),
        &renderer,
        &mut Cursor::new(input),
        &mut out,
        echo_input,
    )
    .unwrap();
    String::from_utf8(out).unwrap()
}

fn english(input: &[u8]) -> String {
    play(catalog_en(), RenderMode::Full, 1, input, true)
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
[3] scan - Scan the network
[4] shop - Visit the stall
[5] status - Show your situation
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
        catalog_en(),
        RenderMode::Full,
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
    let run_seed = |seed| play(catalog_en(), RenderMode::Full, seed, script, true);
    assert_eq!(run_seed(5), run_seed(5));
    assert_ne!(run_seed(5), run_seed(6));
}

#[test]
fn french_and_screen_reader_modes_change_the_words_not_the_structure() {
    let script = b"\nNeon\ny\nscan\nquit\nn\nquit\ny\n";
    let french = play(catalog_fr(), RenderMode::Full, 1, script, true);
    assert!(french.contains("[Gain] +"));
    assert!(french.contains("Quitter le réseau ? [o/N] "));
    let reader = play(catalog_en(), RenderMode::ScreenReader, 1, script, true);
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
            save_requested: false,
        }
    }
    fn prompt(&self) -> Prompt {
        Prompt::Command
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
        mode: RenderMode::Full,
        verbosity: Verbosity::Normal,
    };
    let error = run(
        &mut Stubborn,
        &renderer,
        &mut Cursor::new(&b"one\ntwo\n"[..]),
        &mut Vec::new(),
        true,
    )
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
}
