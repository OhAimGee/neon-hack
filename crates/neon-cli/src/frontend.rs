//! Which interface runs, and the question asked the first time.
//!
//! The full-screen interface starts only when it can be used well: standard input and
//! output are both terminals, `TERM` is not `dumb`, the terminal is at least as big as the
//! compact layout, and neither the player (`--plain`, a screen reader, `display = "plain"`
//! in `settings.toml`) nor the build (`--no-default-features`) rules it out. Everywhere
//! else the plain interface runs, which needs nothing. [`choose`] is that decision, as a
//! pure function of facts the caller collected once.
//!
//! The first time the game is started on a terminal, with no settings file, a question in
//! plain text asks how the player wants to play and writes the answer to `settings.toml`
//! ([`ask_mode`], [`save_choice`]); an existing file is never touched, damaged or not.

use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};
use std::path::Path;

use neon_engine::text::{Catalog, RenderMode, Text, render};

/// The interface to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Frontend {
    /// The full-screen interface.
    Tui,
    /// Line by line.
    Plain,
}

/// Why the player does not get the full-screen interface they could have expected, said on
/// `stderr` before the plain interface starts. Absent when nothing was expected: a pipe, a
/// screen reader and `--plain` need no excuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Notice {
    /// The terminal is smaller than the compact layout.
    TooSmall { width: u16, height: u16 },
    /// `--tui` was asked on something that is not a terminal.
    NoTerminal,
    /// `--tui` was asked on a terminal that does not move the cursor (`TERM=dumb`).
    Dumb,
    /// `--tui` was asked of a build without the full-screen interface.
    NoTui,
}

/// What decides the interface.
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent facts about the terminal, not a state machine"
)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct Facts {
    /// The wish: `--plain`, `--tui`, `settings.toml`, or nothing.
    pub(crate) display: crate::config::DisplayChoice,
    /// The wish was typed on the command line (and deserves an answer when it fails).
    pub(crate) asked_on_command_line: bool,
    /// Screen-reader mode, which is always the plain interface.
    pub(crate) screen_reader: bool,
    pub(crate) stdin_is_terminal: bool,
    pub(crate) stdout_is_terminal: bool,
    /// `TERM` is `dumb`.
    pub(crate) dumb: bool,
    /// Columns and rows of the terminal, if they can be known.
    pub(crate) size: Option<(u16, u16)>,
    /// The build has the full-screen interface.
    pub(crate) tui_built: bool,
}

/// What [`choose`] decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Decision {
    pub(crate) frontend: Frontend,
    pub(crate) notice: Option<Notice>,
}

/// Smallest terminal the full-screen interface takes, in columns and rows.
#[cfg(feature = "tui")]
pub(crate) const MIN_SIZE: (u16, u16) = crate::tui::COMPACT_SIZE;
#[cfg(not(feature = "tui"))]
pub(crate) const MIN_SIZE: (u16, u16) = (64, 20);

fn plain(notice: Option<Notice>) -> Decision {
    Decision {
        frontend: Frontend::Plain,
        notice,
    }
}

/// Decides the interface. Reasons that are routine (a pipe) are silent; the player is told
/// when they asked for the full screen on the command line, or when the only thing missing
/// is room.
pub(crate) fn choose(facts: &Facts) -> Decision {
    use crate::config::DisplayChoice;
    let said = |notice| facts.asked_on_command_line.then_some(notice);
    if facts.screen_reader || facts.display == DisplayChoice::Plain {
        return plain(None);
    }
    if !facts.tui_built {
        return plain(said(Notice::NoTui));
    }
    if !facts.stdin_is_terminal || !facts.stdout_is_terminal {
        return plain(said(Notice::NoTerminal));
    }
    if facts.dumb {
        return plain(said(Notice::Dumb));
    }
    match facts.size {
        None => plain(said(Notice::NoTerminal)),
        Some((width, height)) if width < MIN_SIZE.0 || height < MIN_SIZE.1 => {
            plain(Some(Notice::TooSmall { width, height }))
        }
        Some(_) => Decision {
            frontend: Frontend::Tui,
            notice: None,
        },
    }
}

/// The sentence for a notice.
pub(crate) fn notice_text(notice: Notice) -> Text {
    match notice {
        Notice::TooSmall { width, height } => Text::new("ui.fallback.too_small")
            .with_int("width", i64::from(width))
            .with_int("height", i64::from(height))
            .with_int("min_width", i64::from(MIN_SIZE.0))
            .with_int("min_height", i64::from(MIN_SIZE.1)),
        Notice::NoTerminal => Text::new("ui.fallback.no_terminal"),
        Notice::Dumb => Text::new("ui.fallback.dumb"),
        Notice::NoTui => Text::new("ui.fallback.no_tui"),
    }
}

// ---- The first-launch question --------------------------------------------------------

/// How the player chose to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModeAnswer {
    /// Full screen.
    Full,
    /// Line by line.
    Plain,
    /// Line by line with the wording and the settings of a screen reader.
    ScreenReader,
}

impl ModeAnswer {
    /// The lines `settings.toml` is created with.
    fn settings(self) -> &'static str {
        match self {
            Self::Full => "display = \"tui\"\n",
            Self::Plain => "display = \"plain\"\n",
            Self::ScreenReader => "display = \"plain\"\nscreen_reader = true\n",
        }
    }
}

/// Understands an answer: the number of the entry or a word for it, in either language.
/// An empty answer is the first entry.
pub(crate) fn parse_answer(line: &str) -> Option<ModeAnswer> {
    match line.trim().to_lowercase().as_str() {
        "" | "1" | "full" | "tui" | "plein" => Some(ModeAnswer::Full),
        "2" | "plain" | "ligne" => Some(ModeAnswer::Plain),
        "3" | "screen-reader" | "screen reader" | "lecteur" => Some(ModeAnswer::ScreenReader),
        _ => None,
    }
}

/// Whether the question is due: the first launch on a terminal, when nothing else already
/// says how to play and nothing forbids writing the answer.
#[allow(
    clippy::fn_params_excessive_bools,
    reason = "five independent facts, named at the call site and in the tests"
)]
pub(crate) fn question_due(
    settings_missing: bool,
    interactive: bool,
    asked_on_command_line: bool,
    screen_reader: bool,
    no_save: bool,
) -> bool {
    settings_missing && interactive && !asked_on_command_line && !screen_reader && !no_save
}

/// Asks how to play, in plain text, until the answer makes sense. `None` when the input
/// closes first: nothing is decided and the question comes back next time.
pub(crate) fn ask_mode(
    catalog: &Catalog,
    mode: RenderMode,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
) -> io::Result<Option<ModeAnswer>> {
    let say = |key: &'static str| render(&Text::new(key), catalog, mode);
    writeln!(out, "{}", say("ui.mode.title"))?;
    for key in ["ui.mode.full", "ui.mode.plain", "ui.mode.screen_reader"] {
        writeln!(out, "{}", say(key))?;
    }
    loop {
        write!(out, "{}", say("ui.mode.prompt"))?;
        out.flush()?;
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            writeln!(out)?;
            return Ok(None);
        }
        if let Some(answer) = parse_answer(&line) {
            return Ok(Some(answer));
        }
        writeln!(out, "{}", say("ui.mode.invalid"))?;
    }
}

/// Writes the answer in a new `settings.toml`. A file that exists is left alone, whatever it
/// holds (`create_new` refuses even when it appeared since we looked).
pub(crate) fn save_choice(path: &Path, answer: ModeAnswer) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(b"# Written by the first question of neon-hack; edit freely.\n")?;
    file.write_all(
        b"# `neon-hack --print-settings` shows every setting and where it comes from.\n",
    )?;
    file.write_all(answer.settings().as_bytes())?;
    file.flush()
}

#[cfg(test)]
mod tests;
