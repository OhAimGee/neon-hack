//! `neon-hack`: the command-line entry point.
//!
//! Phase R1.1: the engine contract is exercised by a small demo game (`--demo`) through
//! two frontends, plain and TUI. The real game arrives with phase R2 (see
//! `docs/ROADMAP.md`); without `--demo` the binary still says it is not playable yet.

mod input;
mod plain;
mod render;
#[cfg(test)]
mod test_support;
#[cfg(feature = "tui")]
mod tui;

use std::io::{self, BufReader, IsTerminal};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, ValueEnum};
use neon_engine::Game;
use neon_engine::demo::DemoGame;
use neon_engine::text::{Catalog, Lang, RenderMode};

use crate::render::{Renderer, Verbosity};

/// Neon Hack: a cyberpunk text RPG for the terminal.
#[derive(Debug, Parser)]
#[command(name = "neon-hack", version)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent command-line flags, not a state machine"
)]
struct Cli {
    /// Play the engine demo (development): a tiny game that exercises both frontends.
    #[arg(long)]
    demo: bool,

    /// Use the plain line-by-line interface even on a terminal.
    #[arg(long)]
    plain: bool,

    /// Screen-reader mode: the plain interface with wording that needs no symbols read aloud.
    #[arg(long)]
    screen_reader: bool,

    /// 7-bit ASCII output: ASCII symbols, no decoration, accents and typed text transliterated.
    #[arg(long)]
    ascii: bool,

    /// Language of the texts.
    #[arg(long, value_enum, default_value_t = LangArg::En)]
    lang: LangArg,

    /// How much atmosphere to show.
    #[arg(long, value_enum, default_value_t = VerbosityArg::Normal)]
    verbosity: VerbosityArg,

    /// Seed of the random number generator (reproducible games).
    #[arg(long)]
    seed: Option<u64>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum LangArg {
    En,
    Fr,
}

impl From<LangArg> for Lang {
    fn from(value: LangArg) -> Self {
        match value {
            LangArg::En => Self::En,
            LangArg::Fr => Self::Fr,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum VerbosityArg {
    Brief,
    Normal,
    Full,
}

impl From<VerbosityArg> for Verbosity {
    fn from(value: VerbosityArg) -> Self {
        match value {
            VerbosityArg::Brief => Self::Brief,
            VerbosityArg::Normal => Self::Normal,
            VerbosityArg::Full => Self::Full,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if !cli.demo {
        banner::not_playable_yet();
        return ExitCode::SUCCESS;
    }
    match run_demo(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            banner::fatal(&error);
            ExitCode::FAILURE
        }
    }
}

fn run_demo(cli: &Cli) -> io::Result<()> {
    let seed = cli.seed.unwrap_or_else(clock_seed);
    let mut game = DemoGame::new(seed);
    // The catalogs are checked by the tests, so this only fails on a broken build.
    let catalog = Catalog::embedded(cli.lang.into())
        .map_err(|errors| io::Error::other(format!("the texts do not load:\n{errors}")))?;
    let mode = if cli.screen_reader {
        RenderMode::ScreenReader
    } else if cli.ascii {
        RenderMode::Ascii
    } else {
        RenderMode::Full
    };
    let renderer = Renderer {
        catalog: &catalog,
        mode,
        verbosity: cli.verbosity.into(),
    };
    // A new game: the owner starts it once and hands the step to whichever frontend runs.
    let first = game.start();

    let stdin = io::stdin();
    let interactive = stdin.is_terminal() && io::stdout().is_terminal();
    // The TUI needs a real terminal; a screen reader needs the plain interface.
    let use_tui = cfg!(feature = "tui") && interactive && !cli.plain && !cli.screen_reader;
    if use_tui {
        #[cfg(feature = "tui")]
        return tui::run(&mut game, first, renderer);
    }
    // On a pipe the lines read are repeated in the output so that it reads as a transcript;
    // on a terminal the terminal echoes by itself.
    let echo_input = !stdin.is_terminal();
    plain::run(
        &mut game,
        first,
        &renderer,
        &mut BufReader::new(stdin.lock()),
        &mut io::stdout().lock(),
        echo_input,
    )
}

/// A seed from the clock. The engine never reads the clock: it receives a number.
fn clock_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| u64::try_from(elapsed.as_nanos()).ok())
        .unwrap_or(0)
}

/// Console messages that do not come from a frontend.
mod banner {
    #![allow(clippy::print_stdout, clippy::print_stderr)]

    pub(super) fn not_playable_yet() {
        println!(
            "Neon Hack {}: not playable yet, the Rust rewrite is in progress (see docs/ROADMAP.md).",
            env!("CARGO_PKG_VERSION")
        );
    }

    pub(super) fn fatal(error: &std::io::Error) {
        eprintln!("neon-hack: {error}");
    }
}
