//! The `options` and `verbosity` commands, run by the frontends.
//!
//! They act on the display, not on the game, so a frontend takes the line before the engine
//! sees it (the engine only declares them: they are listed, completed and explained like any
//! other command, see `docs/spec/commands.md` section 2.1 bis). Both frontends share this
//! module: [`Host::run`] reads the line and answers with events the frontend renders like any
//! others, and [`Host::apply`] hands the new settings to its renderer.
//!
//! What changes lasts for the session. `options save` keeps it: it writes the settings changed
//! during the session into `settings.toml` and leaves every other line, comment and key alone,
//! and it never touches a file that does not read (a damaged file is the player's to repair).

use std::fs;
use std::io;
use std::path::PathBuf;

use neon_engine::event::{Event, Table};
use neon_engine::text::{Catalog, Text};

use crate::config::{LangChoice, VerbosityChoice};
use crate::palette::{Palette, PaletteChoice};
use crate::render::Renderer;
use crate::store::write_atomic;

/// The settings the player can change in the middle of a game, in the order of the screen.
const SETTINGS: [&str; 5] = ["lang", "verbosity", "ascii", "palette", "reduce_motion"];
const LANGS: [&str; 2] = ["en", "fr"];
const VERBOSITIES: [&str; 3] = ["brief", "normal", "full"];
const SWITCH: [&str; 2] = ["on", "off"];
const PALETTES: [&str; 4] = ["default", "high-contrast", "cvd", "mono"];
/// What the first line of a new `settings.toml` says.
const NEW_FILE_HEADER: &str = "# Written by the options command of neon-hack; edit freely.\n\
# `neon-hack --print-settings` shows every setting and where it comes from.\n";

/// The text catalogs of both languages, so that the language can change in the middle of a game.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Catalogs<'a> {
    pub(crate) en: &'a Catalog,
    pub(crate) fr: &'a Catalog,
}

impl<'a> Catalogs<'a> {
    pub(crate) fn get(&self, lang: LangChoice) -> &'a Catalog {
        match lang {
            LangChoice::En => self.en,
            LangChoice::Fr => self.fr,
        }
    }
}

/// What can change during the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Live {
    pub(crate) lang: LangChoice,
    pub(crate) verbosity: VerbosityChoice,
    pub(crate) ascii: bool,
    pub(crate) palette: PaletteChoice,
    pub(crate) reduce_motion: bool,
}

/// What was decided at launch and stays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fixed {
    /// Screen-reader mode: the plain interface, never below `normal` verbosity, no colour.
    pub(crate) screen_reader: bool,
    /// There is colour at all (the palette only matters then).
    pub(crate) color: bool,
    /// The terminal does 24-bit colour.
    pub(crate) truecolor: bool,
}

/// What `options save` may do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    /// The game was started with `--no-save`: nothing is written.
    Off,
    /// The settings file, which reads (or does not exist yet).
    File(PathBuf),
    /// The settings file exists and does not read: it is left alone.
    Damaged(PathBuf),
}

/// The settings of a session, and what is needed to show and keep them.
pub(crate) struct Host<'a> {
    pub(crate) catalogs: Catalogs<'a>,
    pub(crate) live: Live,
    pub(crate) fixed: Fixed,
    pub(crate) target: Target,
    /// The settings the player changed in this session, in the order they were first changed.
    /// Only those are kept by `options save`: a value that comes from the command line or from
    /// `NO_COLOR` is not frozen into the file by accident.
    pub(crate) changed: Vec<&'static str>,
}

/// A setting and a value, read from a line.
enum Request {
    Show,
    Save,
    Set { setting: String, value: String },
    Verbosity(Option<String>),
    Usage(&'static str),
}

impl<'a> Host<'a> {
    /// Runs the line if it is one of the presentation commands (`options`, `verbosity`),
    /// whatever its case; `None` means it is not, and belongs to the engine.
    pub(crate) fn run(&mut self, line: &str) -> Option<Vec<Event>> {
        let request = request(line)?;
        Some(match request {
            Request::Show => self.show(),
            Request::Save => self.save(),
            Request::Set { setting, value } => self.set(&setting, &value),
            Request::Verbosity(None) => vec![Event::system(
                Text::new("ui.options.verbosity_now").with_str("value", self.live.verbosity.name()),
            )],
            Request::Verbosity(Some(value)) => self.set("verbosity", &value),
            Request::Usage(key) => vec![Event::error(Text::new(key))],
        })
    }

    /// Gives the renderer the settings of the moment.
    pub(crate) fn apply(&self, renderer: &mut Renderer<'a>) {
        renderer.catalog = self.catalogs.get(self.live.lang);
        renderer.mode.ascii = self.live.ascii;
        renderer.verbosity = self.live.verbosity.into();
        renderer.colors = self
            .fixed
            .color
            .then(|| Palette::new(self.live.palette, self.fixed.truecolor));
    }

    fn value_of(&self, setting: &str) -> String {
        match setting {
            "lang" => self.live.lang.name().to_owned(),
            "verbosity" => self.live.verbosity.name().to_owned(),
            "ascii" => switch(self.live.ascii),
            "palette" => self.live.palette.name().to_owned(),
            "reduce_motion" => switch(self.live.reduce_motion),
            _ => String::new(),
        }
    }

    fn show(&self) -> Vec<Event> {
        let mut rows: Vec<Vec<Text>> = SETTINGS
            .iter()
            .map(|setting| {
                vec![
                    Text::raw(*setting),
                    Text::raw(self.value_of(setting)),
                    Text::raw(choices(setting).join(", ")),
                ]
            })
            .collect();
        rows.push(vec![
            Text::raw("screen_reader"),
            Text::raw(switch(self.fixed.screen_reader)),
            Text::new("ui.options.at_launch"),
        ]);
        let hint = match &self.target {
            Target::Off => Text::new("ui.options.hint_off"),
            Target::File(path) | Target::Damaged(path) => {
                Text::new("ui.options.hint").with_str("file", path.display().to_string())
            }
        };
        vec![
            Event::Screen(Table::new(
                Text::new("ui.options.title"),
                vec![
                    Text::new("ui.options.col_setting"),
                    Text::new("ui.options.col_value"),
                    Text::new("ui.options.col_choices"),
                ],
                rows,
            )),
            Event::system(hint),
            Event::system(Text::new("ui.options.usage")),
        ]
    }

    fn set(&mut self, setting: &str, value: &str) -> Vec<Event> {
        let setting = setting.to_ascii_lowercase();
        let value = value.to_ascii_lowercase();
        if setting == "screen_reader" {
            return vec![Event::error(
                Text::new("ui.options.fixed").with_str("setting", setting),
            )];
        }
        if !SETTINGS.contains(&setting.as_str()) {
            return vec![Event::error(
                Text::new("ui.options.unknown_setting").with_str("setting", setting),
            )];
        }
        let Some(live) = self.with(&setting, &value) else {
            return vec![Event::error(
                Text::new("ui.options.bad_value")
                    .with_str("setting", setting.clone())
                    .with_str("value", value)
                    .with_str("choices", choices(&setting).join(", ")),
            )];
        };
        // A screen reader is never given less than the normal amount of text (VRB-3).
        if self.fixed.screen_reader && live.verbosity == VerbosityChoice::Brief {
            return vec![Event::error(Text::new("ui.options.floor"))];
        }
        self.live = live;
        if let Some(name) = SETTINGS.iter().find(|name| **name == setting)
            && !self.changed.contains(name)
        {
            self.changed.push(name);
        }
        let mut events = vec![Event::system(
            Text::new("ui.options.changed")
                .with_str("setting", setting.clone())
                .with_str("value", self.value_of(&setting)),
        )];
        if setting == "palette" && !self.fixed.color {
            events.push(Event::system(Text::new("ui.options.no_colour")));
        }
        events
    }

    /// The settings with one changed, or `None` if the value is not one of its choices.
    fn with(&self, setting: &str, value: &str) -> Option<Live> {
        let mut live = self.live;
        match setting {
            "lang" => live.lang = LangChoice::from_code(value)?,
            "verbosity" => live.verbosity = verbosity_of(value)?,
            "ascii" => live.ascii = switch_of(value)?,
            "palette" => live.palette = PaletteChoice::from_name(value)?,
            "reduce_motion" => live.reduce_motion = switch_of(value)?,
            _ => return None,
        }
        Some(live)
    }

    fn save(&self) -> Vec<Event> {
        let path = match &self.target {
            Target::Off => return vec![Event::error(Text::new("ui.options.save_off"))],
            Target::Damaged(path) => {
                return vec![Event::error(
                    Text::new("ui.options.save_damaged")
                        .with_str("file", path.display().to_string()),
                )];
            }
            Target::File(path) => path,
        };
        if self.changed.is_empty() {
            return vec![Event::system(Text::new("ui.options.save_nothing"))];
        }
        let updates: Vec<(&str, String)> = self
            .changed
            .iter()
            .map(|setting| {
                let value = self.value_of(setting);
                let written = if matches!(*setting, "ascii" | "reduce_motion") {
                    // Booleans in the file, `on` and `off` on the screen.
                    (value == "on").to_string()
                } else {
                    format!("\"{value}\"")
                };
                (*setting, written)
            })
            .collect();
        let written = match fs::read_to_string(path) {
            Ok(text) => upsert(&text, &updates),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                upsert(NEW_FILE_HEADER, &updates)
            }
            Err(error) => Err(error.to_string()),
        };
        let result = written.and_then(|text| {
            if let Some(folder) = path.parent() {
                fs::create_dir_all(folder).map_err(|error| error.to_string())?;
            }
            write_atomic(path, text.as_bytes(), || Ok(())).map_err(|error| error.to_string())
        });
        match result {
            Ok(()) => vec![Event::system(
                Text::new("ui.options.saved").with_str("file", path.display().to_string()),
            )],
            Err(reason) => vec![Event::error(
                Text::new("ui.options.save_failed").with_str("reason", reason),
            )],
        }
    }
}

fn choices(setting: &str) -> &'static [&'static str] {
    match setting {
        "lang" => &LANGS,
        "verbosity" => &VERBOSITIES,
        "palette" => &PALETTES,
        _ => &SWITCH,
    }
}

fn switch(on: bool) -> String {
    if on { "on" } else { "off" }.to_owned()
}

fn switch_of(value: &str) -> Option<bool> {
    match value {
        "on" | "true" => Some(true),
        "off" | "false" => Some(false),
        _ => None,
    }
}

fn verbosity_of(value: &str) -> Option<VerbosityChoice> {
    match value {
        "brief" => Some(VerbosityChoice::Brief),
        "normal" => Some(VerbosityChoice::Normal),
        "full" => Some(VerbosityChoice::Full),
        _ => None,
    }
}

/// Reads a line: `None` when it is not a presentation command.
fn request(line: &str) -> Option<Request> {
    let mut words = line.split_whitespace();
    let command = words.next()?.to_ascii_lowercase();
    let rest: Vec<&str> = words.collect();
    match (command.as_str(), rest.as_slice()) {
        ("options", []) => Some(Request::Show),
        ("options", [save]) if save.eq_ignore_ascii_case("save") => Some(Request::Save),
        ("options", [setting, value]) => Some(Request::Set {
            setting: (*setting).to_owned(),
            value: (*value).to_owned(),
        }),
        ("options", _) => Some(Request::Usage("ui.options.usage")),
        ("verbosity", []) => Some(Request::Verbosity(None)),
        ("verbosity", [value]) => Some(Request::Verbosity(Some((*value).to_owned()))),
        ("verbosity", _) => Some(Request::Usage("ui.options.verbosity_usage")),
        _ => None,
    }
}

/// `text` with each `key = value` of `updates` written: the line of that key is replaced if
/// there is one, the key is added before the first section (or at the end) if not. Comments,
/// blank lines, line endings and keys that are not in `updates` are untouched. `Err` if one
/// of the keys is inside a section: the file is not what the game writes, and a key there
/// would be read as the section's.
fn upsert(text: &str, updates: &[(&str, String)]) -> Result<String, String> {
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let section_at = lines
        .iter()
        .position(|line| line.trim_start().starts_with('['));
    let key_of = |line: &str| -> Option<String> {
        let line = line.trim_start();
        if line.starts_with('#') {
            return None;
        }
        line.split_once('=').map(|(name, _)| name.trim().to_owned())
    };
    let is_managed =
        |line: &str| key_of(line).is_some_and(|name| updates.iter().any(|(key, _)| *key == name));
    let top_level = section_at.unwrap_or(lines.len());
    if lines.iter().skip(top_level).any(|line| is_managed(line)) {
        return Err("a managed setting is inside a section of the file".to_owned());
    }
    let mut missing: Vec<String> = Vec::new();
    for (key, value) in updates {
        let found = lines
            .iter()
            .take(top_level)
            .position(|line| key_of(line).is_some_and(|name| name == *key));
        match found {
            Some(index) => {
                if let Some(line) = lines.get_mut(index) {
                    *line = format!("{key} = {value}");
                }
            }
            None => missing.push(format!("{key} = {value}")),
        }
    }
    for (offset, line) in missing.into_iter().enumerate() {
        lines.insert(top_level + offset, line);
    }
    let mut out = lines.join(eol);
    out.push_str(eol);
    Ok(out)
}

#[cfg(test)]
mod tests;
