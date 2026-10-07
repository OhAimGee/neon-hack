//! The plain frontend: line by line, linear, deterministic.
//!
//! It is the reference frontend, the one of pipes, tests, transcripts and screen readers.
//! It never addresses the cursor, never wraps a paragraph, and never loops forever: when
//! the input closes, the engine is told and must end the game.

use std::io::{self, BufRead, Write};

use neon_engine::event::Event;
use neon_engine::text::Text;
use neon_engine::{Game, Input, Prompt, Step};

use crate::input::{InputError, to_input};
use crate::render::Renderer;

/// Plays a game to its end, starting from `first`: the step of a new game
/// ([`Game::start`]) or the current state of a game under way ([`Game::resume`]).
///
/// `echo_input` repeats each line read in the output (`> scan`), which a pipe needs to
/// give a readable transcript; on a terminal the terminal itself echoes what is typed.
pub(crate) fn run(
    game: &mut dyn Game,
    first: Step,
    renderer: &Renderer<'_>,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    echo_input: bool,
) -> io::Result<()> {
    let mut step = first;
    let mut closed = false;
    loop {
        write_step(out, renderer, &step)?;
        if step.prompt == Prompt::End {
            return Ok(());
        }
        if closed {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the game did not end after the input was closed",
            ));
        }
        let engine_input =
            if let Some(typed) = read_input(input, out, renderer, &step.prompt, echo_input)? {
                typed
            } else {
                closed = true;
                writeln!(out)?;
                Input::Eof
            };
        step = game.handle(engine_input);
    }
}

fn write_step(out: &mut dyn Write, renderer: &Renderer<'_>, step: &Step) -> io::Result<()> {
    for event in &step.events {
        for line in renderer.event(event) {
            writeln!(out, "{}", line.text)?;
        }
    }
    let view = renderer.prompt(&step.prompt);
    for line in &view.header {
        writeln!(out, "{}", line.text)?;
    }
    write!(out, "{}", view.marker)?;
    out.flush()
}

/// Reads until a line makes sense for the prompt. `None` means the input is closed.
fn read_input(
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    renderer: &Renderer<'_>,
    prompt: &Prompt,
    echo_input: bool,
) -> io::Result<Option<Input>> {
    loop {
        let mut bytes = Vec::new();
        if input.read_until(b'\n', &mut bytes)? == 0 {
            return Ok(None);
        }
        // Invalid UTF-8 becomes U+FFFD rather than an error: a stray byte must not end a game.
        let text = String::from_utf8_lossy(&bytes);
        let line = text.trim_end_matches(['\n', '\r']);
        if echo_input {
            writeln!(out, "{line}")?;
        }
        match to_input(prompt, line) {
            Ok(engine_input) => return Ok(Some(engine_input)),
            Err(InputError::NotYesOrNo) => {
                let error = Event::error(Text::new("ui.invalid_confirm"));
                for rendered in renderer.event(&error) {
                    writeln!(out, "{}", rendered.text)?;
                }
                write!(out, "{}", renderer.prompt(prompt).marker)?;
                out.flush()?;
            }
        }
    }
}

#[cfg(test)]
mod tests;
