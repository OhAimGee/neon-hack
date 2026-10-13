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
use crate::loading::{self, Answer, Loading};
use crate::options::Host;
use crate::persist::Persistence;
use crate::render::{Line, Renderer};
use crate::transcript::Transcript;

/// [`run_with_options`] with no presentation commands, as the tests play it.
#[cfg(test)]
pub(crate) fn run(
    game: &mut dyn Game,
    first: Step,
    renderer: &Renderer<'_>,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    echo_input: bool,
    persistence: &mut Persistence,
) -> io::Result<()> {
    run_with_options(
        game,
        first,
        renderer,
        None,
        input,
        out,
        echo_input,
        persistence,
    )
}

/// Plays a game to its end, starting from `first`: the step of a new game
/// ([`Game::start`]) or the current state of a game under way ([`Game::resume`]).
///
/// `echo_input` repeats each line read in the output (`> scan`), which a pipe needs to
/// give a readable transcript; on a terminal the terminal itself echoes what is typed.
///
/// The presentation commands (`options`, `verbosity`) are run by the interface when a `host`
/// is given: they change what is written from then on, never the game.
#[allow(
    clippy::too_many_arguments,
    reason = "the seven things a session is made of, named at the one call site and in the tests"
)]
pub(crate) fn run_with_options<'a>(
    game: &mut dyn Game,
    first: Step,
    renderer: &Renderer<'a>,
    mut host: Option<&mut Host<'a>>,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    echo_input: bool,
    persistence: &mut Persistence,
) -> io::Result<()> {
    // The settings can change in the middle of the game: this copy follows them.
    let mut renderer = *renderer;
    let renderer = &mut renderer;
    let mut step = first;
    let mut closed = false;
    // What is said and typed, for `export`.
    let mut output = Output {
        out,
        transcript: Transcript::new(),
    };
    // `load` belongs to the campaign, like the other commands of the interface.
    let mut loading = Loading::new();
    loop {
        write_step(&mut output, renderer, &step)?;
        if step.prompt == Prompt::End {
            return Ok(());
        }
        if closed {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the game did not end after the input was closed",
            ));
        }
        let load = host.is_some().then_some(LoadContext {
            loading: &mut loading,
            game: &mut *game,
            persistence: &*persistence,
        });
        let read = read_input(
            input,
            &mut output,
            renderer,
            host.as_deref_mut(),
            &step.prompt,
            echo_input,
            load,
        )?;
        let engine_input = match read {
            Read::Input(typed) => typed,
            Read::Loaded(loaded) => {
                // A different game, told and shown from where it is.
                step = loaded;
                continue;
            }
            Read::Closed => {
                closed = true;
                writeln!(output.out)?;
                Input::Eof
            }
        };
        step = game.handle(engine_input);
        persistence.after_step(&*game, &mut step);
    }
}

/// Where the lines go: the output, and the transcript that keeps them.
struct Output<'o> {
    out: &'o mut dyn Write,
    transcript: Transcript,
}

impl Output<'_> {
    /// Writes the lines of events, painted, and keeps their text.
    fn events(&mut self, renderer: &Renderer<'_>, events: &[Event]) -> io::Result<()> {
        for event in events {
            self.lines(renderer, &renderer.event(event))?;
        }
        Ok(())
    }

    fn lines(&mut self, renderer: &Renderer<'_>, lines: &[Line]) -> io::Result<()> {
        for line in lines {
            writeln!(self.out, "{}", renderer.paint(line))?;
            self.transcript.record([line.text.clone()]);
        }
        Ok(())
    }

    /// The marker that asks for a line, left at the end of the row.
    fn ask(&mut self, marker: &str) -> io::Result<()> {
        write!(self.out, "{marker}")?;
        self.out.flush()
    }
}

fn write_step(output: &mut Output<'_>, renderer: &Renderer<'_>, step: &Step) -> io::Result<()> {
    output.events(renderer, &step.events)?;
    let view = renderer.prompt(&step.prompt);
    output.lines(renderer, &view.header)?;
    output.ask(&view.marker)
}

/// What the game and the saves lend to the reading of a line, for `load`.
struct LoadContext<'g> {
    loading: &'g mut Loading,
    game: &'g mut dyn Game,
    persistence: &'g Persistence,
}

/// What came of reading.
enum Read {
    /// A line the game is to hear.
    Input(Input),
    /// A save was loaded: the step to carry on from.
    Loaded(Step),
    /// The input is closed.
    Closed,
}

/// Reads until a line makes sense for the prompt, or means a game loaded.
fn read_input<'a>(
    input: &mut dyn BufRead,
    output: &mut Output<'_>,
    renderer: &mut Renderer<'a>,
    mut host: Option<&mut Host<'a>>,
    prompt: &Prompt,
    echo_input: bool,
    mut load: Option<LoadContext<'_>>,
) -> io::Result<Read> {
    loop {
        // While `load` waits for its yes, that is the question being asked.
        let question = load.as_ref().and_then(|load| load.loading.question());
        let asking = question.as_ref().unwrap_or(prompt);
        let mut bytes = Vec::new();
        if input.read_until(b'\n', &mut bytes)? == 0 {
            return Ok(Read::Closed);
        }
        // Invalid UTF-8 becomes U+FFFD rather than an error: a stray byte must not end a game.
        let text = String::from_utf8_lossy(&bytes);
        let line = text.trim_end_matches(['\n', '\r']);
        if echo_input {
            writeln!(output.out, "{}", renderer.verbatim(line))?;
        }
        // The line as typed, after the marker that asked for it: what a transcript reads.
        output.transcript.record([format!(
            "{}{}",
            renderer.prompt(asking).marker,
            renderer.verbatim(line)
        )]);
        // The commands of the interface are its own: the game never hears of them.
        if *asking == Prompt::Command
            && let Some(host) = host.as_deref_mut()
            && let Some(events) = host.run(line, &output.transcript)
        {
            host.apply(renderer);
            output.events(renderer, &events)?;
            output.ask(&renderer.prompt(prompt).marker)?;
            continue;
        }
        if let Some(load) = load.as_mut() {
            if *asking == Prompt::Command
                && let Some(request) = loading::request(line)
            {
                let events = load.loading.run(request, load.persistence);
                output.events(renderer, &events)?;
                let next = load.loading.question();
                output.ask(&renderer.prompt(next.as_ref().unwrap_or(prompt)).marker)?;
                continue;
            }
            if question.is_some() {
                match to_input(asking, line) {
                    Ok(answer) => match load.loading.answer(&answer, &mut *load.game) {
                        Answer::Said(events) => {
                            output.events(renderer, &events)?;
                            let next = load.loading.question();
                            output.ask(&renderer.prompt(next.as_ref().unwrap_or(prompt)).marker)?;
                        }
                        Answer::Loaded(step) => return Ok(Read::Loaded(step)),
                        Answer::Closed => return Ok(Read::Input(Input::Eof)),
                    },
                    Err(InputError::NotYesOrNo) => {
                        output
                            .events(renderer, &[Event::error(Text::new("ui.invalid_confirm"))])?;
                        output.ask(&renderer.prompt(asking).marker)?;
                    }
                }
                continue;
            }
        }
        match to_input(prompt, line) {
            Ok(engine_input) => return Ok(Read::Input(engine_input)),
            Err(InputError::NotYesOrNo) => {
                output.events(renderer, &[Event::error(Text::new("ui.invalid_confirm"))])?;
                output.ask(&renderer.prompt(prompt).marker)?;
            }
        }
    }
}

#[cfg(test)]
mod tests;
