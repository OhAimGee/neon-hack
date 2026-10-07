//! The full-screen interface.
//!
//! [`app`] holds everything testable (state, keys, drawing). This file is only the glue
//! with the real terminal, which needs a pseudo-terminal to be tested (phase R3).

mod app;

use std::io;

use neon_engine::{Game, Step};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::persist::Persistence;
use crate::render::Renderer;
use app::App;

/// Runs the game in the full-screen interface until it ends.
pub(crate) fn run(
    game: &mut dyn Game,
    first: Step,
    renderer: Renderer<'_>,
    persistence: Persistence,
) -> io::Result<()> {
    let mut app = App::new(game, renderer, first, persistence);
    // `try_init` also installs a panic hook that gives the terminal back.
    let mut terminal = ratatui::try_init()?;
    let result = event_loop(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App<'_>) -> io::Result<()> {
    loop {
        terminal.draw(|frame| app.draw(frame))?;
        // Windows also reports key releases: only presses count.
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.on_key(key);
        }
        if app.should_quit() {
            return Ok(());
        }
    }
}
