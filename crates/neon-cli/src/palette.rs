//! Colour as a decision, then as a choice of palette.
//!
//! The decision (is there any colour at all?) follows the precedence of
//! `docs/design/tui-and-accessibility.md` section 5.1:
//!
//! 1. an explicit `--color` / `--no-color`,
//! 2. `NO_COLOR` set to something (an empty value is ignored, as no-color.org says),
//! 3. the `color` setting of `settings.toml`,
//! 4. detection: standard output is a terminal and `TERM` is not `dumb`.
//!
//! No colour means the `mono` palette: attributes only (bold, reverse, underline, dim), so
//! that nothing is told by colour alone. A screen reader never gets an escape sequence,
//! whatever was asked. This module has no terminal library in it: a [`Style`] is plain
//! data, turned into escape sequences by the plain frontend and into widget styles by the
//! full-screen one.

use clap::ValueEnum;
use neon_engine::event::Severity;
use serde::Deserialize;

use crate::render::LineKind;

/// Whether to use colour, as asked on the command line or in `settings.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ColorChoice {
    /// Colour when the output is a terminal that can show it.
    Auto,
    /// Colour even on a pipe.
    Always,
    /// No colour.
    Never,
}

/// The palettes the player can pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PaletteChoice {
    /// The terminal's own named colours; transparent, so it follows the theme.
    Default,
    /// White on black, opaque.
    HighContrast,
    /// Okabe-Ito colours, for colour-vision deficiencies; opaque.
    Cvd,
    /// No colour at all. Chosen automatically when there is no colour.
    Mono,
}

impl PaletteChoice {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::HighContrast => "high-contrast",
            Self::Cvd => "cvd",
            Self::Mono => "mono",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "default" => Some(Self::Default),
            "high-contrast" => Some(Self::HighContrast),
            "cvd" => Some(Self::Cvd),
            "mono" => Some(Self::Mono),
            _ => None,
        }
    }
}

/// What the terminal is known to do, from the environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Terminal {
    /// Standard output is a terminal.
    pub(crate) is_tty: bool,
    /// `TERM` is `dumb`.
    pub(crate) dumb: bool,
    /// `COLORTERM` says `truecolor` or `24bit`.
    pub(crate) truecolor: bool,
}

/// The inputs of the colour decision, from the most to the least specific.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ColorInputs {
    /// `--color` (`--no-color` is `Never`).
    pub(crate) cli: Option<ColorChoice>,
    /// `NO_COLOR` is set and not empty.
    pub(crate) no_color: bool,
    /// `color` in `settings.toml`.
    pub(crate) file: Option<ColorChoice>,
    pub(crate) terminal: Terminal,
    /// Screen-reader mode: no escape sequence at all.
    pub(crate) screen_reader: bool,
}

/// Where the decision came from, for `--print-settings`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorSource {
    ScreenReader,
    Cli,
    NoColorVariable,
    File,
    Detection,
}

impl ColorSource {
    pub(crate) fn describe(self) -> &'static str {
        match self {
            Self::ScreenReader => "screen reader: never any escape sequence",
            Self::Cli => "command line",
            Self::NoColorVariable => "NO_COLOR",
            Self::File => "settings.toml",
            Self::Detection => "detection",
        }
    }
}

/// Decides whether there is colour.
pub(crate) fn decide(inputs: ColorInputs) -> (bool, ColorSource) {
    let from_choice = |choice: ColorChoice, terminal: Terminal| match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => terminal.is_tty && !terminal.dumb,
    };
    if inputs.screen_reader {
        (false, ColorSource::ScreenReader)
    } else if let Some(choice) = inputs.cli {
        (from_choice(choice, inputs.terminal), ColorSource::Cli)
    } else if inputs.no_color {
        (false, ColorSource::NoColorVariable)
    } else if let Some(choice) = inputs.file {
        (from_choice(choice, inputs.terminal), ColorSource::File)
    } else {
        (
            from_choice(ColorChoice::Auto, inputs.terminal),
            ColorSource::Detection,
        )
    }
}

/// A colour the terminal knows by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ansi {
    Black,
    Red,
    Green,
    Yellow,
    Magenta,
    Cyan,
    White,
}

impl Ansi {
    /// The SGR code of the foreground, 30 to 37.
    pub(crate) fn sgr_foreground(self) -> u8 {
        30 + match self {
            Self::Black => 0,
            Self::Red => 1,
            Self::Green => 2,
            Self::Yellow => 3,
            Self::Magenta => 5,
            Self::Cyan => 6,
            Self::White => 7,
        }
    }
}

/// A foreground colour: an exact one, with the named colour that stands for it where the
/// terminal does not do 24-bit colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hue {
    pub(crate) rgb: (u8, u8, u8),
    pub(crate) ansi: Ansi,
}

const fn hue(rgb: (u8, u8, u8), ansi: Ansi) -> Hue {
    Hue { rgb, ansi }
}

/// How one kind of line is drawn: a colour and attributes. Never a blink.
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent text attributes, not a state machine"
)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Style {
    pub(crate) hue: Option<Hue>,
    pub(crate) bold: bool,
    pub(crate) reverse: bool,
    pub(crate) underline: bool,
    pub(crate) dim: bool,
}

const fn plain() -> Style {
    Style {
        hue: None,
        bold: false,
        reverse: false,
        underline: false,
        dim: false,
    }
}

/// A set of styles, one per kind of line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Palette {
    pub(crate) choice: PaletteChoice,
    /// The colour painted behind every cell, for the opaque palettes; `None` follows the
    /// terminal's theme.
    pub(crate) background: Option<Hue>,
    /// The colour of unstyled text on an opaque background.
    pub(crate) foreground: Option<Hue>,
    /// Whether the terminal does 24-bit colour: otherwise the named colours are used.
    pub(crate) truecolor: bool,
}

const BLACK: Hue = hue((0, 0, 0), Ansi::Black);
const WHITE: Hue = hue((255, 255, 255), Ansi::White);
/// The background of the `cvd` palette (see the contrast table of the design document).
const CVD_BACKGROUND: Hue = hue((0x10, 0x10, 0x10), Ansi::Black);

impl Palette {
    /// The palette for a choice, on a terminal that does or does not do 24-bit colour.
    pub(crate) fn new(choice: PaletteChoice, truecolor: bool) -> Self {
        let (background, foreground) = match choice {
            PaletteChoice::Default | PaletteChoice::Mono => (None, None),
            PaletteChoice::HighContrast => (Some(BLACK), Some(WHITE)),
            PaletteChoice::Cvd => (Some(CVD_BACKGROUND), Some(WHITE)),
        };
        Self {
            choice,
            background,
            foreground,
            truecolor,
        }
    }

    /// True when colours are sent as exact values. The default palette always uses the
    /// terminal's named colours, so that it follows the theme; the others use exact values
    /// only where the terminal does 24-bit colour.
    #[cfg(feature = "tui")]
    pub(crate) fn uses_exact_colors(&self) -> bool {
        self.truecolor && self.choice != PaletteChoice::Default
    }

    /// True when the palette paints its own background under every cell.
    #[cfg(test)]
    pub(crate) fn is_opaque(&self) -> bool {
        self.background.is_some()
    }

    /// How a kind of line is drawn. Danger is always told by reverse video too, and an
    /// error by an underline: the colour is never the only information.
    pub(crate) fn style(&self, kind: LineKind) -> Style {
        let colored = |hue: Hue, style: Style| Style {
            hue: Some(hue),
            ..style
        };
        let bold = Style {
            bold: true,
            ..plain()
        };
        let underline = Style {
            underline: true,
            ..plain()
        };
        let reverse_bold = Style {
            reverse: true,
            bold: true,
            ..plain()
        };
        if self.choice == PaletteChoice::Mono {
            return match kind {
                LineKind::Alert(Severity::Danger) => reverse_bold,
                LineKind::Alert(_) | LineKind::Dialogue | LineKind::Gauge => bold,
                LineKind::Error => underline,
                LineKind::Decor => Style {
                    dim: true,
                    ..plain()
                },
                LineKind::Narration
                | LineKind::System
                | LineKind::Blank
                | LineKind::Table
                | LineKind::Reward => plain(),
            };
        }
        let roles = self.roles();
        match kind {
            LineKind::Narration | LineKind::System | LineKind::Blank => Style {
                hue: self.foreground,
                ..plain()
            },
            LineKind::Dialogue | LineKind::Table => colored(roles.speech, plain()),
            LineKind::Alert(Severity::Danger) => colored(roles.danger, reverse_bold),
            LineKind::Alert(_) => colored(roles.warning, bold),
            LineKind::Reward => colored(roles.good, plain()),
            LineKind::Error => colored(roles.danger, underline),
            LineKind::Decor => colored(roles.accent, plain()),
            LineKind::Gauge => colored(roles.warning, plain()),
        }
    }

    fn roles(&self) -> Roles {
        match self.choice {
            // Named colours only: their real value is the theme's, so no ratio is claimed.
            PaletteChoice::Default | PaletteChoice::Mono => Roles {
                speech: hue((0, 255, 255), Ansi::Cyan),
                warning: hue((255, 255, 0), Ansi::Yellow),
                danger: hue((255, 0, 0), Ansi::Red),
                good: hue((0, 255, 0), Ansi::Green),
                accent: hue((255, 0, 255), Ansi::Magenta),
            },
            PaletteChoice::HighContrast => Roles {
                speech: hue((0, 255, 255), Ansi::Cyan),
                warning: hue((255, 255, 0), Ansi::Yellow),
                danger: hue((255, 96, 96), Ansi::Red),
                good: hue((0, 255, 0), Ansi::Green),
                accent: hue((255, 128, 255), Ansi::Magenta),
            },
            // Okabe-Ito: sky blue, orange, vermillion, bluish green, reddish purple.
            PaletteChoice::Cvd => Roles {
                speech: hue((0x56, 0xB4, 0xE9), Ansi::Cyan),
                warning: hue((0xE6, 0x9F, 0x00), Ansi::Yellow),
                danger: hue((0xD5, 0x5E, 0x00), Ansi::Red),
                good: hue((0x00, 0x9E, 0x73), Ansi::Green),
                accent: hue((0xCC, 0x79, 0xA7), Ansi::Magenta),
            },
        }
    }
}

struct Roles {
    speech: Hue,
    warning: Hue,
    danger: Hue,
    good: Hue,
    accent: Hue,
}

/// The escape sequence that starts a style on a plain terminal: 8 colours and bold, nothing
/// else (underline, reverse and dim are for the full-screen interface). `None` when the
/// style has nothing to say.
pub(crate) fn sgr_start(style: Style) -> Option<String> {
    let mut codes = Vec::new();
    if style.bold {
        codes.push(1);
    }
    if let Some(hue) = style.hue {
        codes.push(hue.ansi.sgr_foreground());
    }
    if codes.is_empty() {
        return None;
    }
    let joined: Vec<String> = codes.iter().map(ToString::to_string).collect();
    Some(format!("\x1b[{}m", joined.join(";")))
}

/// The escape sequence that ends a style.
pub(crate) const SGR_END: &str = "\x1b[0m";

#[cfg(test)]
mod tests;
