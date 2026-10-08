//! Saving and loading the campaign: the envelope, the state it holds, and what it refuses.

use super::support::*;
use crate::campaign::CampaignGame;
use crate::demo::DemoGame;
use crate::game::Game;
use crate::prompt::{Input, Prompt};
use crate::save::{self, SaveError, SaveRequest};

fn replace_once(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "`{from}` is not in:\n{text}");
    text.replacen(from, to, 1)
}

/// A campaign a few commands in, with something bought and a contact met.
fn played() -> Driver {
    let mut driver = Driver::new();
    driver.finish_m02();
    driver.lines(&["buy ghost", "y", "talk r4z0r", "0", "hint"]);
    driver
}

#[test]
fn save_load_save_is_byte_identical_at_every_stage_of_a_play() {
    let mut driver = Driver::raw();
    let mut stages = vec![driver.game.snapshot().unwrap()];
    for line in [
        "Neon",
        "quests",
        "talk echo7",
        "1",
        "hack localhost",
        "y",
        "laylow",
        "hack corp-server-01",
        "y",
        "buy 1",
        "y",
        "hint",
        "shop",
        "buy 2",
    ] {
        driver.line(line);
        stages.push(driver.game.snapshot().unwrap());
    }
    for text in stages {
        let loaded = CampaignGame::from_save(&text).unwrap();
        assert_eq!(loaded.snapshot().unwrap(), text);
    }
}

#[test]
fn the_envelope_tells_the_handle_and_the_clock_without_loading_the_game() {
    let driver = played();
    let text = driver.game.snapshot().unwrap();
    let meta = save::peek(&text).unwrap();
    assert_eq!(meta.player, "Neon");
    assert_eq!(meta.turn, driver.game.clock());
    assert!(text.starts_with("version = 1\n"), "{text}");
    assert!(text.trim_end().ends_with("[end]\nok = true"));
    assert!(text.contains("game = \"campaign\""));
}

#[test]
fn a_loaded_game_continues_exactly_like_the_one_it_was_saved_from() {
    let mut original = played();
    let text = original.game.snapshot().unwrap();
    let mut loaded = CampaignGame::from_save(&text).unwrap();
    assert_eq!(loaded.prompt(), original.game.prompt());
    for line in [
        "status",
        "quests m03",
        "hack underground-market",
        "y",
        "contacts",
        "shop",
    ] {
        let input = |game: &CampaignGame| match game.prompt() {
            Prompt::Confirm { .. } => Input::Confirm(true),
            _ => Input::Line(line.to_owned()),
        };
        let a = original.game.handle(input(&original.game));
        let b = loaded.handle(input(&loaded));
        assert_eq!(a, b, "`{line}`");
    }
}

#[test]
fn a_checkpoint_made_before_a_purchase_comes_back_at_the_question() {
    let mut driver = Driver::new();
    driver.finish_m01();
    driver.line("buy ghost");
    assert_eq!(driver.last.save, Some(SaveRequest::Checkpoint));
    let text = driver.game.snapshot().unwrap();
    let mut loaded = CampaignGame::from_save(&text).unwrap();
    assert!(matches!(
        loaded.prompt(),
        Prompt::Confirm { default: false, .. }
    ));
    assert_eq!(loaded.credits(), driver.credits(), "nothing was spent yet");
    // Saying no leaves everything as it was; saying yes buys.
    let step = loaded.handle(Input::Confirm(true));
    assert!(step.events.iter().any(|event| matches!(
        event,
        crate::event::Event::Message {
            role: crate::event::Role::Reward,
            ..
        }
    )));
    assert_eq!(loaded.credits(), driver.credits() - 80);
}

#[test]
fn a_game_of_another_kind_is_refused_by_both_games() {
    let campaign = played().game.snapshot().unwrap();
    let demo = DemoGame::new(1).snapshot().unwrap();
    assert!(
        matches!(CampaignGame::from_save(&demo), Err(SaveError::Schema(_))),
        "{:?}",
        CampaignGame::from_save(&demo).err()
    );
    assert!(DemoGame::from_save(&campaign).is_err());
    let relabeled = replace_once(&campaign, "game = \"campaign\"", "game = \"demo\"");
    assert!(matches!(
        CampaignGame::from_save(&relabeled),
        Err(SaveError::Invalid(_))
    ));
}

#[test]
fn a_save_from_a_newer_game_or_a_cut_one_is_refused() {
    let text = played().game.snapshot().unwrap();
    let newer = replace_once(&text, "version = 1", "version = 2");
    assert!(matches!(
        CampaignGame::from_save(&newer),
        Err(SaveError::TooNew { .. })
    ));
    let cut = &text[..text.len() / 2];
    assert!(CampaignGame::from_save(cut).is_err());
    assert!(CampaignGame::from_save("").is_err());
    // Every strict prefix is refused (the end marker is the last table).
    let body = text.trim_end();
    for (cut, _) in body.char_indices().step_by(97) {
        assert!(
            CampaignGame::from_save(&body[..cut]).is_err(),
            "prefix {cut} loaded"
        );
    }
}

#[test]
fn a_state_the_game_could_never_reach_is_refused_with_the_reason() {
    let text = played().game.snapshot().unwrap();
    let cases: &[(&str, &str, &str)] = &[
        (
            "base_notoriety = ",
            "base_notoriety = 200\n#",
            "notoriety 200",
        ),
        ("handle = \"Neon\"", "handle = \"\"", "handle"),
        ("handle = \"Neon\"", "handle = \"Neo\\nn\"", "handle"),
        (
            "difficulty = \"normal\"",
            "difficulty = \"nightmare\"",
            "difficulty",
        ),
        ("quest_log = [", "quest_log = [\"m01\", \"m01\", ", "twice"),
        ("quest_log = [", "quest_log = [\"nowhere\", ", "not valid"),
        (
            "inbox = [\"mail-welcome\"]",
            "inbox = [\"f01\"]",
            "not valid",
        ),
        ("tier = 3", "tier = 9", "tier"),
        ("earned = ", "earned = 0\n#", "cost more"),
    ];
    for (from, to, expected) in cases {
        assert!(text.contains(from), "the save has no `{from}`:\n{text}");
        let hostile = replace_once(&text, from, to);
        match CampaignGame::from_save(&hostile) {
            Ok(_) => panic!("accepted `{to}`"),
            Err(error) => assert!(
                error
                    .to_string()
                    .to_lowercase()
                    .contains(&expected.to_lowercase()),
                "`{to}`: {error}"
            ),
        }
    }
}

#[test]
fn hints_beyond_the_budget_and_stacked_menus_are_refused() {
    let mut driver = Driver::new();
    driver.line("hint");
    let text = driver.game.snapshot().unwrap();
    assert!(text.contains("[state.hints]"));
    let greedy = replace_once(&text, "\"m01.1\" = 1", "\"m01.1\" = 9");
    assert!(
        CampaignGame::from_save(&greedy)
            .unwrap_err()
            .to_string()
            .contains("hints used")
    );
    let nowhere = replace_once(&text, "\"m01.1\" = 1", "\"m01.40\" = 1");
    assert!(CampaignGame::from_save(&nowhere).is_err());
    // Menus: a conversation inside a purchase is not a state of the game.
    let mut driver = Driver::new();
    driver.line("talk echo7");
    let talk = driver.game.snapshot().unwrap();
    assert!(talk.contains("kind = \"talk\""), "{talk}");
    let stacked = format!(
        "{}\n[[state.flows]]\nkind = \"confirm_quit\"\n\n[end]\nok = true\n",
        talk.split("\n[end]").next().unwrap()
    );
    assert!(CampaignGame::from_save(&stacked).is_err());
    // A conversation with someone who does not exist.
    let ghost = replace_once(&talk, "contact = \"echo7\"", "contact = \"nobody\"");
    assert!(CampaignGame::from_save(&ghost).is_err());
}

#[test]
fn the_notoriety_floors_of_the_story_survive_a_save() {
    let mut driver = Driver::new();
    driver.finish_m01();
    // A floor told by the story: +20 until M13 opens.
    let mut events = Vec::new();
    driver
        .game
        .state
        .heat_mods
        .push(crate::campaign::state::HeatMod {
            delta: 20,
            until: "m13".parse().unwrap(),
        });
    driver
        .game
        .apply(Vec::new(), driver.game.state.next_turn(), &mut events);
    assert_eq!(
        driver.game.notoriety(),
        driver.game.state.base_notoriety + 20
    );
    let text = driver.game.snapshot().unwrap();
    let loaded = CampaignGame::from_save(&text).unwrap();
    assert_eq!(loaded.notoriety(), driver.game.notoriety());
    assert_eq!(loaded.snapshot().unwrap(), text);
}
