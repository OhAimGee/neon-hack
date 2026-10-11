use std::fs;
use std::path::{Path, PathBuf};

use neon_engine::text::RenderMode;
use tempfile::TempDir;

use super::*;
use crate::config::{FileState, load_file};
use crate::render::Verbosity;
use crate::test_support::{catalog_en, catalog_fr};

fn live() -> Live {
    Live {
        lang: LangChoice::En,
        verbosity: VerbosityChoice::Normal,
        ascii: false,
        palette: PaletteChoice::Default,
        reduce_motion: false,
    }
}

fn fixed() -> Fixed {
    Fixed {
        screen_reader: false,
        color: true,
        truecolor: false,
    }
}

fn host<'a>(en: &'a Catalog, fr: &'a Catalog, target: Target) -> Host<'a> {
    Host {
        catalogs: Catalogs { en, fr },
        live: live(),
        fixed: fixed(),
        target,
        changed: Vec::new(),
    }
}

/// What the events say, one line per row, in the language the renderer has.
fn said(renderer: &Renderer<'_>, events: &[Event]) -> String {
    events
        .iter()
        .flat_map(|event| renderer.event(event))
        .map(|line| line.text)
        .collect::<Vec<_>>()
        .join("\n")
}

fn plain_renderer(catalog: &Catalog) -> Renderer<'_> {
    Renderer {
        catalog,
        mode: RenderMode::FULL,
        verbosity: Verbosity::Normal,
        colors: None,
    }
}

fn file_target(folder: &TempDir) -> Target {
    Target::File(folder.path().join("settings.toml"))
}

fn ran(host: &mut Host<'_>, line: &str) -> Vec<Event> {
    host.run(line).unwrap_or_else(|| panic!("{line:?} is ours"))
}

#[test]
fn only_the_presentation_commands_are_taken_whatever_their_case() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut host = host(&en, &fr, Target::Off);
    for line in [
        "",
        "   ",
        "hack localhost",
        "optionsx",
        "option",
        "verbose",
        "help options",
    ] {
        assert!(host.run(line).is_none(), "{line:?}");
    }
    for line in [
        "options",
        "OPTIONS",
        "  Options  ",
        "verbosity",
        "Verbosity full",
    ] {
        assert!(host.run(line).is_some(), "{line:?}");
    }
}

#[test]
fn the_screen_lists_each_setting_its_value_and_its_choices() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut host = host(&en, &fr, Target::File("/data/settings.toml".into()));
    let text = said(&plain_renderer(&en), &ran(&mut host, "options"));
    for expected in [
        "Settings",
        "lang",
        "en, fr",
        "verbosity",
        "brief, normal, full",
        "ascii",
        "on, off",
        "palette",
        "default, high-contrast, cvd, mono",
        "reduce_motion",
        "screen_reader",
        "at launch",
        "`options save` keeps them in /data/settings.toml.",
        "Usage: `options [setting value]`",
    ] {
        assert!(text.contains(expected), "{expected:?} missing:\n{text}");
    }
    // In French, with the same facts.
    let text = said(&plain_renderer(&fr), &ran(&mut host, "options"));
    assert!(
        text.contains("Réglages") && text.contains("au lancement"),
        "{text}"
    );
}

#[test]
fn started_without_saving_the_screen_does_not_offer_to_keep_anything() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut host = host(&en, &fr, Target::Off);
    let text = said(&plain_renderer(&en), &ran(&mut host, "options"));
    assert!(text.contains("--no-save"), "{text}");
    assert!(!text.contains("keeps them"), "{text}");
}

#[test]
fn a_setting_changes_is_confirmed_and_reaches_the_renderer() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut host = host(&en, &fr, Target::Off);
    let mut renderer = plain_renderer(&en);
    for (line, confirmation) in [
        ("options lang fr", "Setting changed: lang = fr."),
        ("options verbosity full", "verbosity = full"),
        ("options ascii on", "ascii = on"),
        ("options palette cvd", "palette = cvd"),
        ("options reduce_motion on", "reduce_motion = on"),
    ] {
        let events = ran(&mut host, line);
        host.apply(&mut renderer);
        // The confirmation is read in the language that was just chosen.
        let text = said(&renderer, &events);
        assert!(
            text.contains(confirmation) || text.contains("Réglage changé : lang = fr."),
            "{line}: {text}"
        );
    }
    assert_eq!(
        host.live,
        Live {
            lang: LangChoice::Fr,
            verbosity: VerbosityChoice::Full,
            ascii: true,
            palette: PaletteChoice::Cvd,
            reduce_motion: true,
        }
    );
    assert!(std::ptr::eq(renderer.catalog, &raw const fr));
    assert!(renderer.mode.ascii);
    assert_eq!(renderer.verbosity, Verbosity::Full);
    assert_eq!(
        renderer.colors.map(|colors| colors.choice),
        Some(PaletteChoice::Cvd)
    );
    // And back.
    ran(&mut host, "options lang en");
    host.apply(&mut renderer);
    assert!(std::ptr::eq(renderer.catalog, &raw const en));
}

#[test]
fn words_may_be_typed_in_any_case() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut host = host(&en, &fr, Target::Off);
    ran(&mut host, "OPTIONS Verbosity BRIEF");
    assert_eq!(host.live.verbosity, VerbosityChoice::Brief);
    ran(&mut host, "options ASCII True");
    assert!(host.live.ascii);
}

#[test]
fn a_mistake_is_said_with_what_can_be_done_and_changes_nothing() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut host = host(&en, &fr, Target::Off);
    let before = host.live;
    let renderer = plain_renderer(&en);
    let cases = [
        (
            "options speed fast",
            "Unknown setting: speed. See `options`.",
        ),
        (
            "options lang de",
            "Unknown value for lang: de. Choices: en, fr.",
        ),
        ("options verbosity loud", "Choices: brief, normal, full."),
        ("options ascii maybe", "Choices: on, off."),
        ("options screen_reader on", "set at launch"),
        ("options lang", "Usage: `options [setting value]`"),
        ("options lang fr extra", "Usage: `options [setting value]`"),
        ("verbosity a b", "Usage: `verbosity [brief|normal|full]`"),
        ("verbosity loud", "Unknown value for verbosity"),
    ];
    for (line, expected) in cases {
        let events = ran(&mut host, line);
        let text = said(&renderer, &events);
        assert!(text.contains(expected), "{line}: {text}");
        assert!(
            events.iter().all(|event| matches!(
                event,
                Event::Message {
                    role: neon_engine::event::Role::Error,
                    ..
                }
            )),
            "{line}: an error, not a confirmation"
        );
        assert_eq!(host.live, before, "{line}");
    }
}

#[test]
fn a_screen_reader_is_never_given_less_than_normal() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut host = host(&en, &fr, Target::Off);
    host.fixed.screen_reader = true;
    let text = said(&plain_renderer(&en), &ran(&mut host, "verbosity brief"));
    assert!(text.contains("never goes below"), "{text}");
    assert_eq!(host.live.verbosity, VerbosityChoice::Normal);
    // More is allowed.
    ran(&mut host, "verbosity full");
    assert_eq!(host.live.verbosity, VerbosityChoice::Full);
}

#[test]
fn the_verbosity_command_says_the_current_level_or_sets_it() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut host = host(&en, &fr, Target::Off);
    let renderer = plain_renderer(&en);
    assert!(said(&renderer, &ran(&mut host, "verbosity")).contains("Verbosity: normal."));
    ran(&mut host, "verbosity brief");
    assert!(said(&renderer, &ran(&mut host, "verbosity")).contains("Verbosity: brief."));
}

#[test]
fn without_colour_a_palette_is_kept_but_the_renderer_stays_without_colour() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let mut host = host(&en, &fr, Target::Off);
    host.fixed.color = false;
    let mut renderer = plain_renderer(&en);
    let text = said(&renderer, &ran(&mut host, "options palette cvd"));
    assert!(text.contains("No colour here"), "{text}");
    host.apply(&mut renderer);
    assert!(renderer.colors.is_none());
    assert_eq!(host.live.palette, PaletteChoice::Cvd);
}

// ---- Keeping the settings ---------------------------------------------------------------

fn settings_path(folder: &TempDir) -> PathBuf {
    folder.path().join("settings.toml")
}

#[test]
fn saving_creates_the_file_when_there_is_none() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let folder = tempfile::tempdir().unwrap();
    let mut host = host(&en, &fr, file_target(&folder));
    ran(&mut host, "options lang fr");
    ran(&mut host, "options ascii on");
    let text = said(&plain_renderer(&en), &ran(&mut host, "options save"));
    assert!(text.contains("Settings kept in "), "{text}");
    let written = fs::read_to_string(settings_path(&folder)).unwrap();
    assert!(
        written.starts_with("# Written by the options command"),
        "{written}"
    );
    for line in ["lang = \"fr\"", "ascii = true"] {
        assert!(
            written.lines().any(|l| l == line),
            "{line:?} missing:\n{written}"
        );
    }
    // Only what the player changed is kept: the rest comes from elsewhere (the command line,
    // the environment) and is not frozen by accident.
    for untouched in ["verbosity", "palette", "reduce_motion"] {
        assert!(
            !written.contains(untouched),
            "{untouched} was not changed:\n{written}"
        );
    }
    // What the game wrote, the game reads back.
    let (_, state) = load_file(&settings_path(&folder));
    assert_eq!(state, FileState::Loaded);
}

#[test]
fn saving_changes_the_managed_lines_and_keeps_everything_else() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let folder = tempfile::tempdir().unwrap();
    fs::write(
        settings_path(&folder),
        "# my own notes\nlang = \"en\"   # english please\ndisplay = \"tui\"\nfuture_key = 3\n\n# the end\n",
    )
    .unwrap();
    let mut host = host(&en, &fr, file_target(&folder));
    ran(&mut host, "options lang fr");
    ran(&mut host, "options verbosity full");
    ran(&mut host, "options save");
    let written = fs::read_to_string(settings_path(&folder)).unwrap();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines[0], "# my own notes", "{written}");
    assert!(lines.contains(&"lang = \"fr\""), "{written}");
    assert!(lines.contains(&"verbosity = \"full\""), "{written}");
    assert!(
        lines.contains(&"display = \"tui\""),
        "a key it does not manage stays: {written}"
    );
    assert!(lines.contains(&"future_key = 3"), "{written}");
    assert!(lines.contains(&"# the end"), "{written}");
    assert!(
        !written.contains("english please"),
        "the replaced line is replaced whole"
    );
    assert_eq!(written.matches("lang =").count(), 1, "{written}");
    let (_, state) = load_file(&settings_path(&folder));
    assert_eq!(state, FileState::Loaded);
    // A second save changes nothing: the file is stable.
    ran(&mut host, "options save");
    assert_eq!(fs::read_to_string(settings_path(&folder)).unwrap(), written);
}

#[test]
fn saving_keeps_the_line_endings_of_a_file_edited_on_windows() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let folder = tempfile::tempdir().unwrap();
    fs::write(
        settings_path(&folder),
        "lang = \"en\"\r\ndisplay = \"tui\"\r\n",
    )
    .unwrap();
    let mut host = host(&en, &fr, file_target(&folder));
    ran(&mut host, "options ascii on");
    ran(&mut host, "options save");
    let written = fs::read_to_string(settings_path(&folder)).unwrap();
    assert!(
        written.contains("\r\n") && !written.replace("\r\n", "").contains('\n'),
        "{written:?}"
    );
}

#[test]
fn new_keys_go_before_the_first_section_and_a_key_inside_one_is_refused() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let folder = tempfile::tempdir().unwrap();
    fs::write(
        settings_path(&folder),
        "lang = \"en\"\n[other]\nthing = 1\n",
    )
    .unwrap();
    let mut host = host(&en, &fr, file_target(&folder));
    ran(&mut host, "options verbosity full");
    ran(&mut host, "options save");
    let written = fs::read_to_string(settings_path(&folder)).unwrap();
    let section = written.lines().position(|line| line == "[other]").unwrap();
    let verbosity = written
        .lines()
        .position(|line| line.starts_with("verbosity"))
        .unwrap();
    assert!(verbosity < section, "{written}");

    // A managed key inside a section is not what the game writes: the file stays as it is.
    fs::write(settings_path(&folder), "[other]\nlang = \"en\"\n").unwrap();
    let before = fs::read_to_string(settings_path(&folder)).unwrap();
    ran(&mut host, "options lang fr");
    let text = said(&plain_renderer(&en), &ran(&mut host, "options save"));
    assert!(text.contains("Settings not kept"), "{text}");
    assert_eq!(fs::read_to_string(settings_path(&folder)).unwrap(), before);
}

#[test]
fn a_damaged_file_or_a_game_started_without_saving_is_never_written() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let folder = tempfile::tempdir().unwrap();
    fs::write(settings_path(&folder), "lang = [broken\n").unwrap();
    let mut host = host(&en, &fr, Target::Damaged(settings_path(&folder)));
    let text = said(&plain_renderer(&en), &ran(&mut host, "options save"));
    assert!(
        text.contains("is damaged") && text.contains("left as it is"),
        "{text}"
    );
    assert_eq!(
        fs::read_to_string(settings_path(&folder)).unwrap(),
        "lang = [broken\n"
    );

    host.target = Target::Off;
    let text = said(&plain_renderer(&en), &ran(&mut host, "options save"));
    assert!(text.contains("nothing is written"), "{text}");
    assert_eq!(
        fs::read_dir(folder.path()).unwrap().count(),
        1,
        "no stray file"
    );
}

#[test]
fn saving_in_a_folder_that_does_not_exist_yet_creates_it() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("deep").join("er").join("settings.toml");
    let mut host = host(&en, &fr, Target::File(path.clone()));
    ran(&mut host, "options ascii on");
    ran(&mut host, "options save");
    assert!(Path::new(&path).is_file());
}

#[test]
fn saving_with_nothing_changed_says_so_and_writes_nothing() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let folder = tempfile::tempdir().unwrap();
    let mut host = host(&en, &fr, file_target(&folder));
    let text = said(&plain_renderer(&en), &ran(&mut host, "options save"));
    assert!(text.contains("Nothing to keep"), "{text}");
    assert!(!settings_path(&folder).exists());
    // A refused change is not a change.
    ran(&mut host, "options lang de");
    ran(&mut host, "options save");
    assert!(!settings_path(&folder).exists());
}

#[test]
fn each_save_keeps_what_was_changed_so_far() {
    let (en, fr) = (catalog_en(), catalog_fr());
    let folder = tempfile::tempdir().unwrap();
    let mut host = host(&en, &fr, file_target(&folder));
    ran(&mut host, "options lang fr");
    ran(&mut host, "options save");
    let first = fs::read_to_string(settings_path(&folder)).unwrap();
    assert!(
        first.contains("lang = \"fr\"") && !first.contains("ascii"),
        "{first}"
    );
    ran(&mut host, "options ascii on");
    ran(&mut host, "options lang en");
    ran(&mut host, "options save");
    let second = fs::read_to_string(settings_path(&folder)).unwrap();
    assert!(
        second.contains("lang = \"en\"") && second.contains("ascii = true"),
        "{second}"
    );
    assert_eq!(second.matches("lang").count(), 1, "{second}");
}

#[test]
fn upsert_is_a_pure_text_edit() {
    let updates = [("a", "1".to_owned()), ("b", "2".to_owned())];
    assert_eq!(upsert("", &updates).unwrap(), "a = 1\nb = 2\n");
    assert_eq!(upsert("a=0\n", &updates).unwrap(), "a = 1\nb = 2\n");
    assert_eq!(
        upsert("  b  =  0\n# a = 9\n", &updates).unwrap(),
        "b = 2\n# a = 9\na = 1\n"
    );
    assert!(upsert("[t]\na = 1\n", &updates).is_err());
    assert_eq!(
        upsert("x = 1\n[t]\ny = 2\n", &updates).unwrap(),
        "x = 1\na = 1\nb = 2\n[t]\ny = 2\n"
    );
}

proptest::proptest! {
    /// Whatever the file holds before, what `upsert` writes holds each key once with its value.
    #[test]
    fn every_key_ends_up_once_with_its_value(
        before in "(#[ -~]{0,20}\n|[a-z_]{1,8} = [0-9]{1,3}\n|\n){0,8}",
    ) {
        let updates = [("lang", "\"fr\"".to_owned()), ("ascii", "true".to_owned())];
        let after = upsert(&before, &updates).unwrap();
        for (key, value) in &updates {
            let wanted = format!("{key} = {value}");
            proptest::prop_assert_eq!(after.lines().filter(|line| *line == wanted).count(), 1);
            let others = after
                .lines()
                .filter(|line| !line.starts_with('#') && line.split('=').next().is_some_and(|n| n.trim() == *key))
                .count();
            proptest::prop_assert_eq!(others, 1);
        }
    }
}
