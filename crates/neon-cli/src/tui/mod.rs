//! The full-screen interface.
//!
//! [`app`] holds everything testable (state, keys, drawing), [`editor`] the input line and
//! [`text`] the measuring and cutting of text; none of them touches the terminal, and they
//! are tested with ratatui's `TestBackend`. [`term`] takes the terminal and always gives it
//! back. This file is the glue with the real terminal, tested by running the real binary in
//! a pseudo-terminal (`tests/pty.rs`).
//!
//! The screen is redrawn when the state can have changed (a key, a paste, a new size) and
//! never otherwise: the engine has no clock, so an idle interface writes nothing. The one
//! exception is a staged scene ([`motion`]): while one plays, which is at most two seconds
//! and ends at the first key, the screen is redrawn every [`motion::FRAME`].

mod app;
mod editor;
mod motion;
mod term;
mod text;
mod typing;

use std::io::{self, BufReader};
use std::time::{Duration, Instant};

use neon_engine::command::Capabilities;
use neon_engine::event::Event as GameEvent;
use neon_engine::text::Text;
use neon_engine::{Game, Step};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::config::TypewriterChoice;
use crate::options::Host;
use crate::persist::Persistence;
use crate::plain;
use crate::render::Renderer;
use app::{App, Exit};

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

/// How the interface moves: whether scenes play and how fast the text is typed out.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Motion {
    /// `--reduce-motion`: no scene, no typing.
    pub(crate) reduced: bool,
    pub(crate) typewriter: TypewriterChoice,
}

/// Runs the game in the full-screen interface until it ends. The terminal is given back
/// whatever the way out. If the player asks for the line-by-line display (`Ctrl+P`, `plain`)
/// the game goes on there, from the state it is in: the frontend is a throwaway view.
pub(crate) fn run<'a>(
    game: &'a mut dyn Game,
    first: Step,
    renderer: Renderer<'a>,
    persistence: Persistence,
    motion: Motion,
    options: Option<Host<'a>>,
    test_fault: Option<&str>,
) -> io::Result<()> {
    // The interface runs `panel` and `plain`: the game may now offer them.
    game.set_capabilities(Capabilities::TUI);
    let mut app = App::new(game, renderer, first, persistence)
        .with_motion(!motion.reduced)
        .with_typewriter(motion.typewriter);
    if let Some(options) = options {
        // Only the campaign has the presentation commands, and `panel` and `plain` with them.
        app = app.with_options(options).with_frontend_commands();
    }
    {
        // The terminal goes back to the player at the end of this block, before anything else
        // is written to it.
        let (mut terminal, _guard) = term::enter()?;
        event_loop(&mut terminal, &mut app, test_fault.and_then(Fault::parse))?;
    }
    if app.exit() == Some(Exit::Plain) {
        return hand_over(app);
    }
    Ok(())
}

/// Plays on in the plain frontend, with the game, the settings and the saves of the
/// interface that is leaving. The step is the one a loaded game starts from: nothing is
/// replayed, and the situation is told again, since the log of the full-screen interface is
/// gone with its screen.
fn hand_over(app: App<'_>) -> io::Result<()> {
    let (game, renderer, mut options, mut persistence) = app.into_parts();
    // `panel` and `plain` are the full-screen interface's: the game stops offering them.
    game.set_capabilities(Capabilities::PLAIN);
    let mut first = game.resume();
    first
        .events
        .insert(0, GameEvent::system(Text::new("ui.tui.switched_to_plain")));
    plain::run_with_options(
        game,
        first,
        &renderer,
        options.as_mut(),
        &mut BufReader::new(io::stdin().lock()),
        &mut io::stdout().lock(),
        // The terminal echoes what is typed.
        false,
        &mut persistence,
    )
}

fn event_loop(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    mut fault: Option<Fault>,
) -> io::Result<()> {
    let mut dirty = true;
    // When the scene being staged was last moved forward; only meaningful while one plays.
    let mut last_frame = Instant::now();
    let mut animating = false;
    loop {
        if dirty {
            terminal.draw(|frame| app.draw(frame))?;
            dirty = false;
            match fault.take() {
                Some(Fault::Panic) => panic!("forced panic (NEON_HACK_TEST_FAULT=panic)"),
                Some(Fault::Error) => {
                    return Err(io::Error::other(
                        "forced error (NEON_HACK_TEST_FAULT=error)",
                    ));
                }
                None => {}
            }
        }
        // A scene just began: its clock starts now, not when the last key was read.
        if app.is_animating() && !animating {
            last_frame = Instant::now();
        }
        animating = app.is_animating();
        let mut next = if animating {
            // Wait for a key only until the next frame is due.
            let waited = last_frame.elapsed();
            if !event::poll(motion::FRAME.saturating_sub(waited))? {
                let now = Instant::now();
                app.tick(now.duration_since(last_frame));
                last_frame = now;
                dirty = true;
                continue;
            }
            event::read()?
        } else {
            // Nothing moves: block until something happens, and write nothing meanwhile.
            event::read()?
        };
        for _ in 0..BATCH_MAX {
            dirty |= handle(app, next);
            if app.exit().is_some() {
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
