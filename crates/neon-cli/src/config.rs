//! How the player's choices reach the program: command line, environment, settings file.
//!
//! Options come in three families (`DECISIONS.md` R-2, CROSS-CHECK 18): **presentation**
//! (language, verbosity, ASCII, screen reader: what the player sees, kept in
//! `settings.toml`), **game** (what to play: a new game, a save) and **development**
//! (seed, data folder, no saving). This module resolves the first family, one setting at a
//! time, from the most specific source to the least:
//!
//! | Setting | Order |
//! |---|---|
//! | language | `--lang`, `NEON_HACK_LANG`, `settings.toml`, the locale (`LC_ALL`, `LC_MESSAGES`, `LANG`), English |
//! | verbosity | `--verbosity`, `settings.toml`, normal |
//! | ASCII | `--ascii`, `settings.toml`, a locale that names a non-UTF-8 charset, off |
//! | screen reader | `--screen-reader`, `settings.toml`, off |
//!
//! The environment is read **once** ([`Env::from_process`]) and passed by value, so the rules
//! are plain functions that tests call with a hand-made [`Env`]; nothing here changes the
//! process environment.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use directories::ProjectDirs;
use neon_engine::text::Lang;
use serde::Deserialize;

use crate::render::Verbosity;

/// Largest settings file read. Settings are a few lines.
const MAX_SETTINGS_BYTES: u64 = 64 * 1024;

/// What the program reads from the process environment, captured once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Env {
    pub(crate) lc_all: Option<String>,
    pub(crate) lc_messages: Option<String>,
    pub(crate) lc_ctype: Option<String>,
    pub(crate) lang: Option<String>,
    /// `NEON_HACK_LANG`
    pub(crate) neon_lang: Option<String>,
    /// `NEON_HACK_DATA_DIR`
    pub(crate) neon_data_dir: Option<PathBuf>,
    /// The data folder of the platform for this user (`$XDG_DATA_HOME` or `~/.local/share`,
    /// `~/Library/Application Support`, `%APPDATA%`), looked up with the variables so that
    /// nothing else touches the environment.
    pub(crate) system_data_dir: Option<PathBuf>,
}

impl Env {
    /// The only place that reads the environment. Empty variables count as unset.
    pub(crate) fn from_process() -> Self {
        let text = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
        Self {
            lc_all: text("LC_ALL"),
            lc_messages: text("LC_MESSAGES"),
            lc_ctype: text("LC_CTYPE"),
            lang: text("LANG"),
            neon_lang: text("NEON_HACK_LANG"),
            neon_data_dir: std::env::var_os("NEON_HACK_DATA_DIR")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            system_data_dir: ProjectDirs::from("", "", "neon-hack")
                .map(|dirs| dirs.data_dir().to_path_buf()),
        }
    }

    /// The locale that decides the language, by POSIX precedence.
    fn message_locale(&self) -> Option<&str> {
        [&self.lc_all, &self.lc_messages, &self.lang]
            .into_iter()
            .flatten()
            .map(String::as_str)
            .next()
    }

    /// The locale that decides the character set, by POSIX precedence.
    fn charset_locale(&self) -> Option<&str> {
        [&self.lc_all, &self.lc_ctype, &self.lang]
            .into_iter()
            .flatten()
            .map(String::as_str)
            .next()
    }
}

/// A language as written on the command line and in `settings.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LangChoice {
    En,
    Fr,
}

impl From<LangChoice> for Lang {
    fn from(value: LangChoice) -> Self {
        match value {
            LangChoice::En => Self::En,
            LangChoice::Fr => Self::Fr,
        }
    }
}

impl LangChoice {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fr => "fr",
        }
    }

    fn from_code(code: &str) -> Option<Self> {
        match code.to_ascii_lowercase().as_str() {
            "en" => Some(Self::En),
            "fr" => Some(Self::Fr),
            _ => None,
        }
    }
}

/// A verbosity as written on the command line and in `settings.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum VerbosityChoice {
    Brief,
    Normal,
    Full,
}

impl VerbosityChoice {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Brief => "brief",
            Self::Normal => "normal",
            Self::Full => "full",
        }
    }
}

impl From<VerbosityChoice> for Verbosity {
    fn from(value: VerbosityChoice) -> Self {
        match value {
            VerbosityChoice::Brief => Self::Brief,
            VerbosityChoice::Normal => Self::Normal,
            VerbosityChoice::Full => Self::Full,
        }
    }
}

/// The presentation options given on the command line.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CliPresentation {
    pub(crate) lang: Option<LangChoice>,
    pub(crate) verbosity: Option<VerbosityChoice>,
    pub(crate) ascii: bool,
    pub(crate) screen_reader: bool,
}

/// The content of `settings.toml`. A key that is absent leaves the next source in charge;
/// an unknown key is ignored, so a newer file still opens in an older game.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub(crate) struct FileSettings {
    lang: Option<LangChoice>,
    verbosity: Option<VerbosityChoice>,
    ascii: Option<bool>,
    screen_reader: Option<bool>,
}

/// Where a resolved value came from, for `--print-settings` and for tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    Cli,
    EnvLang,
    EnvDataDir,
    File,
    Locale,
    Default,
}

impl Source {
    pub(crate) fn describe(self) -> &'static str {
        match self {
            Self::Cli => "command line",
            Self::EnvLang => "NEON_HACK_LANG",
            Self::EnvDataDir => "NEON_HACK_DATA_DIR",
            Self::File => "settings.toml",
            Self::Locale => "locale",
            Self::Default => "default",
        }
    }
}

/// A value and the source that decided it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sourced<T> {
    pub(crate) value: T,
    pub(crate) source: Source,
}

fn sourced<T>(value: T, source: Source) -> Sourced<T> {
    Sourced { value, source }
}

/// The presentation the player gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Presentation {
    pub(crate) lang: Sourced<LangChoice>,
    pub(crate) verbosity: Sourced<VerbosityChoice>,
    pub(crate) ascii: Sourced<bool>,
    pub(crate) screen_reader: Sourced<bool>,
}

/// Applies the table of the module documentation.
pub(crate) fn resolve(cli: CliPresentation, env: &Env, file: FileSettings) -> Presentation {
    let lang = if let Some(choice) = cli.lang {
        sourced(choice, Source::Cli)
    } else if let Some(choice) = env.neon_lang.as_deref().and_then(LangChoice::from_code) {
        // A value the game has no texts for falls through to the next source.
        sourced(choice, Source::EnvLang)
    } else if let Some(choice) = file.lang {
        sourced(choice, Source::File)
    } else if let Some(choice) = env.message_locale().and_then(language_of_locale) {
        sourced(choice, Source::Locale)
    } else {
        sourced(LangChoice::En, Source::Default)
    };
    let verbosity = if let Some(choice) = cli.verbosity {
        sourced(choice, Source::Cli)
    } else if let Some(choice) = file.verbosity {
        sourced(choice, Source::File)
    } else {
        sourced(VerbosityChoice::Normal, Source::Default)
    };
    let ascii = if cli.ascii {
        sourced(true, Source::Cli)
    } else if let Some(choice) = file.ascii {
        sourced(choice, Source::File)
    } else if env.charset_locale().is_some_and(names_a_non_utf8_charset) {
        sourced(true, Source::Locale)
    } else {
        sourced(false, Source::Default)
    };
    let screen_reader = if cli.screen_reader {
        sourced(true, Source::Cli)
    } else if let Some(choice) = file.screen_reader {
        sourced(choice, Source::File)
    } else {
        sourced(false, Source::Default)
    };
    Presentation {
        lang,
        verbosity,
        ascii,
        screen_reader,
    }
}

/// `fr_FR.UTF-8` is French, `en_GB` is English, `C` says nothing.
fn language_of_locale(locale: &str) -> Option<LangChoice> {
    let code = locale.split(['_', '.', '@']).next()?;
    LangChoice::from_code(code)
}

/// True for `fr_FR.ISO-8859-1`, false for `fr_FR.UTF-8`, `fr_FR`, `C` and `POSIX`: a locale
/// without a charset says nothing, and `C` is what containers set while their terminal does
/// UTF-8 anyway, so only an explicit other charset turns ASCII on by itself.
fn names_a_non_utf8_charset(locale: &str) -> bool {
    let Some((_, rest)) = locale.split_once('.') else {
        return false;
    };
    let charset = rest.split('@').next().unwrap_or_default();
    let normalized: String = charset
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase();
    !charset.is_empty() && normalized != "utf8"
}

/// What reading the settings file found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileState {
    Missing,
    Loaded,
    /// The file exists but cannot be used: the settings are the defaults and the player is
    /// told. The file is never rewritten.
    Damaged(String),
}

/// Reads `settings.toml`; a missing or damaged file gives the defaults.
pub(crate) fn load_file(path: &Path) -> (FileSettings, FileState) {
    let read = || -> Result<Option<FileSettings>, String> {
        let size = match fs::metadata(path) {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        if size > MAX_SETTINGS_BYTES {
            return Err("the file is too large for settings".to_owned());
        }
        let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
        parse_file(&text).map(Some)
    };
    match read() {
        Ok(Some(settings)) => (settings, FileState::Loaded),
        Ok(None) => (FileSettings::default(), FileState::Missing),
        Err(reason) => (FileSettings::default(), FileState::Damaged(reason)),
    }
}

fn parse_file(text: &str) -> Result<FileSettings, String> {
    toml::from_str(text).map_err(|error| error.to_string().trim_end().to_owned())
}

/// The data folder: `--data-dir`, then `NEON_HACK_DATA_DIR`, then the system's.
pub(crate) fn data_dir(cli: Option<&Path>, env: &Env) -> io::Result<(PathBuf, Source)> {
    if let Some(dir) = cli {
        return Ok((dir.to_path_buf(), Source::Cli));
    }
    if let Some(dir) = &env.neon_data_dir {
        return Ok((dir.clone(), Source::EnvDataDir));
    }
    env.system_data_dir
        .clone()
        .map(|dir| (dir, Source::Default))
        .ok_or_else(|| io::Error::other("no data folder found for this user: use --data-dir"))
}

#[cfg(test)]
mod tests;
