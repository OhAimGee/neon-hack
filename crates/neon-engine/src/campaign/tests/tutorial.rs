//! The opening and the guided tutorial: the prologue pages, the offer, the state machine of
//! the steps (hints said once, at the command prompt, as soon as their command is open), the
//! skip and the restart, taking a game up again, and the saves that must be refused.

use std::collections::BTreeSet;

use proptest::prelude::*;

use super::support::*;
use crate::campaign::CampaignGame;
use crate::campaign::tutorial::{Completion, Mode, STEPS};
use crate::content::schema::QuestStatus;
use crate::content::texts::{budget, display_width};
use crate::game::Game;
use crate::prompt::{Input, Prompt};
use crate::save::SaveRequest;
use crate::text::{Lang, RenderMode, Text, render};

fn replace_once(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "`{from}` is not in:\n{text}");
    text.replacen(from, to, 1)
}

/// A text of the opening as the player reads it: the glossary terms the game gives every
/// text, a handle, and a step counter.
fn shown(key: &str, catalog: &crate::text::Catalog) -> String {
    let mut text = Text::dynamic(key.to_owned())
        .with_str("name", "Neon")
        .with_int("number", 10)
        .with_int("total", 10)
        .with_str("text", "");
    crate::campaign::keys::gloss(&mut text);
    render(&text, catalog, RenderMode::FULL)
}

/// The tutorial lines of what was said (the instruction of a step, or the end).
fn hints(said: &str) -> Vec<&str> {
    said.lines()
        .filter(|line| line.starts_with("Tutorial"))
        .collect()
}

fn step_line(number: usize, command: &str) -> String {
    format!("Tutorial, step {number} of 10: Type `{command}`")
}

/// The lines of a guided walk of the whole tutorial, with what each says next.
const WALK: &[(&str, &str)] = &[
    ("help", "Tutorial, step 2 of 10: Type `status`"),
    ("status", "Tutorial, step 3 of 10: Type `quests`"),
    ("quests", "Tutorial, step 4 of 10: Type `talk echo7`"),
    // Inside the conversation nothing is said; leaving it says the next step.
    ("talk echo7", ""),
    ("0", "Tutorial, step 5 of 10: Type `net`"),
    ("net", "Tutorial, step 6 of 10: Type `hack localhost`"),
    ("hack localhost", ""),
    ("y", "Tutorial, step 7 of 10: Type `laylow`"),
    // The end of the first quest opens the stall, in the same breath.
    ("laylow", "Tutorial, step 8 of 10: Type `shop`"),
    ("shop", "Tutorial, step 9 of 10: Type `buy 1`"),
    ("buy 1", ""),
    ("n", "Tutorial, step 10 of 10: Type `save`"),
    ("save", "Tutorial finished."),
];

// ------------------------------------------------------------------------------ the prologue

#[test]
fn the_prologue_has_three_pages_and_the_third_comes_with_the_offer() {
    let mut driver = Driver::raw();
    driver.line("Neon");
    let first = driver.text();
    assert!(first.contains("Welcome to the net, Neon."), "{first}");
    assert!(first.contains("Neo-Tokyo, 2087."), "{first}");
    assert!(first.contains("type `skip`"), "{first}");
    assert_eq!(driver.last.prompt, Prompt::Continue);
    driver.send(Input::Continue);
    assert!(driver.text().contains("Nexus Corp runs the city"));
    assert_eq!(driver.last.prompt, Prompt::Continue);
    driver.send(Input::Continue);
    assert!(driver.text().contains("Encrypted channel #7"));
    assert!(driver.prompt_text().contains("Start the guided tutorial?"));
    assert!(matches!(
        driver.last.prompt,
        Prompt::Confirm { default: true, .. }
    ));
}

#[test]
fn backing_out_or_typing_skip_jumps_to_the_contact_with_echo7() {
    for skip in [Input::Cancel, Input::Line(" SKIP ".to_owned())] {
        let mut driver = Driver::raw();
        driver.line("Neon");
        driver.send(skip.clone());
        let said = driver.text();
        assert!(!said.contains("Nexus Corp runs the city"), "{said}");
        assert!(said.contains("echo7 > Finally."), "{said}");
        assert!(matches!(driver.last.prompt, Prompt::Confirm { .. }));
        // The same from the second page.
        let mut driver = Driver::raw();
        driver.line("Neon");
        driver.send(Input::Continue);
        driver.send(skip);
        assert!(driver.text().contains("echo7 > Finally."));
    }
}

#[test]
fn the_prologue_refuses_what_is_not_continue_or_skip() {
    let mut driver = Driver::raw();
    driver.line("Neon");
    for input in [
        Input::Confirm(true),
        Input::Choice(0),
        Input::Line("help".to_owned()),
    ] {
        driver.send(input);
        assert!(driver.errored());
        assert_eq!(driver.last.prompt, Prompt::Continue);
    }
    driver.send(Input::Continue);
    driver.send(Input::Continue);
    // The offer is a yes or no.
    driver.send(Input::Continue);
    assert!(driver.errored());
    assert!(matches!(driver.last.prompt, Prompt::Confirm { .. }));
}

#[test]
fn a_game_saved_inside_the_opening_comes_back_to_the_same_page() {
    let mut driver = Driver::raw();
    let mut stages = Vec::new();
    for input in [
        Input::Line("Neon".to_owned()),
        Input::Continue,
        Input::Continue,
    ] {
        driver.send(input);
        // Every page changes the state, so each one is autosaved.
        assert_eq!(driver.saves.last(), Some(&SaveRequest::Autosave));
        stages.push((driver.game.snapshot().unwrap(), driver.last.prompt.clone()));
    }
    for (text, prompt) in stages {
        let loaded = CampaignGame::from_save(&text).unwrap();
        assert_eq!(loaded.snapshot().unwrap(), text);
        assert_eq!(loaded.prompt(), prompt);
        let step = loaded.resume();
        assert_eq!(step.prompt, prompt);
        assert!(!step.events.is_empty(), "the page is said again");
        let said: String = step
            .events
            .iter()
            .map(|event| render_event(event, &catalog(Lang::En)))
            .collect();
        assert!(!said.contains("Back on the net"), "{said}");
        // Taking it up twice says the same thing, and changes nothing.
        assert_eq!(loaded.resume(), step);
        assert_eq!(loaded.snapshot().unwrap(), text);
    }
}

#[test]
fn the_opening_ends_with_one_checkpoint_and_the_first_quest_told() {
    for guided in [true, false] {
        let mut driver = Driver::raw();
        driver.opening(guided);
        let told = driver.text();
        assert!(told.contains("New quest:"), "{told}");
        assert_eq!(driver.saves.last(), Some(&SaveRequest::Checkpoint));
        assert_eq!(
            driver
                .saves
                .iter()
                .filter(|save| **save == SaveRequest::Checkpoint)
                .count(),
            1
        );
        assert_eq!(driver.status("m01"), QuestStatus::Active);
        assert_eq!(
            driver.game.state.tutorial.mode(),
            if guided { Mode::Active } else { Mode::Off }
        );
    }
}

// ----------------------------------------------------------------------------- the tutorial

#[test]
fn accepting_the_offer_starts_with_the_first_step_and_declining_does_not() {
    let driver = Driver::guided();
    let said = driver.text();
    assert!(said.contains(&step_line(1, "help")), "{said}");
    assert!(said.contains("echo7 > First thing, rookie"), "{said}");
    assert!(
        !said.contains("Type `help` to list the commands, then"),
        "{said}"
    );
    let driver = Driver::new();
    let said = driver.text();
    assert!(hints(&said).len() == 1, "{said}");
    assert!(said.contains("Tutorial skipped."), "{said}");
    assert!(
        said.contains("Type `help` to list the commands, then"),
        "{said}"
    );
}

#[test]
fn the_walk_gives_each_hint_once_at_the_command_prompt_and_finishes() {
    let mut driver = Driver::guided();
    for (line, next) in WALK {
        let said = driver.line(line);
        assert!(!driver.errored(), "`{line}` was refused:\n{said}");
        if next.is_empty() {
            assert!(
                hints(&said).is_empty(),
                "`{line}` is inside a menu:\n{said}"
            );
            assert_ne!(driver.last.prompt, Prompt::Command, "`{line}`");
        } else {
            assert_eq!(driver.last.prompt, Prompt::Command, "`{line}`");
            let lines = hints(&said);
            assert_eq!(lines.len(), 1, "`{line}`:\n{said}");
            assert!(lines[0].starts_with(next), "`{line}`: {lines:?}");
        }
    }
    assert_eq!(driver.game.state.tutorial.mode(), Mode::Finished);
    // The first quest was done on the way, and nothing of the tutorial is said any more.
    assert_eq!(driver.status("m01"), QuestStatus::Completed);
    for line in ["help", "status", "net", "tutorial"] {
        let said = driver.line(line);
        assert!(!said.contains("Tutorial, step"), "{line}: {said}");
    }
    assert!(driver.text().contains("The tutorial is finished."));
}

#[test]
fn the_hint_of_the_stall_comes_after_the_end_of_the_quest_that_opens_it() {
    let mut driver = Driver::guided();
    driver.lines(&[
        "help",
        "status",
        "quests",
        "talk echo7",
        "0",
        "net",
        "hack localhost",
        "y",
    ]);
    // The stall is closed: the hint of its step waits, whatever the player does.
    assert!(driver.line("shop").contains("Not yet: find R4Z0R."));
    assert!(hints(&driver.text()).is_empty());
    let said = driver.line("laylow");
    let quest = said.find("Quest completed:").expect("the quest is done");
    let hint = said.find(&step_line(8, "shop")).expect("the stall opens");
    assert!(quest < hint, "{said}");
}

#[test]
fn a_step_done_ahead_of_its_turn_is_skipped_over_silently() {
    let mut driver = Driver::guided();
    for line in ["status", "net", "quests"] {
        let said = driver.line(line);
        assert!(hints(&said).is_empty(), "`{line}`: {said}");
    }
    let said = driver.line("help");
    let lines = hints(&said);
    // Steps 1 to 3 and 5 are done: the next is the conversation.
    assert_eq!(lines.len(), 1, "{said}");
    assert!(
        lines[0].starts_with(&step_line(4, "talk echo7")),
        "{lines:?}"
    );
    assert_eq!(driver.game.state.tutorial.mode(), Mode::Active);
}

#[test]
fn a_command_that_was_not_accepted_does_not_count() {
    let mut driver = Driver::guided();
    // Closed, unknown, missing argument, unknown argument: none of them teaches anything.
    for line in ["shop", "bogus", "talk", "talk nobody", "hack"] {
        driver.line(line);
        assert!(driver.errored(), "`{line}`");
    }
    assert_eq!(driver.game.state.tutorial.mode(), Mode::Active);
    assert_eq!(driver.line("help").matches("Tutorial, step 2").count(), 1);
    // An intrusion that is declined is not an intrusion.
    driver.lines(&["status", "quests", "talk echo7", "0", "net"]);
    driver.lines(&["hack localhost", "n"]);
    assert!(hints(&driver.text()).is_empty(), "{}", driver.text());
    let said = driver.lines(&["hack localhost", "y"]);
    assert!(said.contains(&step_line(7, "laylow")), "{said}");
}

#[test]
fn skipping_ends_it_forgets_it_and_leaves_nothing_in_the_save() {
    let mut driver = Driver::guided();
    driver.line("help");
    assert!(driver.game.snapshot().unwrap().contains("[state.tutorial]"));
    let said = driver.line("tutorial skip");
    assert!(said.contains("Tutorial skipped."), "{said}");
    assert!(hints(&said).len() == 1, "{said}");
    assert!(driver.game.state.tutorial.is_off());
    let text = driver.game.snapshot().unwrap();
    assert!(!text.contains("tutorial"), "{text}");
    // Later commands say nothing of it, and `skip` again only reports.
    let said = driver.line("status");
    assert!(!said.contains("Tutorial, step"), "{said}");
    assert!(
        driver
            .line("tutorial skip")
            .contains("The tutorial is off.")
    );
    assert!(driver.line("tutorial").contains("`tutorial restart`"));
}

#[test]
fn a_declined_tutorial_can_be_restarted_from_the_first_step() {
    let mut driver = Driver::new();
    driver.lines(&["help", "status"]);
    let said = driver.line("tutorial restart");
    assert!(said.contains("Tutorial restarted."), "{said}");
    // Nothing is held against steps done before it began: it starts again at the first.
    assert!(said.contains(&step_line(1, "help")), "{said}");
    assert_eq!(driver.game.state.tutorial.mode(), Mode::Active);
    // Restarting in the middle starts over.
    let said = driver.lines(&["help", "status", "tutorial restart"]);
    assert!(said.contains(&step_line(1, "help")), "{said}");
}

#[test]
fn the_tutorial_command_says_where_it_stands_without_saying_it_twice() {
    let mut driver = Driver::guided();
    driver.line("help");
    let said = driver.line("tutorial");
    assert_eq!(hints(&said).len(), 1, "{said}");
    assert!(said.contains(&step_line(2, "status")), "{said}");
    // The command said it: the next command must not say it again.
    assert!(hints(&driver.line("net")).is_empty());
    let said = driver.line("tutorial nonsense");
    assert!(driver.errored(), "{said}");
}

#[test]
fn a_step_whose_command_is_closed_waits_and_is_said_when_it_opens() {
    let mut driver = Driver::guided();
    // Every lesson up to the stall is done, but the first quest is not: the stall is closed.
    for command in ["help", "status", "quests", "talk", "net", "laylow"] {
        driver
            .game
            .state
            .tutorial
            .observe(command, Completion::Command);
    }
    driver
        .game
        .state
        .tutorial
        .observe("hack", Completion::Outcome);
    let said = driver.line("tutorial");
    assert!(said.contains("the next step is not open yet"), "{said}");
    assert!(hints(&driver.line("contacts")).is_empty() || driver.text().contains("not open"));
    driver.lines(&["quests", "help", "net", "status", "talk echo7", "0"]);
    assert!(hints(&driver.text()).len() <= 1);
    driver.lines(&["hack localhost", "y"]);
    let said = driver.line("laylow");
    assert!(said.contains(&step_line(8, "shop")), "{said}");
}

#[test]
fn the_tutorial_never_gets_in_the_way_of_the_game() {
    // The greedy player of the story does not read a hint and still finishes the same three
    // quests for the same credits, with the tutorial on or off.
    let play = |mut driver: Driver| {
        for line in [
            "quests",
            "help",
            "net",
            "status",
            "talk echo7",
            "1",
            "hack localhost",
            "y",
            "laylow",
            "hack corp-server-01",
            "y",
            "talk r4z0r",
            "1",
            "shop",
            "buy stealth",
            "y",
            "hack localhost",
            "y",
            "hack corp-server-01",
            "y",
            "hack underground-market",
            "y",
            "archives",
            "decrypt doc_phase2",
        ] {
            driver.line(line);
            assert!(!driver.errored(), "`{line}`");
        }
        driver
    };
    let (guided, plain) = (play(Driver::guided()), play(Driver::new()));
    for quest in ["m01", "m02", "m03"] {
        assert_eq!(guided.status(quest), QuestStatus::Completed);
    }
    assert_eq!(guided.credits(), plain.credits());
    assert_eq!(guided.game.notoriety(), plain.game.notoriety());
    assert_eq!(
        guided.game.state.missions.reputation,
        plain.game.state.missions.reputation
    );
    assert_eq!(guided.game.state.hints, plain.game.state.hints);
}

#[test]
fn the_steps_teach_commands_of_the_table_in_the_order_the_quest_asks_for_them() {
    let names: Vec<&str> = crate::campaign::command_names().collect();
    for step in STEPS {
        assert!(names.contains(&step.command), "{}", step.id);
    }
    let ids: BTreeSet<&str> = STEPS.iter().map(|step| step.id).collect();
    assert_eq!(ids.len(), STEPS.len(), "step ids are unique");
    // Only the last step closes the tutorial.
    assert_eq!(
        STEPS
            .iter()
            .filter(|step| step.completion == Completion::Closing)
            .count(),
        1
    );
    assert_eq!(
        STEPS.last().map(|step| step.completion),
        Some(Completion::Closing)
    );
    // The tutorial walks the objectives of M01 in a possible order: what the quest asks for
    // is all taught before the stall, which the end of the quest opens.
    let c = Driver::new().game.content;
    let m01 = c.quest(&"m01".parse().unwrap()).unwrap();
    assert_eq!(m01.objective.len(), 7);
    let position = |id: &str| STEPS.iter().position(|step| step.id == id).unwrap();
    for taught in ["help", "status", "quests", "talk", "net", "hack", "laylow"] {
        assert!(position(taught) < position("shop"), "{taught}");
    }
}

// ----------------------------------------------------------------------- taking a game up again

#[test]
fn a_loaded_game_says_its_step_again_but_never_repeats_it_after_the_next_command() {
    let mut driver = Driver::guided();
    driver.line("help");
    let text = driver.game.snapshot().unwrap();
    let mut loaded = CampaignGame::from_save(&text).unwrap();
    let step = loaded.resume();
    let said: String = step
        .events
        .iter()
        .map(|event| render_event(event, &catalog(Lang::En)))
        .collect();
    assert!(said.contains(&step_line(2, "status")), "{said}");
    // Only the instruction: ECHO-7's line was said before the save.
    assert!(!said.contains("echo7 >"), "{said}");
    // Taking it up again changes nothing, and says the same.
    assert_eq!(loaded.resume(), step);
    assert_eq!(loaded.snapshot().unwrap(), text);
    // The next command, if it is not the step, says nothing of it.
    let after = loaded.handle(Input::Line("net".to_owned()));
    let said: String = after
        .events
        .iter()
        .map(|event| render_event(event, &catalog(Lang::En)))
        .collect();
    assert!(hints(&said).is_empty(), "{said}");
}

#[test]
fn a_hint_waiting_for_the_prompt_is_said_after_a_load_when_the_menu_closes() {
    let mut driver = Driver::guided();
    driver.lines(&["help", "status", "quests", "talk echo7"]);
    let text = driver.game.snapshot().unwrap();
    let mut loaded = CampaignGame::from_save(&text).unwrap();
    assert!(matches!(loaded.prompt(), Prompt::Choice(_)));
    // Nothing about the tutorial while the menu is open.
    let said: String = loaded
        .resume()
        .events
        .iter()
        .map(|event| render_event(event, &catalog(Lang::En)))
        .collect();
    assert!(hints(&said).is_empty(), "{said}");
    let after = loaded.handle(Input::Line("0".to_owned()));
    let said: String = after
        .events
        .iter()
        .map(|event| render_event(event, &catalog(Lang::En)))
        .collect();
    assert!(said.contains(&step_line(5, "net")), "{said}");
}

#[test]
fn the_save_at_every_stage_of_the_walk_loads_back_byte_for_byte() {
    let mut driver = Driver::raw();
    let mut stages = vec![driver.game.snapshot().unwrap()];
    for input in [
        Input::Line("Neon".to_owned()),
        Input::Continue,
        Input::Continue,
        Input::Confirm(true),
    ] {
        driver.send(input);
        stages.push(driver.game.snapshot().unwrap());
    }
    for (line, _) in WALK {
        driver.line(line);
        stages.push(driver.game.snapshot().unwrap());
    }
    assert!(
        stages.iter().any(|text| text.contains("mode = \"active\"")),
        "an active tutorial was saved"
    );
    assert!(
        stages.last().unwrap().contains("mode = \"finished\""),
        "{}",
        stages.last().unwrap()
    );
    for text in stages {
        let loaded = CampaignGame::from_save(&text).unwrap();
        assert_eq!(loaded.snapshot().unwrap(), text);
    }
}

#[test]
fn a_save_without_the_tutorial_is_a_game_that_never_had_one() {
    let mut driver = Driver::guided();
    driver.line("help");
    let text = driver.game.snapshot().unwrap();
    // The format of a save that predates the tutorial has no such table.
    let old: String = {
        let mut lines = Vec::new();
        let mut inside = false;
        for line in text.lines() {
            if line.starts_with('[') {
                inside = line == "[state.tutorial]";
            }
            if !inside {
                lines.push(line);
            }
        }
        lines.join("\n") + "\n"
    };
    assert!(!old.contains("tutorial"), "{old}");
    let mut loaded = CampaignGame::from_save(&old).unwrap();
    assert_eq!(loaded.state().tutorial.mode(), Mode::Off);
    let step = loaded.resume();
    let said: String = step
        .events
        .iter()
        .map(|event| render_event(event, &catalog(Lang::En)))
        .collect();
    assert!(hints(&said).is_empty(), "{said}");
    for line in ["help", "status", "quests"] {
        let after = loaded.handle(Input::Line(line.to_owned()));
        assert!(
            after
                .events
                .iter()
                .all(|event| !render_event(event, &catalog(Lang::En)).starts_with("Tutorial")),
            "`{line}`"
        );
    }
    assert_eq!(loaded.snapshot().unwrap().matches("tutorial").count(), 0);
}

// -------------------------------------------------------------------------- hostile saves

#[test]
fn a_tutorial_no_play_can_reach_is_refused_with_the_reason() {
    let mut driver = Driver::guided();
    driver.line("help");
    let text = driver.game.snapshot().unwrap();
    assert!(text.contains("done = [\"help\"]"), "{text}");
    assert!(text.contains("shown = \"status\""), "{text}");
    let cases: &[(&str, &str, &str)] = &[
        (
            "done = [\"help\"]",
            "done = [\"nope\"]",
            "not a tutorial step",
        ),
        (
            "done = [\"help\"]",
            "done = [\"quit\"]",
            "not a tutorial step",
        ),
        ("done = [\"help\"]", "done = [1]", ""),
        (
            "shown = \"status\"",
            "shown = \"nope\"",
            "not a tutorial step",
        ),
        ("shown = \"status\"", "shown = 3", ""),
        (
            "mode = \"active\"",
            "mode = \"finished\"",
            "finished before",
        ),
        ("mode = \"active\"", "mode = \"off\"", "keeps no progress"),
        ("mode = \"active\"", "mode = \"paused\"", ""),
        ("mode = \"active\"", "mode = 1", ""),
    ];
    for (from, to, expected) in cases {
        let hostile = replace_once(&text, from, to);
        match CampaignGame::from_save(&hostile) {
            Ok(_) => panic!("accepted `{to}`"),
            Err(error) => assert!(
                error.to_string().to_lowercase().contains(expected),
                "`{to}`: {error}"
            ),
        }
    }
}

#[test]
fn the_tutorial_cannot_have_begun_before_the_opening_is_over() {
    let tutorial = "[state.tutorial]\nmode = \"active\"\n\n[[state.flows]]";
    // At the handle, in the prologue, at the offer.
    let mut stages = vec![Driver::raw().game.snapshot().unwrap()];
    let mut driver = Driver::raw();
    for input in [
        Input::Line("Neon".to_owned()),
        Input::Continue,
        Input::Continue,
    ] {
        driver.send(input);
        stages.push(driver.game.snapshot().unwrap());
    }
    for text in stages {
        let hostile = replace_once(&text, "[[state.flows]]", tutorial);
        let error = CampaignGame::from_save(&hostile).unwrap_err().to_string();
        assert!(error.contains("before the opening"), "{error}");
    }
}

#[test]
fn a_page_of_the_prologue_that_does_not_exist_is_refused() {
    let mut driver = Driver::raw();
    driver.line("Neon");
    let text = driver.game.snapshot().unwrap();
    assert!(text.contains("kind = \"prologue\""), "{text}");
    assert!(text.contains("page = 1"), "{text}");
    for page in ["0", "3", "255", "-1", "256"] {
        let hostile = replace_once(&text, "page = 1", &format!("page = {page}"));
        assert!(CampaignGame::from_save(&hostile).is_err(), "page {page}");
    }
    // Stacked with another menu, or twice.
    let twice = replace_once(
        &text,
        "[end]",
        "[[state.flows]]\nkind = \"offer_tutorial\"\n\n[end]",
    );
    assert!(CampaignGame::from_save(&twice).is_err());
    // The handle is asked once.
    let asked = replace_once(
        &text,
        "kind = \"prologue\"\npage = 1",
        "kind = \"ask_handle\"",
    );
    assert!(CampaignGame::from_save(&asked).is_err());
}

// ------------------------------------------------------------------------------------ properties

const LINES: &[&str] = &[
    "help",
    "status",
    "quests",
    "quests m01",
    "talk echo7",
    "talk",
    "net",
    "net localhost",
    "hack",
    "hack localhost",
    "laylow",
    "shop",
    "buy 1",
    "buy",
    "save",
    "save 2",
    "hint",
    "contacts",
    "bogus",
    "0",
    "1",
    "y",
    "n",
    "",
];

const TUTORIAL_LINES: &[&str] = &[
    "tutorial",
    "tutorial skip",
    "tutorial restart",
    "tutorial x",
];

fn lines() -> impl Strategy<Value = Input> {
    prop_oneof![
        8 => prop::sample::select(LINES).prop_map(|line| Input::Line(line.to_owned())),
        2 => any::<bool>().prop_map(Input::Confirm),
        1 => Just(Input::Cancel),
    ]
}

fn with_tutorial_commands() -> impl Strategy<Value = Input> {
    prop_oneof![
        8 => lines(),
        2 => prop::sample::select(TUTORIAL_LINES)
            .prop_map(|line| Input::Line(line.to_owned())),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Without `tutorial skip`, a running tutorial only goes forward: the steps done are never
    /// lost, it never turns off, and once finished it stays finished. Every state it reaches
    /// is one the loader accepts, and saves back as it was.
    #[test]
    fn the_tutorial_only_moves_forward_and_saves_back_as_it_was(
        inputs in prop::collection::vec(lines(), 0..60),
    ) {
        let mut driver = Driver::guided();
        let mut before = driver.game.state.tutorial.clone();
        for input in inputs {
            if driver.last.prompt == Prompt::End {
                break;
            }
            driver.send(input);
            let now = driver.game.state.tutorial.clone();
            prop_assert!(now.validate().is_ok(), "{now:?}");
            match (before.mode(), now.mode()) {
                (Mode::Active, Mode::Active) => prop_assert!(before.done().is_subset(now.done())),
                (Mode::Active | Mode::Finished, Mode::Finished) => {}
                (from, to) => prop_assert!(false, "{from:?} to {to:?}"),
            }
            let text = driver.game.snapshot().unwrap();
            let loaded = CampaignGame::from_save(&text).unwrap();
            prop_assert_eq!(loaded.snapshot().unwrap(), text);
            before = now;
        }
    }

    /// With the command that skips and restarts, whatever happens: the same lines give the same
    /// game, the tutorial is valid, a hint is said at most once per command, and never inside a
    /// menu.
    #[test]
    fn skipping_and_restarting_keep_the_game_valid_and_deterministic(
        inputs in prop::collection::vec(with_tutorial_commands(), 0..60),
    ) {
        let (mut a, mut b) = (Driver::guided(), Driver::guided());
        for input in inputs {
            if a.last.prompt == Prompt::End {
                break;
            }
            let (first, second) = (a.game.handle(input.clone()), b.game.handle(input));
            prop_assert_eq!(&first, &second);
            a.last = first.clone();
            let said: String = first
                .events
                .iter()
                .map(|event| render_event(event, &catalog(Lang::En)))
                .collect();
            // Once per command; and never while a menu or a question is open.
            prop_assert!(said.matches("Tutorial, step").count() <= 1, "{said}");
            if first.prompt != Prompt::Command {
                prop_assert!(!said.contains("Tutorial, step"), "{said}");
            }
            prop_assert!(a.game.state.tutorial.validate().is_ok());
        }
        prop_assert_eq!(a.game.snapshot().unwrap(), b.game.snapshot().unwrap());
    }
}

// ------------------------------------------------------------------------------------ the texts

/// Every key of the opening and the tutorial that the code can ask for.
fn expected_keys() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let sources = [
        include_str!("../prologue.rs"),
        include_str!("../tutorial.rs"),
        include_str!("../dispatch.rs"),
    ];
    for source in sources {
        let code: String = source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut rest = code.as_str();
        while let Some(start) = rest.find('"') {
            let tail = &rest[start + 1..];
            let Some(end) = tail.find('"') else { break };
            let word = &tail[..end];
            let is_key = (word.starts_with("prologue.") || word.starts_with("tutorial."))
                && word.bytes().all(|b| {
                    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.'
                });
            if is_key && !word.ends_with('.') {
                keys.insert(word.to_owned());
            }
            rest = &tail[end + 1..];
        }
    }
    for step in STEPS {
        for part in ["say", "do"] {
            keys.insert(format!("tutorial.step.{}.{part}", step.id));
        }
    }
    // The command table names its help, and the usage follows it.
    keys.insert("tutorial.help".to_owned());
    keys.insert("tutorial.help.usage".to_owned());
    // The pages and the lines of ECHO-7 are numbered.
    for key in [
        "prologue.p1.1",
        "prologue.p1.2",
        "prologue.p2.1",
        "prologue.p2.2",
    ] {
        keys.insert(key.to_owned());
    }
    for n in 1..=3 {
        keys.insert(format!("prologue.p3.{n}"));
    }
    keys
}

#[test]
fn every_text_of_the_opening_and_the_tutorial_exists_in_both_languages_and_none_is_dead() {
    let expected = expected_keys();
    // Sanity: the scan found the opening and the steps.
    assert!(expected.contains("tutorial.offer") && expected.contains("prologue.p3.channel"));
    assert!(expected.len() > 30, "{expected:?}");
    for lang in Lang::ALL {
        let catalog = catalog(lang);
        let missing: Vec<&String> = expected
            .iter()
            .filter(|key| catalog.get(key).is_none())
            .collect();
        assert!(missing.is_empty(), "{lang:?} lacks {missing:?}");
        let dead: Vec<&str> = catalog
            .iter()
            .map(|(key, _)| key)
            .filter(|key| key.starts_with("prologue.") || key.starts_with("tutorial."))
            .filter(|key| !expected.contains(key.split('@').next().unwrap_or(key)))
            .collect();
        assert!(
            dead.is_empty(),
            "{lang:?} has texts no code asks for: {dead:?}"
        );
    }
}

#[test]
fn the_opening_and_the_tutorial_stay_within_their_width_budgets() {
    let mut checked = 0;
    for lang in Lang::ALL {
        let catalog = catalog(lang);
        for (key, _) in catalog.iter() {
            if !(key.starts_with("prologue.") || key.starts_with("tutorial.")) {
                continue;
            }
            let base = key.split('@').next().unwrap_or(key);
            let Some(max) = budget(base) else {
                panic!("{key} has no width budget");
            };
            let width = display_width(&shown(key, &catalog));
            assert!(width <= max, "[{lang:?}] {key}: {width} columns for {max}");
            checked += 1;
        }
    }
    assert!(checked > 60, "{checked}");
}

#[test]
fn the_interface_lines_are_neutral_and_only_the_characters_say_tu_or_you() {
    // French: no second person in the narration, the instructions, the questions and the
    // answers. ECHO-7's lines (`say`, and the three of the contact) are the exception.
    let second_person = [
        "tu", "te", "t", "toi", "ton", "ta", "tes", "vous", "votre", "vos",
    ];
    let english = ["you", "your", "yours", "you're", "yourself"];
    for lang in Lang::ALL {
        let catalog = catalog(lang);
        let words = if lang == Lang::Fr {
            &second_person[..]
        } else {
            &english[..]
        };
        for (key, _) in catalog.iter() {
            let in_character = key.rsplit('.').next() == Some("say")
                || (key.starts_with("prologue.p3.") && !key.starts_with("prologue.p3.channel"));
            if in_character || !(key.starts_with("prologue.") || key.starts_with("tutorial.")) {
                continue;
            }
            let text = shown(key, &catalog).to_lowercase();
            for word in text.split(|c: char| !(c.is_alphanumeric() || c == '\'')) {
                // "you're" is one word in English; an elision "t'" is two in French.
                let parts: Vec<&str> = if lang == Lang::Fr {
                    word.split('\'').collect()
                } else {
                    vec![word]
                };
                for part in parts {
                    assert!(
                        !words.contains(&part),
                        "[{lang:?}] {key} addresses the player (`{part}`): {text}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_channel_line_has_a_screen_reader_form_without_the_symbol() {
    for lang in Lang::ALL {
        let catalog = catalog(lang);
        let key = Text::new("prologue.p3.channel");
        let plain = render(&key, &catalog, RenderMode::FULL);
        let spoken = render(
            &key,
            &catalog,
            RenderMode {
                screen_reader: true,
                ascii: false,
            },
        );
        assert!(plain.contains('#'), "{plain}");
        assert!(!spoken.contains('#') && spoken != plain, "{spoken}");
    }
}

#[test]
fn the_opening_reads_in_every_language_without_a_missing_text() {
    let mut driver = Driver::raw();
    let mut said = String::new();
    for input in [
        Input::Line("Neon".to_owned()),
        Input::Continue,
        Input::Continue,
        Input::Confirm(true),
    ] {
        driver.send(input);
        for lang in Lang::ALL {
            said.push_str(&driver.rendered(lang));
            let prompt = render_prompt(&driver.last.prompt, &catalog(lang));
            assert!(!prompt.contains("<missing"), "{prompt}");
        }
    }
    for (line, _) in WALK {
        driver.line(line);
        for lang in Lang::ALL {
            said.push_str(&driver.rendered(lang));
        }
    }
    assert!(
        !said.contains("<missing:") && !said.contains("<?"),
        "{said}"
    );
    assert!(
        said.contains("Étape") || said.contains("étape"),
        "French was read"
    );
}
