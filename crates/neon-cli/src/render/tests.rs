use neon_engine::Game;
use neon_engine::demo::DemoGame;
use neon_engine::event::{Event, Gauge, Importance, Role, Severity, Table};
use neon_engine::ids::ContactId;
use neon_engine::prompt::{Choice, ChoiceOption, Input, Prompt, Step};
use neon_engine::text::{RenderMode, Text};

use super::*;
use crate::test_support::{catalog_en, catalog_fr};

fn renderer(catalog: &Catalog, mode: RenderMode, verbosity: Verbosity) -> Renderer<'_> {
    Renderer {
        catalog,
        mode,
        verbosity,
    }
}

fn texts(lines: &[Line]) -> Vec<&str> {
    lines.iter().map(|line| line.text.as_str()).collect()
}

fn echo7() -> ContactId {
    ContactId::new("echo7").unwrap()
}

#[test]
fn every_role_reads_as_text_without_colour() {
    let catalog = catalog_en();
    let r = renderer(&catalog, RenderMode::Full, Verbosity::Normal);
    let line = |event: Event| texts(&r.event(&event)).join("|");
    assert_eq!(line(Event::narration(Text::raw("rain"))), "rain");
    assert_eq!(line(Event::system(Text::raw("info"))), "info");
    assert_eq!(line(Event::say(echo7(), Text::raw("hi"))), "ECHO-7 » hi");
    assert_eq!(
        line(Event::alert(Severity::Danger, Text::raw("run"))),
        "[ALERT] run"
    );
    assert_eq!(line(Event::reward(Text::raw("+5"))), "[Reward] +5");
    assert_eq!(line(Event::error(Text::raw("no"))), "[Error] no");
}

#[test]
fn roles_follow_the_language() {
    let catalog = catalog_fr();
    let r = renderer(&catalog, RenderMode::Full, Verbosity::Normal);
    let alert = r.event(&Event::alert(Severity::Warning, Text::raw("fuyez")));
    assert_eq!(texts(&alert), ["[ALERTE] fuyez"]);
    let reward = r.event(&Event::reward(Text::raw("+5")));
    assert_eq!(texts(&reward), ["[Gain] +5"]);
}

#[test]
fn dialogue_uses_a_plain_colon_for_screen_readers_and_ascii() {
    let catalog = catalog_en();
    for mode in [RenderMode::ScreenReader, RenderMode::Ascii] {
        let r = renderer(&catalog, mode, Verbosity::Normal);
        let lines = r.event(&Event::say(echo7(), Text::raw("hi")));
        assert_eq!(texts(&lines), ["ECHO-7: hi"], "{mode:?}");
    }
}

#[test]
fn line_kinds_let_a_frontend_style_without_changing_the_text() {
    let catalog = catalog_en();
    let r = renderer(&catalog, RenderMode::Full, Verbosity::Normal);
    let kind = |event: Event| r.event(&event)[0].kind;
    assert_eq!(
        kind(Event::say(echo7(), Text::raw("x"))),
        LineKind::Dialogue
    );
    assert_eq!(
        kind(Event::alert(Severity::Danger, Text::raw("x"))),
        LineKind::Alert(Severity::Danger)
    );
    assert_eq!(kind(Event::error(Text::raw("x"))), LineKind::Error);
    assert_eq!(kind(Event::reward(Text::raw("x"))), LineKind::Reward);
}

#[test]
fn verbosity_filters_by_importance_but_never_hides_what_is_critical() {
    let catalog = catalog_en();
    let flavor = Event::flavor(Text::raw("rain"));
    let normal = Event::narration(Text::raw("story"));
    let counts = |verbosity| {
        let r = renderer(&catalog, RenderMode::Full, verbosity);
        (
            r.event(&flavor).len(),
            r.event(&normal).len(),
            r.event(&Event::Break).len(),
        )
    };
    assert_eq!(counts(Verbosity::Full), (1, 1, 1));
    assert_eq!(counts(Verbosity::Normal), (0, 1, 1));
    assert_eq!(counts(Verbosity::Brief), (0, 0, 0));

    let brief = renderer(&catalog, RenderMode::Full, Verbosity::Brief);
    for role in [Role::Alert(Severity::Notice), Role::Reward, Role::Error] {
        // Even an event filled in by hand with the lowest importance is still shown.
        let sneaky = Event::Message {
            role,
            importance: Importance::Flavor,
            text: Text::raw("x"),
        };
        assert_eq!(brief.event(&sneaky).len(), 1);
    }
}

#[test]
fn decor_is_art_then_alt_text_then_nothing() {
    let catalog = catalog_en();
    let decor = Event::Decor {
        art: &["+--+", "|NH|"],
        alt: Text::new("demo.banner.alt"),
    };
    let lines =
        |mode, verbosity| texts(&renderer(&catalog, mode, verbosity).event(&decor)).join("|");
    assert_eq!(lines(RenderMode::Full, Verbosity::Normal), "+--+||NH|");
    assert_eq!(lines(RenderMode::Ascii, Verbosity::Normal), "NEON HACK");
    assert_eq!(lines(RenderMode::ScreenReader, Verbosity::Normal), "");
    assert_eq!(
        lines(RenderMode::ScreenReader, Verbosity::Full),
        "NEON HACK"
    );
    // Decoration is not essential: brief output drops it in every mode.
    for mode in [
        RenderMode::Full,
        RenderMode::Ascii,
        RenderMode::ScreenReader,
    ] {
        assert_eq!(lines(mode, Verbosity::Brief), "", "{mode:?}");
    }
}

#[test]
fn a_gauge_that_did_not_move_says_nothing() {
    let catalog = catalog_en();
    let r = renderer(&catalog, RenderMode::Full, Verbosity::Normal);
    let unmoved = Event::Changed {
        gauge: Gauge::Trace,
        from: 7,
        to: 7,
        band: Text::new("band.calm"),
    };
    assert!(r.event(&unmoved).is_empty(), "no `Trace +0` line");
}

fn help_screen() -> Event {
    Event::Screen(Table {
        title: Text::new("demo.help.title"),
        columns: vec![
            Text::new("demo.help.col_command"),
            Text::new("demo.help.col_effect"),
        ],
        rows: vec![
            vec![Text::raw("scan"), Text::new("demo.help.scan")],
            vec![Text::raw("shop"), Text::new("demo.help.shop")],
        ],
    })
}

#[test]
fn a_screen_is_a_numbered_list_and_says_its_columns_for_screen_readers() {
    let catalog = catalog_en();
    let full = renderer(&catalog, RenderMode::Full, Verbosity::Normal).event(&help_screen());
    assert_eq!(
        texts(&full),
        [
            "Commands",
            "[1] scan - Scan the network",
            "[2] shop - Visit the stall"
        ]
    );
    let sr = renderer(&catalog, RenderMode::ScreenReader, Verbosity::Normal).event(&help_screen());
    assert_eq!(
        texts(&sr),
        [
            "Commands",
            "1. Command: scan; Effect: Scan the network",
            "2. Command: shop; Effect: Visit the stall"
        ]
    );
    let fr =
        renderer(&catalog_fr(), RenderMode::ScreenReader, Verbosity::Normal).event(&help_screen());
    assert_eq!(
        fr[1].text,
        "1. Commande : scan ; Effet : Scanner le réseau".replace(" ; ", "; ")
    );
}

#[test]
fn a_gauge_change_gives_numbers_and_the_level_in_words() {
    let catalog = catalog_en();
    let changed = |from, to| Event::Changed {
        gauge: Gauge::Trace,
        from,
        to,
        band: Text::new("band.calm"),
    };
    let say =
        |mode, event| texts(&renderer(&catalog, mode, Verbosity::Normal).event(&event)).join("");
    assert_eq!(
        say(RenderMode::Full, changed(4, 14)),
        "Trace +10 (4 → 14, calm)."
    );
    assert_eq!(
        say(RenderMode::Ascii, changed(4, 14)),
        "Trace +10 (4 -> 14, calm)."
    );
    assert_eq!(
        say(RenderMode::ScreenReader, changed(4, 14)),
        "Trace up by 10, from 4 to 14, level calm."
    );
    assert_eq!(
        say(RenderMode::Full, changed(14, 4)),
        "Trace -10 (14 → 4, calm)."
    );
    assert_eq!(
        say(RenderMode::ScreenReader, changed(14, 4)),
        "Trace down by 10, from 14 to 4, level calm."
    );
}

fn menu() -> Prompt {
    Prompt::Choice(Choice {
        title: Text::raw("Stall"),
        options: vec![
            ChoiceOption {
                id: "a".to_owned(),
                label: Text::raw("Alpha"),
                available: Ok(()),
            },
            ChoiceOption {
                id: "b".to_owned(),
                label: Text::raw("Beta"),
                available: Err(Text::new("demo.shop.owned")),
            },
        ],
        cancel: Some(Text::raw("Back")),
    })
}

#[test]
fn prompts_show_what_is_expected_and_why_an_entry_is_unavailable() {
    let catalog = catalog_en();
    let r = renderer(&catalog, RenderMode::Full, Verbosity::Normal);

    assert_eq!(r.prompt(&Prompt::Command).marker, "> ");
    assert_eq!(r.prompt(&Prompt::End).marker, "");
    assert_eq!(r.prompt(&Prompt::Continue).marker, "[Enter] to continue");

    let choice = r.prompt(&menu());
    assert_eq!(
        texts(&choice.header),
        [
            "Stall",
            "[1] Alpha",
            "[2] Beta (You already own it.)",
            "[0] Back"
        ]
    );
    assert_eq!(choice.marker, "? ");

    let text = Prompt::Text {
        label: Text::new("demo.prompt.name"),
        max_chars: 20,
        default: Some("Case".to_owned()),
    };
    assert_eq!(r.prompt(&text).marker, "Your handle [Case]: ");

    let confirm = |default| Prompt::Confirm {
        question: Text::new("demo.quit.confirm"),
        default,
    };
    assert_eq!(r.prompt(&confirm(true)).marker, "Leave the net? [Y/n] ");
    assert_eq!(r.prompt(&confirm(false)).marker, "Leave the net? [y/N] ");

    let sr = renderer(&catalog, RenderMode::ScreenReader, Verbosity::Normal);
    assert_eq!(
        texts(&sr.prompt(&menu()).header),
        [
            "Stall",
            "1. Alpha",
            "2. Beta (You already own it.)",
            "0. Back"
        ]
    );
}

/// Every step of a session that touches every kind of event and prompt.
fn session() -> Vec<Step> {
    let mut game = DemoGame::new(21);
    let mut steps = vec![game.start()];
    let mut script = vec![
        Input::Continue,
        Input::Line("Zoë « 影 »".to_owned()),
        Input::Confirm(true),
    ];
    for word in [
        "help", "status", "nonsense", "shop", "1", "1", "3", "banana",
    ] {
        script.push(Input::Line(word.to_owned()));
    }
    script.push(Input::Cancel);
    script.extend((0..8).map(|_| Input::Line("scan".to_owned())));
    script.push(Input::Line("quit".to_owned()));
    script.push(Input::Confirm(true));
    for input in script {
        steps.push(game.handle(input));
    }
    steps
}

#[test]
fn a_full_session_never_renders_a_missing_key_in_any_language_mode_or_verbosity() {
    let steps = session();
    for catalog in [catalog_en(), catalog_fr()] {
        for mode in [
            RenderMode::Full,
            RenderMode::ScreenReader,
            RenderMode::Ascii,
        ] {
            for verbosity in [Verbosity::Brief, Verbosity::Normal, Verbosity::Full] {
                let r = renderer(&catalog, mode, verbosity);
                for step in &steps {
                    let mut lines: Vec<Line> = step
                        .events
                        .iter()
                        .flat_map(|event| r.event(event))
                        .collect();
                    let view = r.prompt(&step.prompt);
                    lines.extend(view.header);
                    lines.push(Line::new(LineKind::System, view.marker));
                    for line in lines {
                        assert!(
                            !line.text.contains("<missing") && !line.text.contains("<?"),
                            "{mode:?}/{verbosity:?}: {:?}",
                            line.text
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn ascii_mode_gives_ascii_only_text_in_both_languages_whatever_the_player_typed() {
    // The handle of `session()` has an accent, guillemets and a character with no equivalent.
    for catalog in [catalog_en(), catalog_fr()] {
        let r = renderer(&catalog, RenderMode::Ascii, Verbosity::Full);
        let mut greeted = false;
        for step in session() {
            let mut lines: Vec<Line> = step
                .events
                .iter()
                .flat_map(|event| r.event(event))
                .collect();
            let view = r.prompt(&step.prompt);
            lines.extend(view.header);
            lines.push(Line::new(LineKind::System, view.marker));
            for line in lines {
                assert!(line.text.is_ascii(), "not ASCII: {:?}", line.text);
                greeted |= line.text.contains("Zoe \" ? \"");
            }
        }
        assert!(greeted, "the handle must be shown, transliterated");
    }
}
