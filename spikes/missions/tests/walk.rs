//! The optimistic player: no dead end, whatever the decisions.

use std::collections::BTreeSet;

use neon_spike_missions::engine::{Output, montage};
use neon_spike_missions::ids::{ContactId, DecisionId, QuestId};
use neon_spike_missions::optimist::{Plan, all_plans, play};
use neon_spike_missions::schema::{QuestKind, QuestStatus};
use neon_spike_missions::{Content, Sources};

fn shipped() -> Content {
    Content::embedded().unwrap_or_else(|e| panic!("{e}"))
}

fn q(id: &str) -> QuestId {
    QuestId::new(id)
}

fn plan(c: &Content, choices: &[(&str, &str)], optional: bool) -> Plan {
    let _ = c;
    Plan {
        choices: choices
            .iter()
            .map(|(d, ch)| (DecisionId::new(*d), (*ch).into()))
            .collect(),
        optional,
        skip: BTreeSet::new(),
    }
}

#[test]
fn the_main_line_can_always_be_finished() {
    let c = shipped();
    let plans = all_plans(&c);
    assert_eq!(
        plans.len(),
        3 * 3 * 4 * 3 * 2,
        "d1 x d2 x d3 x echo_trust x optional"
    );
    for p in &plans {
        let r = play(&c, p);
        assert!(
            r.stuck.is_empty(),
            "stuck {:?} with {:?} after {} rounds",
            r.stuck,
            p.choices,
            r.rounds
        );
        for quest in c.quests.iter().filter(|x| x.kind == QuestKind::Main) {
            assert_eq!(
                r.state.status(&quest.id),
                QuestStatus::Completed,
                "{} with {:?}",
                quest.id,
                p.choices
            );
        }
        assert!(!r.outputs.contains(&Output::BudgetExceeded));
    }
}

#[test]
fn every_quest_ends_up_done_or_legitimately_unavailable() {
    let c = shipped();
    for p in &all_plans(&c) {
        let r = play(&c, p);
        for quest in &c.quests {
            let st = r.state.status(&quest.id);
            if st == QuestStatus::Unavailable {
                // Only the branch of a decision not taken, or a contact lost on the way.
                let why_not = match quest.id.as_str() {
                    "s07" => p
                        .choices
                        .get(&DecisionId::new("d2"))
                        .is_some_and(|d| d.as_str() != "save"),
                    "s13" => !p.optional,
                    "s08" => p
                        .choices
                        .get(&DecisionId::new("d1"))
                        .is_some_and(|d| d.as_str() == "betray"),
                    _ => false,
                };
                assert!(why_not, "{} unavailable with {:?}", quest.id, p.choices);
            } else {
                assert!(
                    matches!(st, QuestStatus::Completed | QuestStatus::Failed),
                    "{} is {st:?}",
                    quest.id
                );
            }
        }
    }
}

#[test]
fn every_decision_leads_to_exactly_one_ending_and_each_ending_is_reached() {
    let c = shipped();
    let mut reached = BTreeSet::new();
    for p in &all_plans(&c) {
        let r = play(&c, p);
        let ending = r
            .state
            .flag(&c, &"ending".into())
            .map(|v| format!("{v:?}"))
            .unwrap_or_default();
        assert!(!ending.contains("none"), "no ending with {:?}", p.choices);
        let d3 = p
            .choices
            .get(&DecisionId::new("d3"))
            .map(ToString::to_string)
            .unwrap_or_default();
        let expected = match d3.as_str() {
            "liberate" => {
                let echo = p
                    .choices
                    .get(&DecisionId::new("echo"))
                    .map(ToString::to_string)
                    .unwrap_or_default();
                // echo_log (S16) is always earned by the optimistic player.
                if echo == "doubt" { "e1b" } else { "e1a" }
            }
            "entrust" => "e2",
            "burn" => "e3",
            _ => "e4",
        };
        assert!(
            ending.contains(expected),
            "{ending} instead of {expected} with {:?}",
            p.choices
        );
        reached.insert(expected);
        // The montage has exactly one line for each contact.
        let lines = montage(&c, &r.state);
        for contact in &c.contacts {
            let n = lines.iter().filter(|l| l.contact == contact.id).count();
            assert_eq!(
                n, 1,
                "{} has {n} epilogue lines with {:?}",
                contact.id, p.choices
            );
        }
    }
    assert_eq!(reached, BTreeSet::from(["e1a", "e1b", "e2", "e3", "e4"]));
}

#[test]
fn without_the_s16_advantage_liberation_ends_in_e1b() {
    let c = shipped();
    let mut p = plan(&c, &[("d3", "liberate"), ("echo", "believe")], true);
    p.skip.insert(q("s16"));
    let r = play(&c, &p);
    assert!(format!("{:?}", r.state.flag(&c, &"ending".into())).contains("e1b"));
}

#[test]
fn a_skipped_optional_objective_silences_neon_angel_and_closes_s13() {
    let c = shipped();
    let r = play(&c, &plan(&c, &[], false));
    assert!(format!("{:?}", r.state.flag(&c, &"angel_state".into())).contains("silenced"));
    assert_eq!(r.state.status(&q("s13")), QuestStatus::Unavailable);
    let r = play(&c, &plan(&c, &[], true));
    assert_eq!(r.state.status(&q("s13")), QuestStatus::Completed);
}

#[test]
fn betraying_r4z0r_closes_her_raid_quest_and_makes_her_hostile() {
    let c = shipped();
    let r = play(&c, &plan(&c, &[("d1", "betray")], true));
    assert_eq!(
        r.state.status(&q("s08")),
        QuestStatus::Completed,
        "S08 was done before M10"
    );
    assert!(format!("{:?}", r.state.contact(&ContactId::new("r4z0r"))).contains("Hostile"));
}

#[test]
fn rewards_are_paid_once_each() {
    let c = shipped();
    let r = play(
        &c,
        &plan(&c, &[("d1", "steal"), ("d2", "turn"), ("d3", "sign")], true),
    );
    let keys: Vec<&String> = r
        .outputs
        .iter()
        .filter_map(|o| {
            if let Output::Reward { key, .. } = o {
                Some(key)
            } else {
                None
            }
        })
        .collect();
    let unique: BTreeSet<&&String> = keys.iter().collect();
    assert_eq!(keys.len(), unique.len(), "a reward key was paid twice");
}

#[test]
fn the_walker_catches_a_dead_end() {
    // Without the first-breach reputation nothing pays enough for M03's REPUTATION(50).
    let mut src = Sources::embedded();
    src.catalog = src.catalog.replace("first_breach = \"S\"\n", "");
    let c = Content::from_sources(&src).unwrap_or_else(|e| panic!("{e}"));
    let r = play(
        &c,
        &Plan {
            optional: true,
            ..Plan::default()
        },
    );
    assert!(r.stuck.contains(&q("m03")), "stuck: {:?}", r.stuck);
    assert_eq!(r.state.status(&q("m04")), QuestStatus::Unavailable);
}
