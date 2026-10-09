//! The full-screen interface.
//!
//! [`app`] holds everything testable (state, keys, drawing), [`editor`] the input line and
//! [`text`] the measuring and cutting of text; none of them touches the terminal, and they
//! are tested with ratatui's `TestBackend`. [`term`] takes the terminal and always gives it
//! back. This file is the glue with the real terminal, tested by running the real binary in
//! a pseudo-terminal (`tests/pty.rs`).
//!
//! The screen is redrawn when the state can have changed (a key, a paste, a new size) and
//! never otherwise: the engine has no clock, so there is no timer, and an idle interface
//! writes nothing.

mod app;
mod editor;
mod term;
mod text;

use std::io;
use std::time::Duration;

use neon_engine::{Game, Step};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::persist::Persistence;
use crate::render::Renderer;
use app::App;

pub(crate) use app::COMPACT_SIZE;

/// Events handled before the screen is drawn again, so that a burst (a held key, a resize
/// storm) is one drawing and not hundreds.
const BATCH_MAX: usize = 256;

/// The size of the terminal, if it can be known.
pub(crate) fn terminal_size() -> Option<(u16, u16)> {
    ratatui::crossterm::terminal::size().ok()
}

/// A failure the tests ask for with `NEON_HACK_TEST_FAULT`, to check that the terminal is
/// given back on the way out. Only read in debug builds (see `Env::from_process`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    /// The interface panics after its first drawing.
    Panic,
    /// The interface fails with an I/O error after its first drawing.
    Error,
}

impl Fault {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "panic" => Some(Self::Panic),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// Runs the game in the full-screen interface until it ends. The terminal is given back
/// whatever the way out.
pub(crate) fn run(
    game: &mut dyn Game,
    first: Step,
    renderer: Renderer<'_>,
    persistence: Persistence,
    test_fault: Option<&str>,
) -> io::Result<()> {
    let mut app = App::new(game, renderer, first, persistence);
    let (mut terminal, _guard) = term::enter()?;
    event_loop(&mut terminal, &mut app, test_fault.and_then(Fault::parse))
}

fn event_loop(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    mut fault: Option<Fault>,
) -> io::Result<()> {
    let mut dirty = true;
    loop {
        if dirty {
            terminal.draw(|frame| app.draw(frame))?;
            dirty = false;
            match fault.take() {
                Some(Fault::Panic) => panic!("forced panic (NEON_HACK_TEST_FAULT=panic)"),
                Some(Fault::Error) => {
                    return Err(io::Error::other("forced error (NEON_HACK_TEST_FAULT=error)"));
                }
                None => {}
            }
        }
        let mut next = event::read()?;
        for _ in 0..BATCH_MAX {
            dirty |= handle(app, next);
            if app.should_quit() {
                return Ok(());
            }
            if event::poll(Duration::ZERO)? {
                next = event::read()?;
            } else {
                break;
            }
        }
    }
}

/// Gives one terminal event to the interface; true if the screen can look different.
fn handle(app: &mut App<'_>, event: Event) -> bool {
    match event {
        // Windows also reports key releases: only presses (and auto-repeats) count.
        Event::Key(key) if key.kind != KeyEventKind::Release => {
            app.on_key(key);
            true
        }
        Event::Paste(text) => {
            app.on_paste(&text);
            true
        }
        // The next drawing takes the new size.
        Event::Resize(..) => true,
        // Nothing to do for focus, and the mouse is never captured.
        Event::Key(_) | Event::FocusGained | Event::FocusLost | Event::Mouse(_) => false,
    }
}
