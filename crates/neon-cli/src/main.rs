//! `neon-hack`: the command-line entry point.
//!
//! The engine contract and saving are exercised by a small demo game (`--demo`) through
//! two frontends, plain and TUI. The real game arrives with phase R2 (see
//! `docs/ROADMAP.md`); without `--demo` the binary still says it is not playable yet.

mod input;
mod persist;
mod plain;
mod render;
mod store;
#[cfg(test)]
mod test_support;
#[cfg(feature = "tui")]
mod tui;

use std::io::{self, BufReader, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, ValueEnum};
use directories::ProjectDirs;
use neon_engine::demo::DemoGame;
use neon_engine::event::Event;
use neon_engine::text::{Catalog, Lang, RenderMode, Text};
use neon_engine::{Game, Step};

use crate::persist::Persistence;
use crate::render::{Renderer, Verbosity};
use crate::store::{Store, Target};

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

    /// Start a new game instead of resuming the autosave (the previous autosave is kept
    /// as `auto.toml.bak`).
    #[arg(long, requires = "demo", conflicts_with = "load")]
    new: bool,

    /// Resume this save instead of the autosave: auto, checkpoint-1 to checkpoint-3, or
    /// slot-1 to slot-9.
    #[arg(long, requires = "demo", value_name = "SAVE", value_parser = Target::parse)]
    load: Option<Target>,

    /// List the saves and exit.
    #[arg(long, requires = "demo")]
    list_saves: bool,

    /// Play without writing any save.
    #[arg(long, requires = "demo")]
    no_save: bool,

    /// Folder for the saves, instead of the data folder of the system.
    #[arg(long, requires = "demo", value_name = "DIR")]
    data_dir: Option<PathBuf>,
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
    // The catalogs are checked by the tests, so this only fails on a broken build.
    let catalog = Catalog::embedded(cli.lang.into())
        .map_err(|errors| io::Error::other(format!("the texts do not load:\n{errors}")))?;
    // The two are independent and can be combined.
    let mode = RenderMode {
        screen_reader: cli.screen_reader,
        ascii: cli.ascii,
    };
    let renderer = Renderer {
        catalog: &catalog,
        mode,
        verbosity: cli.verbosity.into(),
    };
    let store = Store::new(saves_dir(cli)?);
    if cli.list_saves {
        return list_saves(&store, &renderer, &mut io::stdout().lock());
    }
    // The owner of the game starts or resumes it once and hands the step to whichever
    // frontend runs.
    let (mut game, first) = open_game(cli, &store, &renderer)?;
    let persistence = if cli.no_save {
        Persistence::disabled()
    } else {
        Persistence::new(store)
    };

    let stdin = io::stdin();
    let interactive = stdin.is_terminal() && io::stdout().is_terminal();
    // The TUI needs a real terminal; a screen reader needs the plain interface.
    let use_tui = cfg!(feature = "tui") && interactive && !cli.plain && !cli.screen_reader;
    if use_tui {
        #[cfg(feature = "tui")]
        return tui::run(&mut game, first, renderer, persistence);
    }
    // On a pipe the lines read are repeated in the output so that it reads as a transcript;
    // on a terminal the terminal echoes by itself.
    let echo_input = !stdin.is_terminal();
    let mut persistence = persistence;
    plain::run(
        &mut game,
        first,
        &renderer,
        &mut BufReader::new(stdin.lock()),
        &mut io::stdout().lock(),
        echo_input,
        &mut persistence,
    )
}

/// The folder of the saves: `--data-dir`, or the data folder of the system.
fn saves_dir(cli: &Cli) -> io::Result<PathBuf> {
    if let Some(dir) = &cli.data_dir {
        return Ok(dir.join("saves"));
    }
    ProjectDirs::from("", "", "neon-hack")
        .map(|dirs| dirs.data_dir().join("saves"))
        .ok_or_else(|| io::Error::other("no data folder found for this user: use --data-dir"))
}

/// A game to play: the save asked for (the autosave by default), or a new one.
fn open_game(cli: &Cli, store: &Store, renderer: &Renderer<'_>) -> io::Result<(DemoGame, Step)> {
    if !cli.new {
        let target = cli.load.unwrap_or(Target::Auto);
        match store.read(target, DemoGame::from_save) {
            Ok(Some(loaded)) => {
                let notice = if loaded.from_backup {
                    "ui.save.resumed_backup"
                } else {
                    "ui.save.resumed"
                };
                let mut step = loaded.value.resume();
                step.events.push(Event::system(Text::new(notice)));
                return Ok((loaded.value, step));
            }
            Ok(None) if cli.load.is_some() => {
                let missing = Text::new("ui.save.missing").with_str("save", target.label());
                return Err(io::Error::other(renderer.text(&missing)));
            }
            Ok(None) => {}
            Err(error) => {
                let unreadable =
                    Text::new("ui.save.unreadable").with_str("details", error.to_string());
                return Err(io::Error::other(renderer.text(&unreadable)));
            }
        }
    }
    let mut game = DemoGame::new(cli.seed.unwrap_or_else(clock_seed));
    let first = game.start();
    Ok((game, first))
}

fn list_saves(store: &Store, renderer: &Renderer<'_>, out: &mut dyn Write) -> io::Result<()> {
    let folder = Text::new("ui.save.folder").with_str("folder", store.dir().display().to_string());
    writeln!(out, "{}", renderer.text(&folder))?;
    let listing = store.list();
    if listing.is_empty() {
        return writeln!(out, "{}", renderer.text(&Text::new("ui.save.none")));
    }
    for entry in listing {
        let save = Text::raw(entry.target.label());
        let line = match entry.meta {
            Ok(meta) => Text::new("ui.save.list_entry")
                .with_text("save", save)
                .with_str("player", meta.player)
                .with_int("turn", i64::from(meta.turn)),
            Err(reason) => Text::new("ui.save.list_damaged")
                .with_text("save", save)
                .with_str("reason", reason),
        };
        writeln!(out, "{}", renderer.text(&line))?;
    }
    Ok(())
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
