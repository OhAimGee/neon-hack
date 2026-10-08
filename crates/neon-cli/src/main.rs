//! `neon-hack`: the command-line entry point.
//!
//! The engine contract and saving are exercised by a small demo game (`--demo`) through
//! two frontends, plain and TUI. The real game arrives with phase R2 (see
//! `docs/ROADMAP.md`); without `--demo` the binary still says it is not playable yet.

mod config;
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

use clap::Parser;
use neon_engine::demo::DemoGame;
use neon_engine::event::Event;
use neon_engine::text::{Catalog, RenderMode, Text};
use neon_engine::{Game, Step};

use crate::config::{
    CliPresentation, Env, FileState, LangChoice, Presentation, Source, VerbosityChoice,
};
use crate::persist::Persistence;
use crate::render::Renderer;
use crate::store::{Store, Target};

/// Neon Hack: a cyberpunk text RPG for the terminal.
///
/// Presentation options can also be set in `settings.toml` in the data folder; the command
/// line wins over the file, which wins over the environment's defaults.
#[derive(Debug, Parser)]
#[command(name = "neon-hack", version)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent command-line flags, not a state machine"
)]
struct Cli {
    /// Language of the texts [default: `NEON_HACK_LANG`, `settings.toml`, the locale, then en].
    #[arg(long, value_enum, help_heading = "Presentation")]
    lang: Option<LangChoice>,

    /// How much atmosphere to show [default: normal].
    #[arg(long, value_enum, help_heading = "Presentation")]
    verbosity: Option<VerbosityChoice>,

    /// 7-bit ASCII output: ASCII symbols, no decoration, accents and typed text transliterated.
    #[arg(long, help_heading = "Presentation")]
    ascii: bool,

    /// Screen-reader mode: the plain interface with wording that needs no symbols read aloud.
    #[arg(long, help_heading = "Presentation")]
    screen_reader: bool,

    /// Use the plain line-by-line interface even on a terminal.
    #[arg(long, help_heading = "Presentation")]
    plain: bool,

    /// Start a new game instead of resuming the autosave (the previous autosave is kept
    /// as `auto.toml.bak`).
    #[arg(
        long,
        requires = "demo",
        conflicts_with = "load",
        help_heading = "Game"
    )]
    new: bool,

    /// Resume this save instead of the autosave: auto, checkpoint-1 to checkpoint-3, or
    /// slot-1 to slot-9.
    #[arg(
        long,
        requires = "demo",
        value_name = "SAVE",
        value_parser = Target::parse,
        help_heading = "Game"
    )]
    load: Option<Target>,

    /// List the saves and exit.
    #[arg(long, requires = "demo", help_heading = "Game")]
    list_saves: bool,

    /// Play the engine demo: a tiny game that exercises both frontends.
    #[arg(long, help_heading = "Development")]
    demo: bool,

    /// Seed of the random number generator (reproducible games).
    #[arg(long, help_heading = "Development")]
    seed: Option<u64>,

    /// Play without writing any save.
    #[arg(long, requires = "demo", help_heading = "Development")]
    no_save: bool,

    /// Data folder (saves, settings), instead of `NEON_HACK_DATA_DIR` or the system's.
    #[arg(long, value_name = "DIR", help_heading = "Development")]
    data_dir: Option<PathBuf>,

    /// Print the settings in effect, where each one comes from, and exit.
    #[arg(long, help_heading = "Development")]
    print_settings: bool,
}

impl Cli {
    fn presentation(&self) -> CliPresentation {
        CliPresentation {
            lang: self.lang,
            verbosity: self.verbosity,
            ascii: self.ascii,
            screen_reader: self.screen_reader,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    // The environment is read here, once, and handed down.
    let env = Env::from_process();
    let outcome = if cli.print_settings {
        print_settings(&cli, &env)
    } else if cli.demo {
        run_demo(&cli, &env)
    } else {
        banner::not_playable_yet();
        Ok(())
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            banner::fatal(&error);
            ExitCode::FAILURE
        }
    }
}

/// The settings that apply: the data folder, the settings file, and the presentation.
struct Setup {
    data_dir: PathBuf,
    data_dir_source: Source,
    settings_file: PathBuf,
    file_state: FileState,
    presentation: Presentation,
}

fn setup(cli: &Cli, env: &Env) -> io::Result<Setup> {
    let (data_dir, data_dir_source) = config::data_dir(cli.data_dir.as_deref(), env)?;
    let settings_file = data_dir.join("settings.toml");
    let (file, file_state) = config::load_file(&settings_file);
    let presentation = config::resolve(cli.presentation(), env, file);
    Ok(Setup {
        data_dir,
        data_dir_source,
        settings_file,
        file_state,
        presentation,
    })
}

fn print_settings(cli: &Cli, env: &Env) -> io::Result<()> {
    let setup = setup(cli, env)?;
    let shown = &setup.presentation;
    let line = |name: &str, value: String, source: Source| {
        format!("{name} = {value}  # {}", source.describe())
    };
    let file_state = match &setup.file_state {
        FileState::Missing => "missing".to_owned(),
        FileState::Loaded => "loaded".to_owned(),
        FileState::Damaged(reason) => format!("damaged: {reason}"),
    };
    let lines = [
        line(
            "lang",
            format!("{:?}", shown.lang.value.name()),
            shown.lang.source,
        ),
        line(
            "verbosity",
            format!("{:?}", shown.verbosity.value.name()),
            shown.verbosity.source,
        ),
        line("ascii", shown.ascii.value.to_string(), shown.ascii.source),
        line(
            "screen_reader",
            shown.screen_reader.value.to_string(),
            shown.screen_reader.source,
        ),
        line(
            "data_dir",
            format!("{:?}", setup.data_dir.display().to_string()),
            setup.data_dir_source,
        ),
        format!(
            "settings_file = {:?}  # {file_state}",
            setup.settings_file.display().to_string()
        ),
    ];
    banner::print(&lines.join("\n"));
    Ok(())
}

fn run_demo(cli: &Cli, env: &Env) -> io::Result<()> {
    let setup = setup(cli, env)?;
    let shown = setup.presentation;
    // The catalogs are checked by the tests, so this only fails on a broken build.
    let catalog = Catalog::embedded(shown.lang.value.into())
        .map_err(|errors| io::Error::other(format!("the texts do not load:\n{errors}")))?;
    // The two are independent and can be combined.
    let renderer = Renderer {
        catalog: &catalog,
        mode: RenderMode {
            screen_reader: shown.screen_reader.value,
            ascii: shown.ascii.value,
        },
        verbosity: shown.verbosity.value.into(),
    };
    let store = Store::new(setup.data_dir.join("saves"));
    if cli.list_saves {
        return list_saves(&store, &renderer, &mut io::stdout().lock());
    }
    // The owner of the game starts or resumes it once and hands the step to whichever
    // frontend runs.
    let (mut game, mut first) = open_game(cli, &store, &renderer)?;
    if let FileState::Damaged(reason) = &setup.file_state {
        let notice = Text::new("ui.settings.damaged").with_str("reason", reason.clone());
        first.events.insert(0, Event::system(notice));
    }
    let persistence = if cli.no_save {
        Persistence::disabled()
    } else {
        Persistence::new(store)
    };

    let stdin = io::stdin();
    let interactive = stdin.is_terminal() && io::stdout().is_terminal();
    // The TUI needs a real terminal; a screen reader needs the plain interface.
    let use_tui = cfg!(feature = "tui") && interactive && !cli.plain && !shown.screen_reader.value;
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

    pub(super) fn print(text: &str) {
        println!("{text}");
    }

    pub(super) fn fatal(error: &std::io::Error) {
        eprintln!("neon-hack: {error}");
    }
}
