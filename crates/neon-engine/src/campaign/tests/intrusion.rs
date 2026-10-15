//! The intrusion played inside the campaign: the context it opens, what a save keeps of it,
//! and how its end settles the hub.

use super::support::*;
use crate::campaign::{CampaignGame, Difficulty};
use crate::content::schema::QuestStatus;
use crate::game::Game;
use crate::prompt::{Input, Prompt};
use crate::run::solve::{Goal, PROOF_EXPANSIONS, solve};
use crate::save::SaveRequest;

/// The best plan on `localhost`, typed: it wins with both files.
const LOCALHOST_PLAN: [&str; 10] = [
    "breach 2 backdoor",
    "breach 2 backdoor",
    "move 2",
    "end",
    "breach 3 backdoor",
    "breach 3 backdoor",
    "end",
    "move 3",
    "loot",
    "jackout",
];

/// A campaign at the command prompt whose intrusions are left to the test.
fn manual(difficulty: Difficulty) -> Driver {
    let mut driver = Driver::raw_with(difficulty);
    driver.opening(false);
    driver.auto = false;
    driver
}

/// A campaign inside the intrusion on `localhost`.
fn inside(difficulty: Difficulty) -> Driver {
    let mut driver = manual(difficulty);
    driver.lines(&["hack localhost", "y"]);
    assert!(driver.game.intrusion().is_some());
    driver
}

/// Ends turns until the Trace burns the intrusion.
fn burn(driver: &mut Driver) {
    for _ in 0..400 {
        if driver.game.intrusion().is_none() {
            return;
        }
        driver.line("end");
    }
    panic!("the intrusion never burned");
}

#[test]
fn hack_opens_the_intrusion_after_a_checkpoint_and_the_hub_is_closed_while_it_lasts() {
    let mut driver = manual(Difficulty::Normal);
    driver.saves.clear();
    driver.lines(&["hack localhost", "y"]);
    assert!(driver.game.intrusion().is_some());
    assert_eq!(
        driver.saves,
        [SaveRequest::Autosave, SaveRequest::Checkpoint]
    );
    assert!(driver.game.state().run.is_some());
    // The hub's commands are not there: the intrusion's own are.
    let out = driver.line("shop");
    assert!(driver.errored(), "{out}");
    let out = driver.line("hack corp-server-01");
    assert!(driver.errored(), "{out}");
    let out = driver.line("map");
    assert!(!driver.errored(), "{out}");
    assert!(driver.line("status").contains("Trace"));
}

#[test]
fn leaving_the_intrusion_by_quitting_keeps_the_turn_for_the_next_launch() {
    let mut driver = inside(Difficulty::Normal);
    driver.lines(&["breach 2 backdoor", "quit"]);
    assert!(matches!(driver.last.prompt, Prompt::Confirm { .. }));
    driver.saves.clear();
    driver.send(Input::Confirm(true));
    assert_eq!(driver.last.prompt, Prompt::End);
    assert_eq!(driver.saves, [SaveRequest::Autosave]);
    let text = driver.game.snapshot().unwrap();
    let loaded = CampaignGame::from_save(&text).unwrap();
    assert!(loaded.intrusion().is_some());
}

#[test]
fn a_save_in_the_middle_of_a_turn_resumes_at_the_start_of_that_turn() {
    let mut driver = inside(Difficulty::Normal);
    driver.lines(&["breach 2 backdoor", "breach 2 backdoor", "move 2", "end"]);
    let at_turn_start = driver.game.snapshot().unwrap();
    driver.line("breach 3 backdoor");
    // The save follows the start of the turn, not the actions taken since.
    assert_eq!(driver.game.snapshot().unwrap(), at_turn_start);
    let mut loaded = CampaignGame::from_save(&at_turn_start).unwrap();
    // The actions of the turn that was lost are played again; the rest is the same.
    loaded.handle(Input::Line("breach 3 backdoor".to_owned()));
    // Both play the rest of the plan to the same end.
    for line in &LOCALHOST_PLAN[5..] {
        let input = |game: &CampaignGame| match game.prompt() {
            Prompt::Confirm { .. } => Input::Confirm(true),
            _ => Input::Line((*line).to_owned()),
        };
        let a = loaded.handle(input(&loaded));
        let b = driver.game.handle(input(&driver.game));
        assert_eq!(a, b, "`{line}`");
    }
    assert!(loaded.intrusion().is_none());
    assert_eq!(
        loaded.state().base_notoriety,
        driver.game.state().base_notoriety
    );
}

#[test]
fn a_won_intrusion_settles_the_hub_like_the_automatic_one_did() {
    let mut driver = inside(Difficulty::Normal);
    driver.lines(&LOCALHOST_PLAN);
    assert!(driver.game.intrusion().is_none());
    assert!(driver.game.state().run.is_none());
    let text = driver.text();
    assert!(text.contains("Run report"), "{text}");
    assert!(driver.game.state().base_notoriety >= 1);
    assert!(!driver.game.state().missions.extracted.is_empty());
    // The hub is back.
    assert!(!driver.line("shop").is_empty());
}

#[test]
fn a_burned_intrusion_takes_nothing_and_costs_the_whole_trace_in_notoriety() {
    let mut driver = inside(Difficulty::Normal);
    burn(&mut driver);
    let text = driver.text();
    assert!(text.contains("hack localhost"), "{text}");
    assert_eq!(driver.game.state().base_notoriety, 25);
    assert!(driver.game.state().missions.extracted.is_empty());
    assert!(!driver.game.state().lost);
    assert_eq!(driver.last.prompt, Prompt::Command);
    // And it can be tried again.
    driver.lines(&["hack localhost", "y"]);
    assert!(driver.game.intrusion().is_some());
}

#[test]
fn a_burned_intrusion_loses_a_hardcore_game_and_the_loss_survives_a_save() {
    let mut driver = inside(Difficulty::Hardcore);
    burn(&mut driver);
    assert!(driver.game.state().lost);
    assert_eq!(driver.last.prompt, Prompt::End);
    assert!(
        driver.text().contains("the game is lost"),
        "{}",
        driver.text()
    );
    let text = driver.game.snapshot().unwrap();
    let loaded = CampaignGame::from_save(&text).unwrap();
    assert_eq!(loaded.prompt(), Prompt::End);
    let step = loaded.resume();
    assert_eq!(step.prompt, Prompt::End);
    // The checkpoint taken when the intrusion began is the way back.
    assert!(driver.saves.contains(&SaveRequest::Checkpoint));
}

#[test]
fn skip_plays_the_intrusion_in_story_difficulty_only() {
    let mut driver = inside(Difficulty::Normal);
    let out = driver.line("skip");
    assert!(driver.errored(), "{out}");
    assert!(driver.game.intrusion().is_some());

    let mut story = inside(Difficulty::Story);
    story.line("skip");
    assert!(matches!(story.last.prompt, Prompt::Confirm { .. }));
    story.send(Input::Confirm(true));
    assert!(story.game.intrusion().is_none());
    let text = story.text();
    assert!(text.contains("Automatic"), "{text}");
    assert!(story.game.state().base_notoriety >= 1);
}

#[test]
fn the_intrusion_counts_for_the_quest_that_asks_for_it() {
    let mut driver = manual(Difficulty::Normal);
    driver.lines(&["talk echo7", "0"]);
    driver.lines(&["hack localhost", "y"]);
    driver.lines(&LOCALHOST_PLAN);
    driver.line("laylow");
    assert_eq!(driver.status("m01"), QuestStatus::Active);
    driver.lines(&["quests", "help", "net", "status"]);
    assert_eq!(driver.status("m01"), QuestStatus::Completed);
}

#[test]
fn a_save_whose_intrusion_does_not_fit_its_site_is_refused() {
    let mut driver = inside(Difficulty::Normal);
    driver.line("breach 2 backdoor");
    let text = driver.game.snapshot().unwrap();
    // A turn start has the Trace of its floor, and a state that is not one is no save.
    let broken = text.replacen("trace = 0\nturn_floor = 0", "trace = 5\nturn_floor = 0", 1);
    assert_ne!(broken, text, "{text}");
    assert!(CampaignGame::from_save(&broken).is_err());
    let no_site = text.replacen("site = \"localhost\"", "site = \"nowhere\"", 1);
    assert!(CampaignGame::from_save(&no_site).is_err());
}

#[test]
fn every_site_of_the_content_has_a_plan_the_driver_can_follow() {
    let driver = manual(Difficulty::Normal);
    let content = driver.game.content;
    for run in &content.runs {
        let intrusion = crate::intrusion::Intrusion::new(
            content,
            &run.site,
            Difficulty::Normal,
            crate::content::Stock::Everything,
        )
        .expect("an intrusion for every site");
        let solved = solve(
            intrusion.run().mission(),
            intrusion.state(),
            Goal::Feasible,
            PROOF_EXPANSIONS,
            u8::MAX,
        );
        assert!(solved.plan().is_some(), "{}", run.site);
    }
}
