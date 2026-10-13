//! The words of an intrusion: the order of the lines, their importance, the texts they cite,
//! the view against the text, the commands, the end.

use std::collections::BTreeSet;

use crate::campaign::{Difficulty, embedded_content};
use crate::command::{Capabilities, Registry};
use crate::event::{Event, Importance, Role, Table};
use crate::game::Game;
use crate::ids::ContactId;
use crate::prompt::{Input, Prompt, Step};
use crate::run::solve::{PROOF_EXPANSIONS, Proof, prove_solvable};
use crate::run::tests::{Dice, payroll, random_mission};
use crate::run::{Action, Program, Status, forecast, legal_actions};
use crate::text::{Arg, Catalog, Lang, RenderMode, Text, render};
use unicode_width::UnicodeWidthStr;

use super::commands::{COMMANDS, NAMES, SPECS};
use super::lists::Lists;
use super::{ActiveUtility, Charges, Intrusion, IntrusionGame, LootState, NodeState};

const SITE: &str = "corp-server-01";

fn game(site: &str, difficulty: Difficulty) -> IntrusionGame {
    IntrusionGame::new(site, difficulty).unwrap()
}

fn say(game: &mut IntrusionGame, line: &str) -> Step {
    game.handle(Input::Line(line.to_owned()))
}

/// What an event is, as a key: the order and the kind of lines, without their words.
fn kind(event: &Event) -> String {
    match event {
        Event::Message { text, .. } => text.key.to_string(),
        Event::Changed { .. } => "changed".to_owned(),
        Event::Screen(table) => format!("screen:{}", table.title.key),
        Event::Break => "break".to_owned(),
        Event::Decor { .. } => "decor".to_owned(),
        Event::Effect(_) => "effect".to_owned(),
    }
}

fn kinds(step: &Step) -> Vec<String> {
    step.events.iter().map(kind).collect()
}

fn int(text: &Text, name: &str) -> Option<i64> {
    text.args.iter().find_map(|(arg, value)| match value {
        Arg::Int(number) if *arg == name => Some(*number),
        _ => None,
    })
}

fn message<'a>(step: &'a Step, key: &str) -> Option<&'a Text> {
    step.events.iter().find_map(|event| match event {
        Event::Message { text, .. } if text.key == key => Some(text),
        _ => None,
    })
}

/// Every text an event, a table or a prompt cites, nested ones included.
fn collect<'a>(text: &'a Text, out: &mut Vec<&'a Text>) {
    out.push(text);
    for (_, arg) in &text.args {
        if let Arg::Text(inner) = arg {
            collect(inner, out);
        }
    }
}

fn texts_of(step: &Step) -> Vec<&Text> {
    let mut out = Vec::new();
    for event in &step.events {
        match event {
            Event::Message { text, .. } => collect(text, &mut out),
            Event::Changed { band, .. } => collect(band, &mut out),
            Event::Screen(table) => {
                collect(&table.title, &mut out);
                table
                    .columns
                    .iter()
                    .for_each(|text| collect(text, &mut out));
                table
                    .rows
                    .iter()
                    .flatten()
                    .for_each(|text| collect(text, &mut out));
            }
            Event::Decor { .. } | Event::Effect(_) | Event::Break => {}
        }
    }
    if let Prompt::Confirm { question, .. } = &step.prompt {
        collect(question, &mut out);
    }
    out
}

// -- The order of the lines (§ 3) -------------------------------------------------------

#[test]
fn the_opening_is_the_site_the_turn_and_the_forecast() {
    let mut game = game(SITE, Difficulty::Normal);
    let step = game.start();
    assert_eq!(
        kinds(&step),
        [
            "run.start",
            "run.turn.start",
            "run.forecast.total",
            "run.forecast.patrol_moves",
            "run.forecast.scan_cloak",
            "run.forecast.ambient",
        ]
    );
    assert_eq!(step.prompt, Prompt::Command);
}

#[test]
fn an_action_says_its_result_then_the_trace_then_the_projection_then_the_cycles() {
    let mut game = game(SITE, Difficulty::Normal);
    game.start();
    // A breach: result, the node that opens, the Trace; the forecast did not change.
    assert_eq!(
        kinds(&say(&mut game, "breach 2 brute")),
        ["run.breach", "run.node.open", "changed", "run.cycles"]
    );
    // Cloak changes what the end of the turn will do: the projection follows.
    assert_eq!(
        kinds(&say(&mut game, "use cloak")),
        ["run.use.cloak_left", "run.projection", "run.cycles"]
    );
    // The last cycle says what to do.
    assert_eq!(
        kinds(&say(&mut game, "move 2")),
        ["run.move", "run.cycles.none"]
    );
}

#[test]
fn the_end_of_the_turn_moves_the_patrols_scans_adds_the_ambient_and_opens_the_next_turn() {
    let mut game = game(SITE, Difficulty::Normal);
    game.start();
    say(&mut game, "use cloak");
    assert_eq!(
        kinds(&say(&mut game, "end")),
        [
            "run.end.patrol_moves",
            "run.end.scan_blocked",
            "run.end.ambient",
            "changed",
            "break",
            "run.turn.start",
            "run.forecast.total",
            "run.forecast.patrol_moves",
            "run.forecast.scan_cloak",
            "run.forecast.ambient",
        ]
    );
}

#[test]
fn a_scan_that_costs_trace_is_an_alert_and_a_cancelled_one_is_an_aside() {
    let mut game = game(SITE, Difficulty::Normal);
    game.start();
    let hit = say(&mut game, "end");
    let scan = hit
        .events
        .iter()
        .find(|event| kind(event) == "run.end.scan")
        .unwrap();
    assert!(matches!(
        scan,
        Event::Message {
            role: Role::Alert(_),
            importance: Importance::Essential,
            ..
        }
    ));

    let mut game = self::game(SITE, Difficulty::Normal);
    game.start();
    say(&mut game, "use cloak");
    let blocked = say(&mut game, "end");
    let scan = blocked
        .events
        .iter()
        .find(|event| kind(event) == "run.end.scan_blocked")
        .unwrap();
    assert_eq!(scan.importance(), Importance::Normal);
}

#[test]
fn the_total_of_the_forecast_is_essential_and_its_detail_is_not() {
    let mut game = game(SITE, Difficulty::Normal);
    let step = game.start();
    for event in &step.events {
        let importance = event.importance();
        match kind(event).as_str() {
            "run.start" | "run.turn.start" | "run.forecast.total" => {
                assert_eq!(importance, Importance::Essential);
            }
            other => assert_eq!(importance, Importance::Normal, "{other}"),
        }
    }
}

#[test]
fn nothing_the_game_says_needs_the_flavor_verbosity() {
    // RUN-14: at `brief` the player loses no number, because there is no flavor to lose.
    let mut game = game(SITE, Difficulty::Story);
    let mut events = game.start().events;
    for line in ["map", "status", "breach 2 brute", "end", "undo", "help"] {
        events.extend(say(&mut game, line).events);
    }
    assert!(
        events
            .iter()
            .all(|event| event.importance() != Importance::Flavor)
    );
}

#[test]
fn the_forecast_looks_two_turns_ahead_in_story_and_one_elsewhere() {
    let story = game(SITE, Difficulty::Story).start();
    assert!(kinds(&story).contains(&"run.forecast.next_turn".to_owned()));
    for difficulty in [Difficulty::Normal, Difficulty::Expert, Difficulty::Hardcore] {
        let step = game(SITE, difficulty).start();
        assert!(!kinds(&step).contains(&"run.forecast.next_turn".to_owned()));
    }
}

#[test]
fn a_refused_action_says_only_why() {
    let mut game = game(SITE, Difficulty::Normal);
    game.start();
    // Node 2 is linked but its ICE is up: the list refuses it with the reason.
    let step = say(&mut game, "move 2");
    assert_eq!(kinds(&step), ["error.arg.unavailable"]);
    // Loot where there is none: the rules refuse.
    assert_eq!(
        kinds(&say(&mut game, "loot")),
        ["run.error.nothing_to_loot"]
    );
    // Jacking out too early.
    assert_eq!(kinds(&say(&mut game, "jackout")), ["run.error.objectives"]);
}

// -- Every text exists, in both languages ----------------------------------------------

/// A random session: lines typed at random, confirmations answered at random.
fn random_session(seed: u64, site: &str, difficulty: Difficulty) -> Vec<Step> {
    let mut dice = Dice::new(seed, 3);
    let mut game = game(site, difficulty);
    let mut steps = vec![game.start()];
    let programs: Vec<&str> = Program::ALL.iter().map(|program| program.id()).collect();
    let words = [
        "map", "status", "help", "undo", "end", "end", "loot", "jackout", "quit", "bogus",
    ];
    for _ in 0..90 {
        let step = match game.prompt() {
            Prompt::End => break,
            Prompt::Confirm { .. } => game.handle(Input::Confirm(dice.below(3) > 0)),
            _ => {
                let node = 1 + dice.below(6);
                let program = programs[dice.below(programs.len())];
                let line = match dice.below(6) {
                    0 | 1 => format!("breach {node} {program}"),
                    2 | 3 => format!("move {node}"),
                    4 => format!("use {program}"),
                    _ => words[dice.below(words.len())].to_owned(),
                };
                game.handle(Input::Line(line))
            }
        };
        steps.push(step);
    }
    steps
}

#[test]
fn every_text_an_intrusion_says_exists_in_both_languages_with_its_arguments() {
    let catalogs: Vec<Catalog> = Lang::ALL
        .into_iter()
        .map(|lang| Catalog::embedded(lang).unwrap())
        .collect();
    let content = embedded_content().unwrap();
    let mut seen = BTreeSet::new();
    for (index, site) in content.sites.iter().enumerate() {
        for difficulty in [Difficulty::Story, Difficulty::Normal, Difficulty::Hardcore] {
            let seed = u64::try_from(index).unwrap() * 7;
            for step in random_session(seed, site.id.as_str(), difficulty) {
                for text in texts_of(&step) {
                    seen.insert(text.key.to_string());
                    for catalog in &catalogs {
                        let template = catalog.get(&text.key).unwrap_or_else(|| {
                            panic!("{:?}: `{}` is missing", catalog.lang(), text.key)
                        });
                        let given: BTreeSet<&str> =
                            text.args.iter().map(|(name, _)| *name).collect();
                        let wanted = template.placeholders();
                        assert!(
                            wanted.is_subset(&given),
                            "{:?}: `{}` wants {wanted:?}, gets {given:?}",
                            catalog.lang(),
                            text.key
                        );
                    }
                }
            }
        }
    }
    // A run that is burned, and one that is won, to reach the result and the report.
    let mut burned = game(SITE, Difficulty::Hardcore);
    let mut steps = vec![burned.start()];
    while burned.prompt() != Prompt::End {
        steps.push(say(&mut burned, "end"));
    }
    for step in &steps {
        for text in texts_of(step) {
            seen.insert(text.key.to_string());
            for catalog in &catalogs {
                assert!(
                    catalog.get(&text.key).is_some(),
                    "`{}` is missing",
                    text.key
                );
            }
        }
    }
    // The sweep reaches the main families of the intrusion.
    for key in [
        "run.breach",
        "run.end.scan",
        "run.projection",
        "run.alert.band",
        "run.report.title",
    ] {
        assert!(seen.contains(key), "the sweep never met `{key}`");
    }
    assert!(seen.len() > 40, "only {} texts reached", seen.len());
}

#[test]
fn the_help_of_every_command_is_written_in_both_languages() {
    assert!(Registry::issues(&COMMANDS).is_empty());
    for lang in Lang::ALL {
        let catalog = Catalog::embedded(lang).unwrap();
        assert!(COMMANDS.catalog_issues(&catalog).is_empty(), "{lang:?}");
    }
    let names: Vec<&str> = SPECS.iter().map(|spec| spec.name).collect();
    assert_eq!(names, NAMES);
}

// -- The view against the text (RUN-1, RUN-6) ------------------------------------------

fn table_of(events: &[Event]) -> Vec<&Table> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Screen(table) => Some(table),
            _ => None,
        })
        .collect()
}

fn states_to_check(seed: u64, site: &str) -> Vec<IntrusionGame> {
    let mut dice = Dice::new(seed, 5);
    let mut game = game(site, Difficulty::Normal);
    game.start();
    let mut states = vec![game.clone()];
    for _ in 0..40 {
        if game.prompt() == Prompt::End {
            break;
        }
        let legal = legal_actions(game.intrusion().run().mission(), game.intrusion().state());
        let Some(&action) = legal.get(dice.below(legal.len())) else {
            break;
        };
        if action == Action::Jackout {
            continue;
        }
        let line = line_of(action);
        game.handle(Input::Line(line));
        states.push(game.clone());
    }
    states
}

pub(super) fn line_of(action: Action) -> String {
    match action {
        Action::Move(node) => format!("move {}", node.number()),
        Action::Breach { node, program } => format!("breach {} {}", node.number(), program.id()),
        Action::Use(utility) => format!("use {}", utility.program().id()),
        Action::Loot => "loot".to_owned(),
        Action::End => "end".to_owned(),
        Action::Jackout => "jackout".to_owned(),
    }
}

#[test]
fn everything_in_the_view_is_also_said_by_map_and_status() {
    let content = embedded_content().unwrap();
    for (index, site) in content.sites.iter().enumerate() {
        for mut game in states_to_check(u64::try_from(index).unwrap(), site.id.as_str()) {
            if game.prompt() == Prompt::End {
                continue;
            }
            let view = game.run_view();
            let map = say(&mut game.clone(), "map");
            let status = say(&mut game, "status");
            let nodes = &table_of(&map.events)[0];
            assert_eq!(nodes.rows.len(), view.nodes.len());
            for (node, row) in view.nodes.iter().zip(&nodes.rows) {
                assert_eq!(row.len(), nodes.columns.len(), "RUN-6: a cell per column");
                assert_eq!(row[0], node.name);
                let state = match node.state {
                    NodeState::Here => "run.map.state.here",
                    NodeState::Open => "run.map.state.open",
                    NodeState::Closed => "run.map.state.closed",
                };
                assert_eq!(row[1].key, state);
                match node.defense {
                    None => assert_eq!(row[2].key, "run.map.defense.none"),
                    Some(ice) => {
                        assert_eq!(int(&row[2], "strength"), Some(i64::from(ice.strength)));
                        assert_eq!(int(&row[2], "progress"), Some(i64::from(ice.progress)));
                        assert!(row[2].args.contains(&("family", Arg::Term(ice.family))));
                    }
                }
                let links: Vec<String> =
                    node.links.iter().map(|n| n.number().to_string()).collect();
                assert!(row[3].args.contains(&("list", Arg::Str(links.join(", ")))));
                let loot = match node.loot {
                    LootState::None => "run.map.loot.none",
                    LootState::Visible(_) => "run.map.loot.visible",
                    LootState::Taken(_) => "run.map.loot.taken",
                };
                assert_eq!(row[4].key, loot);
            }
            // One line per patrol, with where it is and where it goes.
            let patrol_lines: Vec<&Text> = map
                .events
                .iter()
                .filter_map(|event| match event {
                    Event::Message { text, .. } if text.key.starts_with("run.map.patrol") => {
                        Some(text)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(patrol_lines.len(), view.patrols.len(), "RUN-6");
            for (patrol, line) in view.patrols.iter().zip(patrol_lines) {
                assert_eq!(int(line, "id"), Some(i64::from(patrol.id)));
                assert_eq!(line.key == "run.map.patrol_down", patrol.down_turns > 0);
            }
            // The status: Trace, cycles, deck, loot, undos, forecast.
            let trace = message(&status, "run.status.trace").unwrap();
            assert_eq!(int(trace, "value"), Some(i64::from(view.trace.value)));
            assert_eq!(int(trace, "max"), Some(i64::from(view.trace.max)));
            let cycles = message(&status, "run.cycles")
                .or_else(|| message(&status, "run.cycles.none"))
                .unwrap();
            assert_eq!(int(cycles, "left"), Some(i64::from(view.cycles.left)));
            assert_eq!(int(cycles, "limit"), Some(i64::from(view.cycles.max)));
            let deck = &table_of(&status.events)[0];
            assert_eq!(deck.rows.len(), view.deck.len());
            for (program, row) in view.deck.iter().zip(&deck.rows) {
                assert_eq!(row[0], program.name);
                match program.charges {
                    Charges::Unlimited => assert_eq!(row[4].key, "run.deck.unlimited"),
                    Charges::Left(count) => {
                        assert_eq!(int(&row[4], "count"), Some(i64::from(count)));
                    }
                }
            }
            assert_eq!(
                message(&status, "run.status.cloak").is_some(),
                view.active.contains(&ActiveUtility::Cloak)
            );
            let (taken, total) = view.loot_held;
            if total > 0 {
                let loot = message(&status, "run.status.loot").unwrap();
                assert_eq!(int(loot, "taken"), Some(i64::from(taken)));
                assert_eq!(int(loot, "total"), Some(i64::from(total)));
            }
            let forecast_lines: Vec<&Text> = status
                .events
                .iter()
                .filter_map(|event| match event {
                    Event::Message { text, .. } if text.key.starts_with("run.forecast.") => {
                        Some(text)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(forecast_lines.len(), view.forecast.len());
            for (said, shown) in forecast_lines.iter().zip(&view.forecast) {
                assert_eq!(*said, shown);
            }
            let total_line = message(&status, "run.forecast.total").unwrap();
            assert_eq!(
                int(total_line, "total"),
                Some(i64::from(view.forecast_total))
            );
        }
    }
}

// -- The projection (RUN-4) -------------------------------------------------------------

#[test]
fn the_projection_is_written_only_when_the_announced_trace_changes_and_is_then_exact() {
    use crate::content::ids::SiteId;
    for seed in 0..120u64 {
        let mission = random_mission(seed);
        let site: SiteId = "test-site".parse().unwrap();
        let mut intrusion = Intrusion::from_mission(site, Difficulty::Normal, mission);
        let mut dice = Dice::new(seed, 13);
        for _ in 0..30 {
            let before = *intrusion.state();
            let legal = legal_actions(intrusion.run().mission(), &before);
            let Some(&action) = legal.get(dice.below(legal.len())) else {
                break;
            };
            let mut events = Vec::new();
            assert!(intrusion.act(action, &mut events));
            let after = *intrusion.state();
            if !after.is_running() {
                break;
            }
            let mission = intrusion.run().mission();
            let was = forecast(mission, &before, 1).total();
            let now = forecast(mission, &after, 1);
            let projection = events.iter().find_map(|event| match event {
                Event::Message { text, .. } if text.key == "run.projection" => Some(text),
                _ => None,
            });
            if action == Action::End {
                assert!(
                    projection.is_none(),
                    "the next turn shows the whole forecast"
                );
            } else if was == now.total() {
                assert!(
                    projection.is_none(),
                    "seed {seed}: unchanged forecast announced"
                );
            } else {
                let text =
                    projection.unwrap_or_else(|| panic!("seed {seed}: change not announced"));
                assert_eq!(int(text, "total"), Some(i64::from(now.total())));
                assert_eq!(
                    int(text, "from"),
                    Some(i64::from(after.trace().min(mission.trace_cap())))
                );
            }
        }
    }
}

// -- The commands -----------------------------------------------------------------------

#[test]
fn commands_are_read_against_the_lists_of_the_screen() {
    let mut game = game(SITE, Difficulty::Normal);
    game.start();
    assert_eq!(kinds(&say(&mut game, "")), Vec::<String>::new());
    assert_eq!(kinds(&say(&mut game, "bogus")), ["error.unknown_command"]);
    assert_eq!(kinds(&say(&mut game, "move")), ["error.arg.missing"]);
    assert_eq!(kinds(&say(&mut game, "move 9")), ["error.arg.unknown"]);
    // A utility is not a breach, a breach is not a utility: the list says so.
    assert_eq!(
        kinds(&say(&mut game, "breach 2 cloak")),
        ["error.arg.unavailable"]
    );
    assert_eq!(
        kinds(&say(&mut game, "use brute")),
        ["error.arg.unavailable"]
    );
    // An alias, and a case that does not matter.
    assert_eq!(kinds(&say(&mut game, "GO 3"))[0], "run.move");
}

#[test]
fn help_lists_the_commands_and_explains_one() {
    let mut game = game(SITE, Difficulty::Normal);
    game.start();
    let help = say(&mut game, "help");
    assert_eq!(table_of(&help.events)[0].rows.len(), SPECS.len());
    assert_eq!(
        kinds(&say(&mut game, "help breach")),
        ["run.help.breach", "run.help.usage"]
    );
    assert_eq!(kinds(&say(&mut game, "help map")), ["run.help.map"]);
}

#[test]
fn completion_offers_only_what_could_be_used() {
    let game = game(SITE, Difficulty::Normal);
    assert_eq!(game.complete("ma"), ["map"]);
    // From the Gateway: node 3 is open, node 2 is not, the others are not linked.
    assert_eq!(game.complete("move "), ["3"]);
    let programs = game.complete("breach 2 ");
    assert!(programs.contains(&"brute".to_owned()));
    assert!(!programs.contains(&"cloak".to_owned()));
    assert_eq!(game.complete("use c"), ["cloak"]);
}

#[test]
fn undo_takes_back_the_turn_and_counts_what_is_left() {
    let mut game = game(SITE, Difficulty::Normal);
    game.start();
    assert_eq!(kinds(&say(&mut game, "undo")), ["run.undo.nothing"]);
    say(&mut game, "breach 2 brute");
    let step = say(&mut game, "undo");
    assert_eq!(kinds(&step)[0], "run.undo.done");
    assert_eq!(
        int(message(&step, "run.undo.done").unwrap(), "left"),
        Some(2)
    );
    assert_eq!(game.intrusion().state().trace(), 0);
    // Expert has none; Story has no limit.
    let mut expert = self::game(SITE, Difficulty::Expert);
    expert.start();
    say(&mut expert, "breach 2 brute");
    assert_eq!(kinds(&say(&mut expert, "undo")), ["run.undo.none"]);
    let mut story = self::game(SITE, Difficulty::Story);
    story.start();
    say(&mut story, "breach 2 brute");
    assert_eq!(
        kinds(&say(&mut story, "undo"))[0],
        "run.undo.done_unlimited"
    );
}

#[test]
fn jackout_asks_when_loot_is_left_behind() {
    let mut game = game(SITE, Difficulty::Story);
    game.start();
    for line in ["move 3", "breach 4 brute", "end"] {
        say(&mut game, line);
    }
    for line in ["breach 4 brute", "move 4", "loot"] {
        say(&mut game, line);
    }
    // The extra loot of the Archives is still there.
    let step = say(&mut game, "jackout");
    assert!(matches!(
        step.prompt,
        Prompt::Confirm { default: false, .. }
    ));
    assert_eq!(game.intrusion().state().status(), Status::Running);
    // No: the run goes on.
    let step = game.handle(Input::Confirm(false));
    assert_eq!(step.prompt, Prompt::Command);
    assert!(step.events.is_empty());
    // Yes: it ends with the result and the report.
    say(&mut game, "jackout");
    let step = game.handle(Input::Confirm(true));
    assert_eq!(kinds(&step), ["run.result.won", "screen:run.report.title"]);
    assert_eq!(step.prompt, Prompt::End);
}

#[test]
fn quit_asks_first_and_the_input_closing_ends_the_game() {
    let mut game = game(SITE, Difficulty::Normal);
    game.start();
    assert!(matches!(
        say(&mut game, "quit").prompt,
        Prompt::Confirm { .. }
    ));
    assert_eq!(game.handle(Input::Confirm(false)).prompt, Prompt::Command);
    say(&mut game, "exit");
    assert_eq!(game.handle(Input::Confirm(true)).prompt, Prompt::End);
    let mut game = self::game(SITE, Difficulty::Normal);
    assert_eq!(game.handle(Input::Eof).prompt, Prompt::End);
}

#[test]
fn waiting_burns_the_run_and_the_game_ends_with_the_report() {
    let mut game = game(SITE, Difficulty::Hardcore);
    game.start();
    let mut last = None;
    for _ in 0..40 {
        let step = say(&mut game, "end");
        if step.prompt == Prompt::End {
            last = Some(step);
            break;
        }
    }
    let step = last.expect("the ambient Trace always burns a run");
    let kinds = kinds(&step);
    assert!(kinds.contains(&"run.result.burned".to_owned()));
    assert_eq!(kinds.last().unwrap(), "screen:run.report.title");
    assert!(
        !kinds.contains(&"run.turn.start".to_owned()),
        "no turn after the end"
    );
    assert_eq!(game.intrusion().state().status(), Status::Burned);
    // The Trace the interface shows never goes past the cap it burns at.
    assert!(game.run_view().trace.value <= game.run_view().trace.max);
}

#[test]
fn resuming_says_where_the_player_stands_and_never_the_opening() {
    let mut game = game(SITE, Difficulty::Normal);
    game.start();
    let ended = say(&mut game, "end");
    let resume = game.resume();
    assert_eq!(resume.prompt, Prompt::Command);
    assert!(!kinds(&resume).contains(&"run.start".to_owned()));
    // The end of a turn ends on exactly what a resume says (RUN-12).
    let tail: Vec<String> = kinds(&ended)
        .into_iter()
        .skip_while(|key| key != "run.turn.start")
        .collect();
    assert_eq!(kinds(&resume), tail);
}

#[test]
fn the_game_reads_as_a_game() {
    let mut game = game(SITE, Difficulty::Normal);
    game.start();
    let view = game.view();
    assert_eq!(view.player, SITE);
    assert_eq!(view.gauges.len(), 1);
    assert_eq!(view.gauges[0].max, 100);
    assert!(
        game.snapshot().is_err(),
        "an intrusion on its own is not saved"
    );
    assert!(game.load("").is_err());
    let _ = Capabilities::PLAIN;
    let _ = ContactId::new("echo7");
}

#[test]
fn an_unknown_site_cannot_be_played() {
    assert!(IntrusionGame::new("nowhere", Difficulty::Normal).is_err());
    assert!(IntrusionGame::new("Not A Site!", Difficulty::Normal).is_err());
}

// -- Playing the whole way through the commands ------------------------------------------

#[test]
fn a_winning_plan_of_every_site_plays_through_the_commands_to_the_same_end() {
    let content = embedded_content().unwrap();
    for site in &content.sites {
        let difficulty = Difficulty::Normal;
        let mission = content.mission(&site.id, difficulty.pace()).unwrap();
        let Proof::Solvable(plan) = prove_solvable(&mission, PROOF_EXPANSIONS) else {
            panic!("{}: no plan", site.id);
        };
        let mut game = game(site.id.as_str(), difficulty);
        game.start();
        for action in &plan.actions {
            let step = say(&mut game, &line_of(*action));
            // A jackout with loot left asks; the plan means it.
            if matches!(step.prompt, Prompt::Confirm { .. }) {
                game.handle(Input::Confirm(true));
            }
        }
        assert_eq!(game.prompt(), Prompt::End, "{}", site.id);
        assert_eq!(
            game.intrusion().state().status(),
            Status::Won,
            "{}",
            site.id
        );
        assert_eq!(game.intrusion().state().trace(), plan.trace, "{}", site.id);
    }
}

#[test]
fn the_payroll_run_of_the_spec_keeps_its_numbers_through_the_lists() {
    // The mission of the spec's example, played through the same lists the game reads.
    let mission = payroll();
    let state = mission.initial_state();
    let lists = Lists {
        mission: &mission,
        state: &state,
    };
    let rows =
        crate::command::Resolver::list(&lists, crate::command::ArgKind::Node, &SPECS[3]).rows;
    assert_eq!(rows.len(), mission.node_count());
    assert_eq!(rows[0].display(), "1");
}

// -- The panel (RUN-7) -------------------------------------------------------------------

/// The panel is 30 columns wide: a row longer than that is cut or wrapped, and the rows that
/// matter most (a node, its state, its defense) must not need it.
const PANEL_COLUMNS: usize = 30;

#[test]
fn every_row_of_the_panel_fits_the_columns_of_the_panel_in_both_languages() {
    let catalogs: Vec<Catalog> = Lang::ALL
        .into_iter()
        .map(|lang| Catalog::embedded(lang).unwrap())
        .collect();
    let content = embedded_content().unwrap();
    for (index, site) in content.sites.iter().enumerate() {
        for game in states_to_check(u64::try_from(index).unwrap(), site.id.as_str()) {
            if game.prompt() == Prompt::End {
                continue;
            }
            let view = game.view();
            for catalog in &catalogs {
                for mode in [
                    RenderMode::FULL,
                    RenderMode::ASCII,
                    RenderMode::SCREEN_READER,
                ] {
                    let mut said: Vec<(String, usize)> = vec![];
                    for section in &view.sections {
                        said.push((render(&section.title, catalog, mode), PANEL_COLUMNS));
                        for row in &section.rows {
                            // A patrol that is down says why, and is allowed to wrap.
                            let limit = if row.text.key == "run.panel.patrol_down" {
                                PANEL_COLUMNS * 2
                            } else {
                                PANEL_COLUMNS
                            };
                            said.push((render(&row.text, catalog, mode), limit));
                        }
                    }
                    said.extend(
                        view.status
                            .iter()
                            .map(|text| (render(text, catalog, mode), PANEL_COLUMNS)),
                    );
                    for (row, limit) in said {
                        assert!(
                            row.width() <= limit,
                            "{} {:?}: `{row}` is {} columns",
                            site.id,
                            catalog.lang(),
                            row.width()
                        );
                    }
                    // The rows a node needs are the ones that must never wrap.
                    let nodes = &view.sections[0];
                    for row in &nodes.rows {
                        let line = render(&row.text, catalog, mode);
                        assert!(
                            line.width() <= PANEL_COLUMNS || mode == RenderMode::SCREEN_READER,
                            "{} {:?}: `{line}` is {} columns",
                            site.id,
                            catalog.lang(),
                            line.width()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn the_panel_says_what_the_view_says() {
    let content = embedded_content().unwrap();
    for (index, site) in content.sites.iter().enumerate() {
        for game in states_to_check(u64::try_from(index).unwrap(), site.id.as_str()) {
            if game.prompt() == Prompt::End {
                continue;
            }
            let (view, run) = (game.view(), game.run_view());
            let titles: Vec<&str> = view.sections.iter().map(|s| &*s.title.key).collect();
            assert_eq!(titles[0], "run.panel.nodes");
            assert_eq!(view.sections[0].rows.len(), run.nodes.len());
            let patrols = view
                .sections
                .iter()
                .find(|s| s.title.key == "run.panel.patrols");
            assert_eq!(patrols.map_or(0, |s| s.rows.len()), run.patrols.len());
            let deck = view
                .sections
                .iter()
                .find(|s| s.title.key == "run.panel.deck")
                .unwrap();
            let programs: usize = deck
                .rows
                .iter()
                .map(|row| {
                    if row.text.key == "run.panel.pair" {
                        2
                    } else {
                        1
                    }
                })
                .sum();
            assert_eq!(programs, run.deck.len());
            let forecast = view
                .sections
                .iter()
                .find(|s| s.title.key == "run.panel.forecast")
                .unwrap();
            let total = forecast
                .rows
                .iter()
                .find(|row| row.text.key == "run.panel.forecast_total")
                .unwrap();
            assert_eq!(
                int(&total.text, "total"),
                Some(i64::from(run.forecast_total))
            );
            assert_eq!(view.status.len(), 2);
            assert_eq!(int(&view.status[0], "number"), Some(i64::from(run.turn)));
            assert_eq!(
                int(&view.status[1], "left"),
                Some(i64::from(run.cycles.left))
            );
        }
    }
}
