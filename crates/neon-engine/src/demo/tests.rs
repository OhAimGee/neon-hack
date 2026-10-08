use std::collections::BTreeSet;

use proptest::prelude::*;

use super::*;
use crate::event::Role;
use crate::save::{SLOT_COUNT, SaveError, SaveRequest};
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
    assert_eq!(bought.save, Some(SaveRequest::Autosave));
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
    assert_eq!(too_dear.save, None);
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
    assert_eq!(left.save, Some(SaveRequest::Autosave));
}

#[test]
fn only_state_changing_commands_ask_for_a_save() {
    let mut game = at_command_line(5);
    for command in ["help", "status", "nonsense", ""] {
        assert_eq!(game.handle(line(command)).save, None, "{command:?}");
    }
    assert_eq!(game.handle(line("scan")).save, Some(SaveRequest::Autosave));
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
    // Tense, critical, and then the game is lost (the later scans do nothing).
    assert_eq!(
        warnings,
        [Severity::Warning, Severity::Danger, Severity::Danger]
    );
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
    assert_eq!(game.complete("s"), ["save", "scan", "shop", "status"]);
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

// ---- Playing to the end --------------------------------------------------------------------

/// The result of a whole game.
struct Played {
    won: bool,
    /// The lines typed, from the prologue to the end.
    script: Vec<String>,
}

/// A prudent player: lies low when the trace gets high, buys the cloak when it can, then
/// the deck, and scans otherwise. `None` when the game does not end within 400 actions.
fn prudent_player(seed: u64) -> Option<Played> {
    let mut game = DemoGame::new(seed);
    game.start();
    let mut script = Vec::new();
    let mut last = game.handle(Input::Continue);
    script.push(String::new());
    for (typed, input) in [("Neon", line("Neon")), ("y", Input::Confirm(true))] {
        last = game.handle(input);
        script.push(typed.to_owned());
    }
    for _ in 0..400 {
        let lines: &[&str] = if game.trace >= 45 {
            &["laylow"]
        } else if game.credits >= DECK_PRICE {
            &["shop", "deck"]
        } else if game.credits >= 60 && !game.owns("cloak") {
            &["shop", "cloak", "0"]
        } else {
            &["scan"]
        };
        for typed in lines {
            last = game.handle(line(typed));
            script.push((*typed).to_owned());
        }
        if last.prompt == Prompt::End {
            let won = last.events.iter().any(|event| {
                matches!(event, Event::Message { text, .. } if text.key == "demo.end.win.summary")
            });
            return Some(Played { won, script });
        }
    }
    None
}

#[test]
fn a_prudent_player_wins_whatever_the_seed_and_within_reason() {
    let mut longest = 0;
    for seed in 0..300 {
        let played = prudent_player(seed).unwrap_or_else(|| panic!("seed {seed} never ends"));
        assert!(played.won, "seed {seed} was lost");
        longest = longest.max(played.script.len());
    }
    assert!(longest < 120, "the longest win takes {longest} lines");
}

#[test]
fn a_reckless_player_is_always_traced_and_the_last_save_is_kept() {
    for seed in 0..100 {
        let mut game = at_command_line(seed);
        let mut previous = game.handle(line("scan"));
        let mut scans = 1;
        let last = loop {
            let step = game.handle(line("scan"));
            scans += 1;
            if step.prompt == Prompt::End {
                break step;
            }
            previous = step;
            assert!(scans < 30, "seed {seed}: thirty scans cannot all be safe");
        };
        let caught = last.events.iter().any(|event| {
            matches!(event, Event::Message { text, .. } if text.key == "demo.end.lose.caught")
        });
        assert!(caught, "seed {seed}");
        assert_eq!(last.save, None, "the fatal scan is not saved");
        assert_eq!(
            previous.save,
            Some(SaveRequest::Autosave),
            "the one before is"
        );
    }
}

#[test]
fn buying_the_deck_ends_the_game_with_the_epilogue_and_keeps_the_save_before_it() {
    let mut game = at_command_line(7);
    game.credits = DECK_PRICE;
    game.handle(line("shop"));
    let before = text_of(&game);
    let bought = game.handle(line("deck"));
    assert_eq!(bought.prompt, Prompt::End);
    assert_eq!(
        bought.save, None,
        "an ending does not overwrite the last save"
    );
    let keys: Vec<String> = bought
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Message { text, .. } => Some(text.key.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(
        keys,
        [
            "demo.shop.bought",
            "demo.end.win.echo",
            "demo.end.win.summary"
        ]
    );
    // The game that was saved before still resumes in the stall, with the deck unbought.
    let resumed = DemoGame::from_save(&before).unwrap();
    assert!(matches!(resumed.prompt(), Prompt::Choice(_)));
    assert!(!resumed.owns("deck"));
    // And once it is over, nothing more happens.
    let after = game.handle(line("scan"));
    assert!(after.events.is_empty());
    assert_eq!(after.prompt, Prompt::End);
}

#[test]
fn laying_low_cools_the_trace_and_never_below_zero() {
    let mut game = at_command_line(3);
    game.trace = 60;
    let step = game.handle(line("laylow"));
    assert_eq!(game.trace, 60 - LAY_LOW_COOLING);
    assert_eq!(step.save, Some(SaveRequest::Autosave));
    assert!(step.events.iter().any(|event| matches!(
        event,
        Event::Changed {
            from: 60,
            to: 35,
            ..
        }
    )));
    game.trace = 10;
    let cooled = game.handle(line("laylow"));
    assert_eq!(game.trace, 0);
    assert!(cooled.events.iter().any(|event| matches!(
        event,
        Event::Changed {
            from: 10,
            to: 0,
            ..
        }
    )));
    // At zero there is nothing to announce but the flavour.
    let nothing = game.handle(line("laylow"));
    assert!(
        !nothing
            .events
            .iter()
            .any(|event| matches!(event, Event::Changed { .. }))
    );
}

#[test]
fn the_cloak_halves_the_trace_a_scan_leaves_rounding_up() {
    for seed in 0..40 {
        let (mut bare, mut cloaked) = (at_command_line(seed), at_command_line(seed));
        cloaked.owned.push("cloak".to_owned());
        bare.handle(line("scan"));
        cloaked.handle(line("scan"));
        assert_eq!(cloaked.trace, (bare.trace + 1) / 2, "seed {seed}");
    }
}

// ---- Commands ------------------------------------------------------------------------------

#[test]
fn the_command_table_is_well_formed_and_every_command_has_a_handler_and_a_help_text() {
    assert_eq!(COMMANDS.issues(), []);
    for spec in COMMANDS.specs() {
        let mut game = at_command_line(1);
        let step = game.handle(line(spec.name));
        assert!(
            !error_keys(&step).contains(&"demo.unknown_command".to_owned()),
            "`{}` is declared but not handled",
            spec.name
        );
    }
    for lang in Lang::ALL {
        let catalog = catalog(lang);
        for key in COMMANDS.help_keys() {
            assert!(catalog.get(key).is_some(), "{lang:?}: `{key}` has no text");
        }
    }
}

#[test]
fn the_help_screen_lists_exactly_the_commands_that_run() {
    let mut game = at_command_line(1);
    let step = game.handle(line("help"));
    let Some(Event::Screen(table)) = step.events.first() else {
        panic!("help is a screen: {:?}", step.events);
    };
    let listed: Vec<&Text> = table.rows.iter().map(|row| &row[0]).collect();
    let declared: Vec<Text> = COMMANDS
        .specs()
        .iter()
        .map(|spec| Text::raw(spec.name))
        .collect();
    assert_eq!(listed, declared.iter().collect::<Vec<_>>());
}

#[test]
fn an_alias_runs_the_same_command_as_its_name() {
    for (alias, name) in [
        ("h", "help"),
        ("st", "status"),
        ("buy", "shop"),
        ("exit", "quit"),
    ] {
        let (mut by_alias, mut by_name) = (at_command_line(4), at_command_line(4));
        assert_eq!(
            by_alias.handle(line(alias)),
            by_name.handle(line(name)),
            "{alias} is {name}"
        );
        assert_eq!(by_alias.prompt(), by_name.prompt());
    }
    // Case does not matter, and an alias counts as one action like its name.
    let mut game = at_command_line(4);
    assert!(matches!(game.handle(line("BUY")).prompt, Prompt::Choice(_)));
    assert_eq!(game.snapshot().unwrap().matches("turn = 1").count(), 2);
}

#[test]
fn completion_prefers_names_and_falls_back_to_aliases() {
    let game = at_command_line(2);
    assert_eq!(
        game.complete("st"),
        ["status"],
        "the name wins over the alias `st`"
    );
    assert_eq!(game.complete("e"), ["exit"], "no name starts with e");
    assert_eq!(game.complete("b"), ["buy"]);
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

// ---- Saving --------------------------------------------------------------------------------

fn text_of(game: &DemoGame) -> String {
    game.snapshot().unwrap()
}

/// The same inputs played on two games must give the same steps.
fn assert_same_future(a: &mut DemoGame, b: &mut DemoGame, inputs: &[Input]) {
    for input in inputs {
        assert_eq!(
            a.handle(input.clone()),
            b.handle(input.clone()),
            "{input:?}"
        );
    }
    assert_eq!(text_of(a), text_of(b));
}

#[test]
fn entering_the_stall_asks_for_a_checkpoint_and_save_asks_for_a_slot() {
    let mut game = at_command_line(7);
    assert_eq!(
        game.handle(line("shop")).save,
        Some(SaveRequest::Checkpoint),
        "buying is permanent: the moment before is kept"
    );
    game.handle(Input::Cancel);
    assert_eq!(game.handle(line("save")).save, Some(SaveRequest::Slot(1)));
    assert_eq!(game.handle(line("save 9")).save, Some(SaveRequest::Slot(9)));
    assert_eq!(game.handle(line("SAVE 3")).save, Some(SaveRequest::Slot(3)));
}

#[test]
fn a_bad_slot_is_refused_in_words_and_nothing_is_saved() {
    let mut game = at_command_line(7);
    for bad in ["save 0", "save 10", "save x", "save -1", "save 300"] {
        let step = game.handle(line(bad));
        assert_eq!(step.save, None, "{bad}");
        assert_eq!(error_keys(&step), ["demo.save.bad_slot"], "{bad}");
    }
    assert_eq!(SLOT_COUNT, 9);
}

#[test]
fn a_game_saved_at_the_command_line_comes_back_identical() {
    let mut game = at_command_line(7);
    for _ in 0..3 {
        game.handle(line("scan"));
    }
    let mut loaded = DemoGame::from_save(&text_of(&game)).unwrap();
    assert_eq!(text_of(&loaded), text_of(&game));
    assert_eq!(loaded.view(), game.view());
    assert_eq!(loaded.resume(), game.resume());
    let future = [
        line("scan"),
        line("scan"),
        line("shop"),
        line("1"),
        Input::Cancel,
    ];
    assert_same_future(&mut game, &mut loaded, &future);
}

#[test]
fn a_game_saved_in_the_middle_of_a_menu_resumes_in_the_menu() {
    let mut game = at_command_line(7);
    game.handle(line("scan"));
    game.handle(line("shop"));
    let mut loaded = DemoGame::from_save(&text_of(&game)).unwrap();
    assert!(matches!(loaded.resume().prompt, Prompt::Choice(_)));
    assert_same_future(
        &mut game,
        &mut loaded,
        &[line("1"), line("zzz"), Input::Cancel],
    );
}

#[test]
fn the_prologue_can_be_saved_and_resumed_too() {
    let mut game = DemoGame::new(3);
    game.start();
    game.handle(Input::Continue);
    game.handle(line("Zoë"));
    let mut loaded = DemoGame::from_save(&text_of(&game)).unwrap();
    assert!(matches!(loaded.prompt(), Prompt::Confirm { .. }));
    assert_same_future(
        &mut game,
        &mut loaded,
        &[Input::Confirm(true), line("scan")],
    );
}

#[test]
fn quitting_saves_the_game_at_the_command_line_and_a_loaded_game_is_never_over() {
    let mut game = at_command_line(5);
    game.handle(line("scan"));
    game.handle(line("quit"));
    let left = game.handle(Input::Confirm(true));
    assert_eq!(left.prompt, Prompt::End);
    let loaded = DemoGame::from_save(&text_of(&game)).unwrap();
    assert_eq!(loaded.prompt(), Prompt::Command);
}

fn valid_save() -> String {
    let mut game = at_command_line(7);
    for _ in 0..3 {
        game.handle(line("scan"));
    }
    game.handle(line("shop"));
    game.handle(line("1"));
    game.handle(Input::Cancel);
    text_of(&game)
}

#[test]
fn every_strict_prefix_of_a_game_save_is_rejected() {
    let text = valid_save();
    let body = text.trim_end();
    for (cut, _) in body.char_indices() {
        assert!(DemoGame::from_save(&body[..cut]).is_err(), "prefix {cut}");
    }
    assert!(DemoGame::from_save(body).is_ok());
}

#[test]
fn impossible_states_are_rejected_not_loaded() {
    let text = valid_save();
    let line_with = |prefix: &str| {
        text.lines()
            .find(|l| l.starts_with(prefix))
            .unwrap_or_else(|| panic!("no `{prefix}` line in:\n{text}"))
            .to_owned()
    };
    let (trace, credits, name, owned) = (
        line_with("trace = "),
        line_with("credits = "),
        line_with("name = "),
        line_with("owned = "),
    );
    let cases = [
        (trace.clone(), "trace = 101"),
        (trace, "trace = -1"),
        (credits.clone(), "credits = -1"),
        (credits, "credits = 5000000"),
        (name.clone(), "name = \"\""),
        (name.clone(), "name = \" padded \""),
        (name.clone(), "name = \"tab\\there\""),
        (name, "name = \"123456789012345678901\""),
        (owned.clone(), "owned = [\"deck-of-doom\"]"),
        (owned, "owned = [\"proxy\", \"proxy\"]"),
    ];
    for (original, replacement) in cases {
        let hostile = text.replacen(&original, replacement, 1);
        assert_ne!(hostile, text);
        assert!(
            matches!(DemoGame::from_save(&hostile), Err(SaveError::Invalid(_))),
            "{replacement}: {:?}",
            DemoGame::from_save(&hostile).map(|_| ())
        );
    }
    assert!(
        text.contains("flows = []"),
        "a save at the command line has no flow:\n{text}"
    );
    let two_flows = text.replacen(
        "flows = []",
        "flows = [{ kind = \"shop\" }, { kind = \"shop\" }]",
        1,
    );
    assert!(matches!(
        DemoGame::from_save(&two_flows),
        Err(SaveError::Invalid(_))
    ));
    let unknown_flow = text.replacen("flows = []", "flows = [{ kind = \"teleport\" }]", 1);
    assert!(matches!(
        DemoGame::from_save(&unknown_flow),
        Err(SaveError::Schema(_))
    ));
    let even_stream = text.replacen(&line_with("inc = "), "inc = \"0000000000000002\"", 1);
    assert!(matches!(
        DemoGame::from_save(&even_stream),
        Err(SaveError::Invalid(_))
    ));
}

#[test]
fn a_newer_version_is_refused_and_nothing_is_loaded() {
    let newer = valid_save().replacen("version = 1", "version = 2", 1);
    assert!(matches!(
        DemoGame::from_save(&newer),
        Err(SaveError::TooNew {
            found: 2,
            supported: 1
        })
    ));
}

fn rendered_player_name(text: &str) -> Option<String> {
    DemoGame::from_save(text)
        .ok()
        .map(|game| game.view().player)
}

proptest! {
    #[test]
    fn a_saved_game_always_comes_back_and_plays_on_identically(
        seed: u64,
        before in proptest::collection::vec(any_input(), 0..60),
        after in proptest::collection::vec(any_input(), 0..30),
    ) {
        let mut game = DemoGame::new(seed);
        game.start();
        for input in before {
            game.handle(input);
        }
        let mut loaded = DemoGame::from_save(&text_of(&game)).unwrap();
        prop_assert_eq!(text_of(&loaded), text_of(&game));
        // A game that has ended is saved as it was, and a loaded game is never over:
        // there is nothing left to compare once the original has ended.
        for input in after {
            if game.prompt() == Prompt::End {
                break;
            }
            prop_assert_eq!(game.handle(input.clone()), loaded.handle(input));
        }
    }

    #[test]
    fn a_damaged_save_never_panics_and_what_loads_is_stable(
        position in 0usize..400,
        replacement in "[ -~]{0,3}",
    ) {
        let text = valid_save();
        let mut start = position.min(text.len());
        while !text.is_char_boundary(start) { start -= 1; }
        let end = (start + replacement.len().max(1)).min(text.len());
        let mut end = end;
        while !text.is_char_boundary(end) { end += 1; }
        let damaged = format!("{}{}{}", &text[..start], replacement, &text[end..]);
        if let Some(name) = rendered_player_name(&damaged) {
            prop_assert!(!name.is_empty());
            let game = DemoGame::from_save(&damaged).unwrap();
            let again = DemoGame::from_save(&text_of(&game)).unwrap();
            prop_assert_eq!(text_of(&again), text_of(&game));
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
    assert_eq!(resumed.save, None);
}
