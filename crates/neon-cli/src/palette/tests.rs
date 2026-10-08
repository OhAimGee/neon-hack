use super::*;
use ColorChoice::{Always, Auto, Never};

/// Contrast ratio of two colours, WCAG 2.x: from 1 (identical) to 21 (black on white).
fn contrast(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let (lighter, darker) = {
        let (la, lb) = (luminance(a), luminance(b));
        if la >= lb { (la, lb) } else { (lb, la) }
    };
    (lighter + 0.05) / (darker + 0.05)
}

fn luminance((r, g, b): (u8, u8, u8)) -> f64 {
    let linear = |channel: u8| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

const TTY: Terminal = Terminal {
    is_tty: true,
    dumb: false,
    truecolor: false,
};
const PIPE: Terminal = Terminal {
    is_tty: false,
    dumb: false,
    truecolor: false,
};
const DUMB: Terminal = Terminal {
    is_tty: true,
    dumb: true,
    truecolor: false,
};

fn inputs(terminal: Terminal) -> ColorInputs {
    ColorInputs {
        cli: None,
        no_color: false,
        file: None,
        terminal,
        screen_reader: false,
    }
}

const ALL_KINDS: [LineKind; 12] = [
    LineKind::Narration,
    LineKind::Dialogue,
    LineKind::System,
    LineKind::Alert(Severity::Notice),
    LineKind::Alert(Severity::Warning),
    LineKind::Alert(Severity::Danger),
    LineKind::Reward,
    LineKind::Error,
    LineKind::Decor,
    LineKind::Table,
    LineKind::Gauge,
    LineKind::Blank,
];

const ALL_PALETTES: [PaletteChoice; 4] = [
    PaletteChoice::Default,
    PaletteChoice::HighContrast,
    PaletteChoice::Cvd,
    PaletteChoice::Mono,
];

#[test]
fn colour_is_detected_when_nothing_else_is_said() {
    assert_eq!(decide(inputs(TTY)), (true, ColorSource::Detection));
    assert_eq!(decide(inputs(PIPE)), (false, ColorSource::Detection));
    assert_eq!(
        decide(inputs(DUMB)),
        (false, ColorSource::Detection),
        "TERM=dumb has none"
    );
}

#[test]
fn the_precedence_is_command_line_then_no_color_then_settings_then_detection() {
    let with = |cli, no_color, file, terminal| {
        decide(ColorInputs {
            cli,
            no_color,
            file,
            terminal,
            screen_reader: false,
        })
    };
    // An explicit --color beats NO_COLOR and the file.
    assert_eq!(
        with(Some(Always), true, Some(Never), PIPE),
        (true, ColorSource::Cli)
    );
    assert_eq!(
        with(Some(Never), false, Some(Always), TTY),
        (false, ColorSource::Cli)
    );
    assert_eq!(
        with(Some(Auto), true, Some(Always), TTY),
        (true, ColorSource::Cli)
    );
    // NO_COLOR beats the file and the detection.
    assert_eq!(
        with(None, true, Some(Always), TTY),
        (false, ColorSource::NoColorVariable)
    );
    assert_eq!(
        with(None, true, None, TTY),
        (false, ColorSource::NoColorVariable)
    );
    // The file beats the detection.
    assert_eq!(
        with(None, false, Some(Always), PIPE),
        (true, ColorSource::File)
    );
    assert_eq!(
        with(None, false, Some(Never), TTY),
        (false, ColorSource::File)
    );
    assert_eq!(
        with(None, false, Some(Auto), TTY),
        (true, ColorSource::File)
    );
    assert_eq!(
        with(None, false, Some(Auto), DUMB),
        (false, ColorSource::File)
    );
}

#[test]
fn a_screen_reader_never_gets_colour_whatever_was_asked() {
    for cli in [None, Some(Always)] {
        let decided = decide(ColorInputs {
            cli,
            file: Some(Always),
            screen_reader: true,
            ..inputs(TTY)
        });
        assert_eq!(decided, (false, ColorSource::ScreenReader));
    }
}

#[test]
fn contrast_follows_wcag() {
    assert!((contrast((0, 0, 0), (255, 255, 255)) - 21.0).abs() < 1e-9);
    assert!((contrast((255, 255, 255), (0, 0, 0)) - 21.0).abs() < 1e-9);
    assert!((contrast((12, 34, 56), (12, 34, 56)) - 1.0).abs() < 1e-9);
    // Two values of the design document's table.
    assert!((contrast((0x56, 0xB4, 0xE9), (0x10, 0x10, 0x10)) - 8.25).abs() < 0.05);
    assert!((contrast((0xF0, 0xE4, 0x42), (255, 255, 255)) - 1.32).abs() < 0.05);
}

#[test]
fn every_colour_of_an_opaque_palette_reads_at_least_4_5_to_1_on_its_background() {
    for choice in [PaletteChoice::HighContrast, PaletteChoice::Cvd] {
        let palette = Palette::new(choice, true);
        let background = palette
            .background
            .expect("an opaque palette has a background")
            .rgb;
        assert!(palette.is_opaque());
        for kind in ALL_KINDS {
            if let Some(hue) = palette.style(kind).hue {
                let ratio = contrast(hue.rgb, background);
                assert!(ratio >= 4.5, "{choice:?} {kind:?}: {ratio:.2}:1");
            }
        }
        let foreground = palette.foreground.expect("and a text colour").rgb;
        assert!(contrast(foreground, background) >= 4.5, "{choice:?}");
    }
    let hc = Palette::new(PaletteChoice::HighContrast, true);
    let ratio = contrast(hc.foreground.unwrap().rgb, hc.background.unwrap().rgb);
    assert!((ratio - 21.0).abs() < 1e-9, "white on black");
}

#[test]
fn the_default_palette_is_transparent_and_mono_has_no_colour_at_all() {
    let default = Palette::new(PaletteChoice::Default, false);
    assert!(!default.is_opaque());
    assert_eq!(default.foreground, None);
    let mono = Palette::new(PaletteChoice::Mono, true);
    assert!(!mono.is_opaque());
    for kind in ALL_KINDS {
        assert_eq!(mono.style(kind).hue, None, "{kind:?}");
    }
}

#[test]
fn colour_is_never_the_only_information() {
    for choice in ALL_PALETTES {
        let palette = Palette::new(choice, true);
        let danger = palette.style(LineKind::Alert(Severity::Danger));
        assert!(
            danger.reverse && danger.bold,
            "{choice:?}: danger is reverse video"
        );
        let error = palette.style(LineKind::Error);
        assert!(error.underline, "{choice:?}: an error is underlined");
        let warning = palette.style(LineKind::Alert(Severity::Warning));
        assert!(warning.bold, "{choice:?}: a warning is bold");
        assert_ne!(danger, warning, "{choice:?}");
    }
}

#[test]
fn the_three_levels_of_alert_do_not_look_alike_in_any_palette() {
    for choice in ALL_PALETTES {
        let palette = Palette::new(choice, true);
        let styles: Vec<Style> = [Severity::Notice, Severity::Warning, Severity::Danger]
            .into_iter()
            .map(|severity| palette.style(LineKind::Alert(severity)))
            .collect();
        assert_ne!(styles[1], styles[2], "{choice:?}: warning and danger");
        // Notice and warning share a style by design (both are "bold, yellow"): the tag
        // and the words tell them apart, which is why the test stops at danger.
    }
}

#[test]
fn plain_terminals_get_eight_colours_and_bold_only() {
    for choice in ALL_PALETTES {
        let palette = Palette::new(choice, false);
        for kind in ALL_KINDS {
            let Some(start) = sgr_start(palette.style(kind)) else {
                continue;
            };
            let codes = start
                .strip_prefix("\x1b[")
                .and_then(|rest| rest.strip_suffix('m'))
                .expect("a well-formed sequence");
            for code in codes.split(';') {
                let code: u8 = code.parse().expect("a number");
                assert!(
                    code == 1 || (30..=37).contains(&code),
                    "{choice:?} {kind:?}: {start:?}"
                );
            }
        }
    }
    assert_eq!(sgr_start(Style::default()), None);
    assert_eq!(
        sgr_start(Style {
            hue: Some(hue((0, 0, 0), Ansi::Green)),
            bold: true,
            ..Style::default()
        })
        .as_deref(),
        Some("\x1b[1;32m")
    );
    assert_eq!(SGR_END, "\x1b[0m");
}

#[test]
fn palette_names_round_trip() {
    for choice in ALL_PALETTES {
        assert_eq!(PaletteChoice::from_name(choice.name()), Some(choice));
    }
    assert_eq!(
        PaletteChoice::from_name("HIGH-CONTRAST"),
        Some(PaletteChoice::HighContrast)
    );
    assert_eq!(PaletteChoice::from_name("neon"), None);
}
