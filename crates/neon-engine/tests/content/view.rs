//! The reads the hub commands rely on: quest log, contacts, ending, chapter.

use neon_engine::content::eval::ObjectiveState;
use neon_engine::content::schema::{ContactState, FlagValue, QuestStatus};
use neon_engine::content::view::{chapter, contact_list, ending, flag_value, quests_with_status};
use neon_engine::content::{Fact, new_game};

use crate::common::{Game, id, shipped};
use crate::optimist::{all_plans, play};

#[test]
fn a_new_game_shows_the_first_quest_with_all_its_objectives_pending() {
    let c = shipped();
    let (s, _) = new_game(&c);
    let active = quests_with_status(&c, &s, QuestStatus::Active);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].def.id.as_str(), "m01");
    assert_eq!(active[0].objectives.len(), 7);
    assert!(
        active[0]
            .objectives
            .iter()
            .all(|o| o.state == ObjectiveState::Pending)
    );
    assert!(quests_with_status(&c, &s, QuestStatus::Completed).is_empty());
    assert!(quests_with_status(&c, &s, QuestStatus::Available).is_empty());
    assert_eq!(chapter(&c, &s), 1);
    assert!(ending(&c, &s).is_none());
}

#[test]
fn the_counted_objectives_report_how_far_they_got() {
    let mut g = Game::new();
    for cmd in ["quests", "help", "scan", "status", "laylow"] {
        g.send(&[Fact::CommandUsed { command: id(cmd) }]);
    }
    g.talk("echo7", 1);
    g.compromise("localhost");
    g.tick();
    g.compromise("corp-server-01");
    g.tick();
    assert_eq!(g.status("m03"), QuestStatus::Active);
    let progress = |g: &Game| {
        let entry = quests_with_status(&g.c, &g.s, QuestStatus::Active)
            .into_iter()
            .find(|e| e.def.id.as_str() == "m03")
            .unwrap();
        // The compromise objective of M03 asks for three different systems.
        entry.objectives[2]
    };
    assert_eq!(progress(&g).progress, Some((0, 3)));
    g.compromise("localhost");
    assert_eq!(progress(&g).progress, Some((1, 3)));
    g.compromise("corp-server-01");
    g.compromise("nexus-mainframe");
    let done = progress(&g);
    assert_eq!(done.state, ObjectiveState::Done);
    assert_eq!(done.progress, None, "a finished objective shows no counter");
    assert_eq!(chapter(&g.c, &g.s), 2);
}

#[test]
fn contacts_show_their_state_trust_offers_and_open_topics() {
    let g = Game::chapter4(shipped());
    let list = contact_list(&g.c, &g.s);
    assert_eq!(list.len(), g.c.contacts.len());
    let r4z0r = list.iter().find(|e| e.def.id.as_str() == "r4z0r").unwrap();
    assert_eq!(r4z0r.state, ContactState::Available);
    assert!(r4z0r.reachable);
    assert!(
        r4z0r.offers.iter().any(|q| q.id.as_str() == "s08"),
        "R4Z0R offers her raid"
    );
    assert!(r4z0r.topics >= 1);
    let broker = list.iter().find(|e| e.def.id.as_str() == "broker").unwrap();
    assert!(!broker.reachable && broker.offers.is_empty() && broker.topics == 0);
    // The list follows the file order of the catalog.
    let ids: Vec<&str> = list.iter().map(|e| e.def.id.as_str()).collect();
    let catalog: Vec<&str> = g.c.contacts.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, catalog);
}

#[test]
fn the_ending_and_the_chapter_follow_the_campaign() {
    let c = shipped();
    for plan in &all_plans(&c) {
        let state = play(&c, plan).state;
        let reached = ending(&c, &state).expect("the campaign ended");
        assert_eq!(
            flag_value(&c, &state, "ending"),
            Some(FlagValue::Str(reached.id.to_string()))
        );
        assert_eq!(chapter(&c, &state), 6);
        assert_eq!(
            quests_with_status(&c, &state, QuestStatus::Active).len(),
            0,
            "{:?}",
            plan.choices
        );
    }
}
