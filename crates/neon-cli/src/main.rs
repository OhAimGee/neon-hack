//! `neon-hack`: the command-line entry point.
//!
//! The binary plays the campaign, the real game, through two frontends, plain and TUI; a
//! campaign is started or resumed by running it without any game option. The engine contract
//! and saving are also exercised by a small toy game, the demo (`--demo`), which keeps its own
//! saves and is never mistaken for the campaign.

mod config;
mod frontend;
mod input;
mod palette;
mod persist;
mod plain;
mod render;
mod store;
#[cfg(test)]
mod test_support;
#[cfg(feature = "tui")]
mod tui;

use std::io::{self, BufReader, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, ValueEnum};
use neon_engine::campaign::{CampaignGame, Difficulty};
use neon_engine::demo::DemoGame;
use neon_engine::event::Event;
use neon_engine::text::{Catalog, RenderMode, Text, to_ascii};
use neon_engine::{Game, Step};

use crate::config::{
    CliPresentation, DisplayChoice, Env, FileState, LangChoice, Presentation, Source,
    VerbosityChoice,
};
use crate::frontend::{Facts, Frontend, ModeAnswer};
use crate::palette::{ColorChoice, Palette, PaletteChoice};
use crate::persist::Persistence;
use crate::render::Renderer;
use crate::store::{Store, Target};

/// Neon Hack: a cyberpunk text RPG for the terminal.
///
/// Without a game option it plays the campaign: a new one, or the one saved last. `--demo`
/// plays the small demo game instead, with saves of its own.
///
/// Presentation options can also be set in `settings.toml` in the data folder; the command
/// line wins over the file, which wins over the environment's defaults.
#[derive(Debug, Parser)]
#[command(name = "neon-hack", version)]
// The two games exclude each other; the campaign is the one played when none is named.
#[command(group(clap::ArgGroup::new("game").args(["demo", "campaign"]).multiple(false)))]
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

    /// Colour: `auto` (a terminal that can show it), `always`, or `never`. `NO_COLOR` turns it
    /// off unless this says otherwise; a screen reader never gets any.
    #[arg(long, value_enum, value_name = "WHEN", help_heading = "Presentation")]
    color: Option<ColorChoice>,

    /// Same as `--color never`.
    #[arg(long, conflicts_with = "color", help_heading = "Presentation")]
    no_color: bool,

    /// Colours of the full-screen interface [default: `NEON_HACK_PALETTE`, `settings.toml`,
    /// then default]. `mono` is used by itself when there is no colour.
    #[arg(long, value_enum, help_heading = "Presentation")]
    palette: Option<PaletteChoice>,

    /// Use the plain line-by-line interface even on a terminal.
    #[arg(long, help_heading = "Presentation")]
    plain: bool,

    /// Use the full-screen interface, and say why when the terminal cannot show it. This is
    /// what runs by itself on a terminal that is big enough (at least 64x20).
    #[arg(long, conflicts_with_all = ["plain", "screen_reader"], help_heading = "Presentation")]
    tui: bool,

    /// Start a new game instead of resuming the autosave (the previous autosave is kept
    /// as `auto.toml.bak`).
    #[arg(long, conflicts_with = "load", help_heading = "Game")]
    new: bool,

    /// Resume this save instead of the autosave: auto, checkpoint-1 to checkpoint-3, or
    /// slot-1 to slot-9.
    #[arg(
        long,
        value_name = "SAVE",
        value_parser = Target::parse,
        help_heading = "Game"
    )]
    load: Option<Target>,

    /// List the saves and exit.
    #[arg(long, help_heading = "Game")]
    list_saves: bool,

    /// Difficulty of a new campaign [default: normal]. A campaign that is resumed keeps its own.
    #[arg(long, value_enum, conflicts_with = "demo", help_heading = "Game")]
    difficulty: Option<DifficultyChoice>,

    /// Play the campaign. This is what runs when no game is named: the option is kept for
    /// scripts that asked for it before it became the default.
    #[arg(long, hide = true)]
    campaign: bool,

    /// Play the engine demo instead of the campaign: a tiny toy game that exercises both
    /// frontends. It has its own saves.
    #[arg(long, help_heading = "Development")]
    demo: bool,

    /// Seed of the random number generator (reproducible games).
    #[arg(long, help_heading = "Development")]
    seed: Option<u64>,

    /// Play without writing any save.
    #[arg(long, help_heading = "Development")]
    no_save: bool,

    /// Data folder (saves, settings), instead of `NEON_HACK_DATA_DIR` or the system's.
    #[arg(long, value_name = "DIR", help_heading = "Development")]
    data_dir: Option<PathBuf>,

    /// Print the settings in effect, where each one comes from, and exit.
    #[arg(long, help_heading = "Development")]
    print_settings: bool,
}

/// A difficulty as written on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum DifficultyChoice {
    Story,
    Normal,
    Expert,
    Hardcore,
}

impl From<DifficultyChoice> for Difficulty {
    fn from(choice: DifficultyChoice) -> Self {
        match choice {
            DifficultyChoice::Story => Self::Story,
            DifficultyChoice::Normal => Self::Normal,
            DifficultyChoice::Expert => Self::Expert,
            DifficultyChoice::Hardcore => Self::Hardcore,
        }
    }
}

/// The game being played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Demo,
    Campaign,
}

impl Kind {
    /// The folder of the saves inside the data folder: one per game, so a demo save and a
    /// campaign save can never be mistaken for each other.
    fn saves_folder(self) -> &'static str {
        match self {
            Self::Demo => "saves",
            Self::Campaign => "saves-campaign",
        }
    }

    /// The other game.
    fn other(self) -> Self {
        match self {
            Self::Demo => Self::Campaign,
            Self::Campaign => Self::Demo,
        }
    }

    /// How the game is named in a sentence, and the command that plays it.
    fn name(self) -> (&'static str, &'static str) {
        match self {
            Self::Demo => ("ui.game.demo", "neon-hack --demo"),
            Self::Campaign => ("ui.game.campaign", "neon-hack"),
        }
    }
}

impl Cli {
    /// The presentation asked for, with the answer to the first question if there was one
    /// (it holds for this session even when it could not be written down).
    fn presentation(&self, answer: Option<ModeAnswer>) -> CliPresentation {
        let display = if self.plain {
            Some(DisplayChoice::Plain)
        } else if self.tui {
            Some(DisplayChoice::Tui)
        } else {
            match answer {
                Some(ModeAnswer::Full) => Some(DisplayChoice::Tui),
                Some(ModeAnswer::Plain | ModeAnswer::ScreenReader) => Some(DisplayChoice::Plain),
                None => None,
            }
        };
        CliPresentation {
            display,
            lang: self.lang,
            verbosity: self.verbosity,
            ascii: self.ascii,
            screen_reader: self.screen_reader || answer == Some(ModeAnswer::ScreenReader),
            color: self.no_color.then_some(ColorChoice::Never).or(self.color),
            palette: self.palette,
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
        run_game(&cli, &env, Kind::Demo)
    } else {
        run_game(&cli, &env, Kind::Campaign)
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

fn setup(cli: &Cli, env: &Env, answer: Option<ModeAnswer>) -> io::Result<Setup> {
    let (data_dir, data_dir_source) = config::data_dir(cli.data_dir.as_deref(), env)?;
    let settings_file = data_dir.join("settings.toml");
    let (file, file_state) = config::load_file(&settings_file);
    let presentation = config::resolve(cli.presentation(answer), env, file);
    Ok(Setup {
        data_dir,
        data_dir_source,
        settings_file,
        file_state,
        presentation,
    })
}

fn print_settings(cli: &Cli, env: &Env) -> io::Result<()> {
    let setup = setup(cli, env, None)?;
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
        format!("color = {}  # {}", shown.color.0, shown.color.1.describe()),
        line(
            "palette",
            format!("{:?}", shown.palette.value.name()),
            shown.palette.source,
        ),
        line(
            "screen_reader",
            shown.screen_reader.value.to_string(),
            shown.screen_reader.source,
        ),
        line(
            "display",
            format!("{:?}", shown.display.value.name()),
            shown.display.source,
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
    // Like everything printed in ASCII mode, a path or a reason with accents is transliterated.
    let text = lines.join("\n");
    banner::print(&if shown.ascii.value {
        to_ascii(&text)
    } else {
        text
    });
    Ok(())
}

fn run_game(cli: &Cli, env: &Env, kind: Kind) -> io::Result<()> {
    let mut setup = setup(cli, env, None)?;
    // The first time, on a terminal, the player says how they want to play.
    if !cli.list_saves {
        let asked = cli.plain || cli.tui || cli.screen_reader;
        let due = frontend::question_due(
            setup.file_state == FileState::Missing,
            env.stdin_is_terminal && env.stdout_is_terminal,
            asked,
            setup.presentation.screen_reader.value,
            cli.no_save,
        );
        if due && let Some(answer) = ask_first_mode(&setup)? {
            let saved = match frontend::save_choice(&setup.settings_file, answer) {
                Ok(()) => Text::new("ui.mode.saved")
                    .with_str("file", setup.settings_file.display().to_string()),
                Err(error) => Text::new("ui.mode.not_saved").with_str("reason", error.to_string()),
            };
            banner::print(&render_ui(&setup, &saved)?);
            setup = self::setup(cli, env, Some(answer))?;
        }
    }
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
        colors: shown
            .color
            .0
            .then(|| Palette::new(shown.palette.value, shown.truecolor)),
    };
    let store = Store::new(setup.data_dir.join(kind.saves_folder()));
    if cli.list_saves {
        return list_saves(&store, &renderer, &mut io::stdout().lock());
    }
    // The owner of the game starts or resumes it once and hands the step to whichever
    // frontend runs.
    let (mut game, mut first) = open_game(cli, kind, &store, &renderer)?;
    if let FileState::Damaged(reason) = &setup.file_state {
        let notice = Text::new("ui.settings.damaged").with_str("reason", reason.clone());
        first.events.insert(0, Event::system(notice));
    }
    let persistence = if cli.no_save {
        Persistence::disabled()
    } else {
        Persistence::new(store)
    };

    // The full-screen interface needs a real terminal of a decent size; a screen reader
    // needs the plain interface; nothing else is asked of the plain one.
    let both_terminals = env.stdin_is_terminal && env.stdout_is_terminal;
    let decision = frontend::choose(&Facts {
        display: shown.display.value,
        asked_on_command_line: shown.display.source == Source::Cli,
        screen_reader: shown.screen_reader.value,
        stdin_is_terminal: env.stdin_is_terminal,
        stdout_is_terminal: env.stdout_is_terminal,
        dumb: env.terminal().dumb,
        size: if both_terminals {
            terminal_size()
        } else {
            None
        },
        tui_built: cfg!(feature = "tui"),
    });
    if let Some(notice) = decision.notice {
        banner::notice(&renderer.text(&frontend::notice_text(notice)));
    }
    if decision.frontend == Frontend::Tui {
        #[cfg(feature = "tui")]
        return tui::run(&mut *game, first, renderer, persistence, env.test_fault.as_deref());
    }
    // On a pipe the lines read are repeated in the output so that it reads as a transcript;
    // on a terminal the terminal echoes by itself.
    let stdin = io::stdin();
    let echo_input = !env.stdin_is_terminal;
    let mut persistence = persistence;
    plain::run(
        &mut *game,
        first,
        &renderer,
        &mut BufReader::new(stdin.lock()),
        &mut io::stdout().lock(),
        echo_input,
        &mut persistence,
    )
}

/// The size of the terminal, when there is a way to ask.
fn terminal_size() -> Option<(u16, u16)> {
    #[cfg(feature = "tui")]
    return tui::terminal_size();
    #[cfg(not(feature = "tui"))]
    None
}

/// A text of the interface in the language and mode of these settings, for what is said
/// before the game exists.
fn render_ui(setup: &Setup, text: &Text) -> io::Result<String> {
    let shown = &setup.presentation;
    let catalog = Catalog::embedded(shown.lang.value.into())
        .map_err(|errors| io::Error::other(format!("the texts do not load:\n{errors}")))?;
    Ok(neon_engine::text::render(
        text,
        &catalog,
        RenderMode {
            screen_reader: shown.screen_reader.value,
            ascii: shown.ascii.value,
        },
    ))
}

/// Asks how to play on the terminal, in the language the settings say.
fn ask_first_mode(setup: &Setup) -> io::Result<Option<ModeAnswer>> {
    let shown = &setup.presentation;
    let catalog = Catalog::embedded(shown.lang.value.into())
        .map_err(|errors| io::Error::other(format!("the texts do not load:\n{errors}")))?;
    let mode = RenderMode {
        screen_reader: false,
        ascii: shown.ascii.value,
    };
    frontend::ask_mode(
        &catalog,
        mode,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
    )
}

/// Loads a save text as a game of the given kind.
fn load_game(kind: Kind, text: &str) -> Result<Box<dyn Game>, neon_engine::save::SaveError> {
    match kind {
        Kind::Demo => DemoGame::from_save(text).map(|game| Box::new(game) as Box<dyn Game>),
        Kind::Campaign => CampaignGame::from_save(text).map(|game| Box::new(game) as Box<dyn Game>),
    }
}

/// The message for a save that cannot be read as `kind`, when it is a save of the other game:
/// it is not damaged, it is somewhere else, and the player is told so. Only the latest file
/// counts: a valid backup of this game must not hide that the save itself is another game's.
fn foreign_save(kind: Kind, store: &Store, target: Target) -> Option<Text> {
    let other = kind.other();
    let text = store.primary_text(target)?;
    if load_game(kind, &text).is_ok() || load_game(other, &text).is_err() {
        return None;
    }
    let (other_name, other_command) = other.name();
    Some(
        Text::new("ui.save.other_game")
            .with_text("other", Text::new(other_name))
            .with_text("this", Text::new(kind.name().0))
            .with_str("command", other_command),
    )
}

/// A game to play: the save asked for (the autosave by default), or a new one.
fn open_game(
    cli: &Cli,
    kind: Kind,
    store: &Store,
    renderer: &Renderer<'_>,
) -> io::Result<(Box<dyn Game>, Step)> {
    if !cli.new {
        let target = cli.load.unwrap_or(Target::Auto);
        if let Some(message) = foreign_save(kind, store, target) {
            return Err(io::Error::other(renderer.text(&message)));
        }
        match store.read(target, |text| load_game(kind, text)) {
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
                let message =
                    Text::new("ui.save.unreadable").with_str("details", error.to_string());
                return Err(io::Error::other(renderer.text(&message)));
            }
        }
    }
    let seed = cli.seed.unwrap_or_else(clock_seed);
    let mut game: Box<dyn Game> = match kind {
        Kind::Demo => Box::new(DemoGame::new(seed)),
        Kind::Campaign => {
            let difficulty = cli.difficulty.map_or(Difficulty::Normal, Difficulty::from);
            let game = CampaignGame::new(difficulty, seed)
                .map_err(|error| io::Error::other(error.to_string()))?;
            Box::new(game)
        }
    };
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

    pub(super) fn print(text: &str) {
        println!("{text}");
    }

    /// A word to the player that is not part of the game (why an interface was not used).
    pub(super) fn notice(text: &str) {
        eprintln!("neon-hack: {text}");
    }

    pub(super) fn fatal(error: &std::io::Error) {
        eprintln!("neon-hack: {error}");
    }
}
