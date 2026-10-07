use std::collections::BTreeSet;

use proptest::prelude::*;

use super::*;
use crate::event::Role;
use crate::text::{Arg, Catalog, Lang, RenderMode, Text, render};

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
        !bought
            .events
            .iter()
            .any(|e| matches!(e, Event::Changed { .. })),
        "the trace was already at 0: nothing moved, so nothing is announced"
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

fn catalog(lang: Lang) -> Catalog {
    Catalog::embedded(lang).unwrap()
}

/// Every text the game sent in a step: events and the prompt, nested texts apart.
fn texts_of(step: &Step) -> Vec<Text> {
    let mut texts = Vec::new();
    for event in &step.events {
        match event {
            Event::Message { text, .. } => texts.push(text.clone()),
            Event::Decor { alt, .. } => texts.push(alt.clone()),
            Event::Screen(table) => {
                texts.push(table.title.clone());
                texts.extend(table.columns.iter().cloned());
                texts.extend(table.rows.iter().flatten().cloned());
            }
            Event::Changed { gauge, band, .. } => {
                texts.push(Text::new(gauge.name_key()));
                texts.push(band.clone());
            }
            Event::Break => {}
        }
    }
    match &step.prompt {
        Prompt::Text { label, .. } => texts.push(label.clone()),
        Prompt::Confirm { question, .. } => texts.push(question.clone()),
        Prompt::Choice(choice) => {
            texts.push(choice.title.clone());
            texts.extend(choice.cancel.iter().cloned());
            for option in &choice.options {
                texts.push(option.label.clone());
                texts.extend(option.available.as_ref().err().cloned());
            }
        }
        Prompt::Command | Prompt::Continue | Prompt::End => {}
    }
    texts
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

/// A session that goes through the prologue, the stall, errors, alerts and the end.
fn full_session(seed: u64) -> Vec<Step> {
    let mut game = DemoGame::new(seed);
    let mut steps = vec![game.start()];
    let mut script = vec![
        Input::Continue,
        line("Zoë"),
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
    ];
    script.extend((0..8).map(|_| line("scan")));
    script.extend([line("quit"), Input::Confirm(true), Input::Eof]);
    for input in script {
        steps.push(game.handle(input));
    }
    steps
}

#[test]
fn a_full_session_never_uses_a_key_missing_from_either_language() {
    let mut found = BTreeSet::new();
    for step in full_session(21) {
        for text in texts_of(&step) {
            collect_keys(&text, &mut found);
        }
    }
    assert!(
        found.len() > 25,
        "the session should exercise many texts: {}",
        found.len()
    );
    for lang in Lang::ALL {
        let catalog = catalog(lang);
        for key in &found {
            assert!(
                catalog.get(key).is_some(),
                "key `{key}` is missing from the {lang:?} catalog"
            );
        }
    }
}

/// What a frontend must never show: a missing key, or a placeholder without its argument.
fn assert_clean(rendered: &str, context: &str) {
    assert!(!rendered.contains("<missing:"), "{context}: {rendered}");
    assert!(!rendered.contains("<?"), "{context}: {rendered}");
}

#[test]
fn every_text_of_a_session_renders_cleanly_in_every_language_and_mode() {
    for seed in [3, 21] {
        for step in full_session(seed) {
            for text in texts_of(&step) {
                for lang in Lang::ALL {
                    let catalog = catalog(lang);
                    for mode in [
                        RenderMode::FULL,
                        RenderMode::SCREEN_READER,
                        RenderMode::ASCII,
                    ] {
                        let rendered = render(&text, &catalog, mode);
                        assert_clean(&rendered, &format!("{lang:?} {mode:?} {}", text.key));
                        if mode == RenderMode::ASCII {
                            assert!(rendered.is_ascii(), "{lang:?} {}: {rendered}", text.key);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn a_scan_reads_correctly_with_one_port_and_with_several() {
    let lines = |lang, found| {
        render(
            &Text::new("demo.scan.found").with_int("found", found),
            &catalog(lang),
            RenderMode::FULL,
        )
    };
    assert_eq!(lines(Lang::En, 1), "The scan finds 1 open port.");
    assert_eq!(lines(Lang::En, 4), "The scan finds 4 open ports.");
    assert_eq!(lines(Lang::Fr, 1), "Le scan trouve 1 port ouvert.");
    assert_eq!(lines(Lang::Fr, 4), "Le scan trouve 4 ports ouverts.");
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

#[test]
fn buying_the_proxy_lowers_a_raised_trace_and_says_so() {
    let mut game = at_command_line(7);
    game.handle(line("scan"));
    let raised = game.view().gauges[0].value;
    assert!(raised >= 5);
    game.handle(line("shop"));
    let bought = game.handle(line("proxy"));
    let moves: Vec<(i32, i32)> = bought
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Changed { from, to, .. } => Some((*from, *to)),
            _ => None,
        })
        .collect();
    assert_eq!(moves, [(raised, (raised - 10).max(0))]);
}

#[test]
fn a_gauge_at_the_end_of_its_range_never_announces_a_zero_move() {
    let mut game = at_command_line(11);
    for _ in 0..30 {
        game.handle(line("scan"));
    }
    assert_eq!(game.view().gauges[0].value, TRACE_MAX);
    let at_the_cap = game.handle(line("scan"));
    assert!(
        !at_the_cap
            .events
            .iter()
            .any(|e| matches!(e, Event::Changed { .. })),
        "scanning at the cap moves nothing"
    );
    // Every Changed event of a whole session really moves its gauge.
    let mut game = at_command_line(4);
    for input in ["shop", "1", "0", "scan", "scan", "shop", "2", "0"] {
        for event in game.handle(line(input)).events {
            if let Event::Changed { from, to, .. } = event {
                assert_ne!(from, to);
            }
        }
    }
}

#[test]
fn resuming_gives_the_current_prompt_and_says_nothing_new() {
    let mut game = DemoGame::new(2);
    let first = game.start();
    assert!(
        !first.events.is_empty(),
        "a new game starts with its prologue"
    );
    assert_eq!(game.resume().prompt, Prompt::Continue);
    assert!(game.resume().events.is_empty());
    for input in [
        Input::Continue,
        line("Neon"),
        Input::Confirm(true),
        line("shop"),
    ] {
        game.handle(input);
    }
    let resumed = game.resume();
    assert!(resumed.events.is_empty());
    assert!(
        matches!(resumed.prompt, Prompt::Choice(_)),
        "back in the open menu"
    );
    assert_eq!(resumed.prompt, game.prompt());
    assert!(!resumed.save_requested);
}
