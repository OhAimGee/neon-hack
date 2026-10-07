use std::collections::BTreeSet;

use proptest::prelude::*;

use super::*;
use crate::event::Role;
use crate::text::{Arg, Catalog, placeholders};

fn line(text: &str) -> Input {
    Input::Line(text.to_owned())
}

/// Plays the prologue: continue, a handle, confirmation. Leaves the game at the command line.
fn at_command_line(seed: u64) -> DemoGame {
    let mut game = DemoGame::new(seed);
    game.start();
    for input in [Input::Continue, line("Neon"), Input::Confirm(true)] {
        game.handle(input);
    }
    assert_eq!(game.prompt(), Prompt::Command);
    game
}

fn error_keys(step: &Step) -> Vec<String> {
    step.events
        .iter()
        .filter_map(|event| match event {
            Event::Message {
                role: Role::Error,
                text,
                ..
            } => Some(text.key.to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_prologue_asks_for_a_handle_and_confirms_it() {
    let mut game = DemoGame::new(1);
    let first = game.start();
    assert_eq!(first.prompt, Prompt::Continue);
    assert!(matches!(first.events.first(), Some(Event::Decor { .. })));

    assert!(matches!(
        game.handle(Input::Continue).prompt,
        Prompt::Text { .. }
    ));
    let ask = game.handle(line("  Neon\u{7}  "));
    let Prompt::Confirm { question, default } = ask.prompt else {
        panic!("expected a confirmation, got {:?}", ask.prompt);
    };
    assert!(default);
    assert!(
        question
            .args
            .contains(&("name", Arg::Str("Neon".to_owned())))
    );

    // Refusing sends the player back to the question.
    assert!(matches!(
        game.handle(Input::Confirm(false)).prompt,
        Prompt::Text { .. }
    ));
    game.handle(line(""));
    let done = game.handle(Input::Confirm(true));
    assert_eq!(done.prompt, Prompt::Command);
    assert_eq!(game.view().player, "Case");
}

#[test]
fn clean_name_strips_trims_cuts_and_never_returns_nothing() {
    assert_eq!(clean_name("  Neon \t"), "Neon");
    assert_eq!(clean_name("a\u{1b}[31mb"), "a[31mb");
    assert_eq!(clean_name(&"x".repeat(50)).chars().count(), NAME_MAX_CHARS);
    assert_eq!(clean_name("   "), DEFAULT_NAME);
    assert_eq!(clean_name("\u{7}\u{7}"), DEFAULT_NAME);
}

#[test]
fn nested_menus_are_states_not_blocking_reads() {
    let mut game = at_command_line(7);
    assert!(matches!(
        game.handle(line("shop")).prompt,
        Prompt::Choice(_)
    ));

    // Buying keeps the stall open, and the bought entry now says why it is unavailable.
    let bought = game.handle(line("1"));
    let Prompt::Choice(choice) = &bought.prompt else {
        panic!("the stall must stay open, got {:?}", bought.prompt);
    };
    assert_eq!(
        choice.options[0].available,
        Err(Text::new("demo.shop.owned"))
    );
    assert!(bought.save_requested);
    assert!(
        bought
            .events
            .iter()
            .any(|e| matches!(e, Event::Changed { .. }))
    );

    // Refusals are said in words, nothing is spent, the stall stays open.
    let owned = game.handle(line("proxy"));
    assert_eq!(error_keys(&owned), ["demo.shop.owned"]);
    let too_dear = game.handle(line("3"));
    assert_eq!(error_keys(&too_dear), ["demo.shop.cannot_afford"]);
    assert!(!too_dear.save_requested);
    let nonsense = game.handle(line("banana"));
    assert_eq!(error_keys(&nonsense), ["ui.invalid_choice"]);

    // Back out of the stall to the command line.
    assert_eq!(game.handle(Input::Cancel).prompt, Prompt::Command);
}

#[test]
fn every_entry_is_available_or_says_why_not() {
    let mut game = at_command_line(7);
    game.handle(line("shop"));
    let Prompt::Choice(choice) = game.prompt() else {
        panic!("expected the stall");
    };
    // 50 credits: the proxy chain (30) can be bought, the others cannot yet.
    assert_eq!(choice.options[0].available, Ok(()));
    assert_eq!(
        choice.options[1].available,
        Err(Text::new("demo.shop.cannot_afford").with_int("missing", 10))
    );
    assert!(choice.options[2].available.is_err());
}

#[test]
fn eof_ends_the_game_at_any_depth_and_a_finished_game_stays_finished() {
    let scripts: [&[Input]; 6] = [
        &[],
        &[Input::Continue],
        &[Input::Continue, line("Neon")],
        &[Input::Continue, line("Neon"), Input::Confirm(true)],
        &[
            Input::Continue,
            line("Neon"),
            Input::Confirm(true),
            line("shop"),
        ],
        &[
            Input::Continue,
            line("Neon"),
            Input::Confirm(true),
            line("quit"),
        ],
    ];
    for script in scripts {
        let mut game = DemoGame::new(3);
        game.start();
        for input in script {
            game.handle(input.clone());
        }
        let ended = game.handle(Input::Eof);
        assert_eq!(ended.prompt, Prompt::End, "after {script:?}");
        assert_eq!(game.prompt(), Prompt::End);
        let after = game.handle(line("scan"));
        assert!(after.events.is_empty(), "a finished game says nothing more");
        assert_eq!(after.prompt, Prompt::End);
    }
}

#[test]
fn quitting_asks_for_confirmation_and_saves() {
    let mut game = at_command_line(5);
    let asked = game.handle(line("quit"));
    assert!(matches!(
        asked.prompt,
        Prompt::Confirm { default: false, .. }
    ));
    assert_eq!(game.handle(Input::Confirm(false)).prompt, Prompt::Command);
    game.handle(line("quit"));
    let left = game.handle(Input::Confirm(true));
    assert_eq!(left.prompt, Prompt::End);
    assert!(left.save_requested);
}

#[test]
fn only_state_changing_commands_ask_for_a_save() {
    let mut game = at_command_line(5);
    for command in ["help", "status", "nonsense", ""] {
        assert!(!game.handle(line(command)).save_requested, "{command:?}");
    }
    assert!(game.handle(line("scan")).save_requested);
}

#[test]
fn scanning_moves_the_gauge_and_warns_when_security_closes_in() {
    let mut game = at_command_line(11);
    let mut warnings = Vec::new();
    let mut last_to = 0;
    for _ in 0..30 {
        let step = game.handle(line("scan"));
        for event in &step.events {
            match event {
                Event::Changed { from, to, .. } => {
                    assert_eq!(*from, last_to);
                    assert!(to >= from);
                    last_to = *to;
                }
                Event::Message {
                    role: Role::Alert(severity),
                    ..
                } => warnings.push(*severity),
                _ => {}
            }
        }
    }
    assert_eq!(game.view().gauges[0].value, last_to);
    assert_eq!(last_to, TRACE_MAX, "thirty scans always saturate the gauge");
    assert_eq!(warnings, [Severity::Warning, Severity::Danger]);
}

#[test]
fn the_same_seed_plays_the_same_game_and_another_seed_does_not() {
    let script = [line("scan"), line("scan"), line("status"), line("scan")];
    let play = |seed: u64| {
        let mut game = at_command_line(seed);
        script
            .iter()
            .map(|input| game.handle(input.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(play(42), play(42));
    assert_ne!(play(42), play(43));
}

#[test]
fn a_clone_is_an_independent_game() {
    let mut game = at_command_line(9);
    let snapshot = game.clone();
    game.handle(line("scan"));
    assert_ne!(game.view(), snapshot.view());
    let mut replay = snapshot.clone();
    replay.handle(line("scan"));
    assert_eq!(replay.view(), game.view());
}

#[test]
fn completion_only_offers_what_can_be_used_now() {
    let mut game = at_command_line(2);
    assert_eq!(game.complete("s"), ["scan", "shop", "status"]);
    assert_eq!(game.complete("SH"), ["shop"]);
    assert!(game.complete("scan x").is_empty());
    assert!(game.complete("zz").is_empty());
    game.handle(line("shop"));
    assert_eq!(game.complete("c"), ["cloak"]);
    game.handle(Input::Cancel);
    game.handle(line("quit"));
    assert!(
        game.complete("s").is_empty(),
        "no completion on a yes/no question"
    );
}

// ---- Catalogs ----------------------------------------------------------------------------

fn keys(entries: &[(&str, &str)]) -> BTreeSet<String> {
    entries.iter().map(|(key, _)| (*key).to_owned()).collect()
}

#[test]
fn both_languages_have_the_same_keys_with_the_same_placeholders() {
    let (en, fr) = (catalog_en(), catalog_fr());
    assert_eq!(keys(en.entries()), keys(fr.entries()));
    for (key, english) in en.entries() {
        let french = fr.lookup(key).unwrap();
        assert_eq!(
            placeholders(english),
            placeholders(french),
            "placeholders of `{key}` differ between languages"
        );
    }
}

#[test]
fn mode_variants_keep_the_placeholders_of_their_base_key() {
    for catalog in [catalog_en(), catalog_fr()] {
        for (key, template) in catalog.entries() {
            let Some((base, _mode)) = key.split_once('@') else {
                continue;
            };
            let base_template = catalog
                .lookup(base)
                .unwrap_or_else(|| panic!("variant `{key}` has no base key"));
            assert_eq!(
                placeholders(template),
                placeholders(base_template),
                "`{key}`"
            );
        }
    }
}

fn collect_keys(text: &Text, found: &mut BTreeSet<String>) {
    found.insert(text.key.to_string());
    for (_, arg) in &text.args {
        match arg {
            Arg::Text(inner) => collect_keys(inner, found),
            Arg::Term(term) => {
                found.insert(format!("term.{term}"));
            }
            Arg::Int(_) | Arg::Str(_) => {}
        }
    }
}

fn collect_event_keys(event: &Event, found: &mut BTreeSet<String>) {
    match event {
        Event::Message { text, .. } => collect_keys(text, found),
        Event::Decor { alt, .. } => collect_keys(alt, found),
        Event::Screen(table) => {
            collect_keys(&table.title, found);
            table
                .columns
                .iter()
                .for_each(|text| collect_keys(text, found));
            table
                .rows
                .iter()
                .flatten()
                .for_each(|text| collect_keys(text, found));
        }
        Event::Changed { gauge, band, .. } => {
            found.insert(gauge.name_key().to_owned());
            collect_keys(band, found);
        }
        Event::Break => {}
    }
}

fn collect_prompt_keys(prompt: &Prompt, found: &mut BTreeSet<String>) {
    match prompt {
        Prompt::Text { label, .. } => collect_keys(label, found),
        Prompt::Confirm { question, .. } => collect_keys(question, found),
        Prompt::Choice(choice) => {
            collect_keys(&choice.title, found);
            choice
                .cancel
                .iter()
                .for_each(|text| collect_keys(text, found));
            for option in &choice.options {
                collect_keys(&option.label, found);
                if let Err(reason) = &option.available {
                    collect_keys(reason, found);
                }
            }
        }
        Prompt::Command | Prompt::Continue | Prompt::End => {}
    }
}

#[test]
fn a_full_session_never_uses_a_key_missing_from_either_language() {
    let mut game = DemoGame::new(21);
    let mut found = BTreeSet::new();
    let mut steps = vec![game.start()];
    let script = [
        Input::Continue,
        line("Neon"),
        Input::Confirm(true),
        line("help"),
        line("status"),
        line("nonsense"),
        Input::Choice(0),
        line("shop"),
        line("1"),
        line("1"),
        line("3"),
        line("banana"),
        Input::Cancel,
        line("scan"),
        line("scan"),
        line("scan"),
        line("scan"),
        line("scan"),
        line("scan"),
        line("scan"),
        line("scan"),
        line("quit"),
        Input::Confirm(true),
    ];
    for input in script {
        steps.push(game.handle(input));
    }
    steps.push(game.handle(Input::Eof));
    for step in &steps {
        step.events
            .iter()
            .for_each(|event| collect_event_keys(event, &mut found));
        collect_prompt_keys(&step.prompt, &mut found);
    }
    assert!(
        found.len() > 25,
        "the session should exercise many texts: {}",
        found.len()
    );
    for catalog in [catalog_en(), catalog_fr()] {
        for key in &found {
            assert!(
                catalog.lookup(key).is_some(),
                "key `{key}` is missing from a catalog"
            );
        }
    }
}

// ---- Properties --------------------------------------------------------------------------

fn any_input() -> impl Strategy<Value = Input> {
    prop_oneof![
        4 => Just(line("scan")),
        2 => Just(line("shop")),
        2 => Just(line("1")),
        1 => Just(line("2")),
        1 => Just(line("0")),
        1 => Just(line("")),
        1 => Just(line("help")),
        1 => Just(line("status")),
        1 => Just(line("quit")),
        1 => Just(line("Neo")),
        2 => Just(Input::Confirm(true)),
        1 => Just(Input::Confirm(false)),
        2 => Just(Input::Continue),
        1 => Just(Input::Cancel),
        1 => (0usize..5).prop_map(Input::Choice),
        1 => Just(Input::Eof),
    ]
}

proptest! {
    #[test]
    fn the_prompt_always_matches_the_last_step_and_the_gauge_stays_in_range(
        seed: u64,
        inputs in proptest::collection::vec(any_input(), 0..80),
    ) {
        let mut game = DemoGame::new(seed);
        let first = game.start();
        prop_assert_eq!(&first.prompt, &game.prompt());
        let mut finished = false;
        for input in inputs {
            let step = game.handle(input);
            prop_assert_eq!(&step.prompt, &game.prompt());
            let view = game.view();
            prop_assert!((0..=TRACE_MAX).contains(&view.gauges[0].value));
            if finished {
                prop_assert!(step.events.is_empty());
                prop_assert_eq!(step.prompt.clone(), Prompt::End);
            }
            finished = step.prompt == Prompt::End;
        }
    }
}
