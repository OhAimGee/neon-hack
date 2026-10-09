//! The tests of the whole-campaign walk.
//!
//! The default set plays a representative subset of the 216 plans of the content-level
//! player (every ending, every option of every decision, with and without the optional
//! objectives, at least once). The full set is `#[ignore]`d:
//!
//! ```text
//! cargo test -p neon-engine --test campaign -- --ignored
//! ```

use std::collections::BTreeSet;

use neon_engine::campaign::Difficulty;
use neon_engine::content::Content;
use neon_engine::content::schema::{FlagValue, QuestKind, QuestStatus};

use crate::brain::{Plan, all_plans};
use crate::player::{Player, Reload, Walked, epilogue_contacts, expected_ending};

fn content() -> Content {
    Content::embedded().unwrap()
}

/// What a plan covers: each option of each decision with the optional flag, and its ending.
fn features(plan: &Plan) -> Vec<String> {
    let mut features: Vec<String> = plan
        .choices
        .iter()
        .map(|(decision, choice)| format!("{decision}={choice}/{}", plan.optional))
        .collect();
    features.extend(
        plan.choices
            .iter()
            .map(|(decision, choice)| format!("{decision}={choice}")),
    );
    features.push(format!("ending {}", expected_ending(plan)));
    features.push(format!("optional {}", plan.optional));
    features
}

/// A subset of the plans that covers every feature (a greedy cover): small, and still every
/// ending and every option of every decision at least once, with and without the optional
/// objectives.
fn representative(plans: &[Plan]) -> Vec<Plan> {
    let wanted: BTreeSet<String> = plans.iter().flat_map(features).collect();
    let mut seen = BTreeSet::new();
    let mut chosen = Vec::new();
    while seen.len() < wanted.len() {
        let best = plans
            .iter()
            .max_by_key(|plan| features(plan).iter().filter(|f| !seen.contains(*f)).count())
            .unwrap();
        seen.extend(features(best));
        chosen.push(best.clone());
    }
    chosen
}

/// Everything a finished walk must satisfy, whatever the plan.
fn check(c: &Content, plan: &Plan, walked: &Walked) {
    let name = plan.name();
    let state = walked.game.state().missions();

    // The campaign is finished: every main quest is completed.
    for quest in c.quests.iter().filter(|q| q.kind == QuestKind::Main) {
        assert_eq!(
            state.status(&quest.id),
            QuestStatus::Completed,
            "{name}: {}",
            quest.id
        );
    }
    // Every other quest is done or failed, or was legitimately never offered: the branch of
    // a decision not taken (S07 needs D2 = save, S08 is lost with R4Z0R), or the vigil of the
    // Angel, who is only freed by an optional objective.
    for quest in &c.quests {
        let status = state.status(&quest.id);
        if status == QuestStatus::Unavailable {
            let choice = |decision: &str| {
                plan.choices
                    .iter()
                    .find(|(d, _)| d.as_str() == decision)
                    .map(|(_, c)| c.as_str())
            };
            let why_not = match quest.id.as_str() {
                "s07" => choice("d2") != Some("save"),
                "s08" => choice("d1") == Some("betray"),
                "s13" => !plan.optional,
                _ => false,
            };
            assert!(why_not, "{name}: {} is unavailable", quest.id);
        } else {
            assert!(
                matches!(status, QuestStatus::Completed | QuestStatus::Failed),
                "{name}: {} is {status:?}",
                quest.id
            );
        }
    }

    // Exactly one ending, the one the decisions lead to.
    let expected = expected_ending(plan);
    assert_eq!(
        walked.endings,
        [format!("ending.{expected}.title")],
        "{name}"
    );
    assert_eq!(
        state.flags.get(&"ending".parse().unwrap()),
        Some(&FlagValue::Str(expected.to_owned())),
        "{name}"
    );

    // One epilogue line per contact, none twice.
    let contacts = epilogue_contacts(c);
    let lines: Vec<String> = walked
        .epilogue
        .iter()
        .map(|key| {
            let line = c
                .epilogue
                .iter()
                .find(|l| format!("epilogue.{}", l.id.as_str().replace('-', "_")) == *key)
                .unwrap_or_else(|| panic!("{name}: {key} is not an epilogue line"));
            line.contact.to_string()
        })
        .collect();
    assert_eq!(lines.len(), contacts.len(), "{name}: {:?}", walked.epilogue);
    assert_eq!(
        lines.iter().cloned().collect::<BTreeSet<_>>(),
        contacts,
        "{name}"
    );

    // A quest is announced as completed once, and the credits announced are the credits
    // earned: nothing is paid twice, nothing is paid without being told.
    assert!(
        walked.completed.values().all(|n| *n == 1),
        "{name}: {:?}",
        walked.completed
    );
    assert_eq!(
        i64::from(c.rules.start_credits.get()) + walked.rewarded,
        i64::from(state.earned.get()),
        "{name}: the credits announced are not the credits earned"
    );

    // Every kind of fact the quests ask for has a command that produces it, and the walk
    // used them all: talking, topics, accepting, buying, intrusions (files, marks), reading
    // messages and fragments, decrypting, the neural link, cover services, the commands the
    // first quest asks for, and the decisions and payments of the menus.
    let used: BTreeSet<&str> = walked
        .typed
        .iter()
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    for command in [
        "talk", "accept", "buy", "hack", "read", "archives", "decrypt", "link", "laylow", "help",
        "status", "quests", "net",
    ] {
        assert!(
            used.contains(command),
            "{name}: the walk never used `{command}`"
        );
    }
    for decision in &c.decisions {
        assert!(
            state.decisions.contains_key(&decision.id),
            "{name}: {} was never decided",
            decision.id
        );
    }

    assert!(walked.steps < 1_500, "{name}: {} steps", walked.steps);
    assert!(
        walked.checkpoints >= 8,
        "{name}: {} checkpoints",
        walked.checkpoints
    );
}

fn walk(plan: &Plan, reload: Reload, thorough: bool) -> Walked {
    Player::new(plan, Difficulty::Normal, reload, thorough).play()
}

#[test]
fn the_representative_plans_cover_every_ending_and_every_option() {
    let c = content();
    let plans = all_plans(&c);
    assert_eq!(
        plans.len(),
        3 * 3 * 4 * 3 * 2,
        "d1 x d2 x d3 x echo x optional"
    );
    let chosen = representative(&plans);
    let endings: BTreeSet<&str> = chosen.iter().map(expected_ending).collect();
    assert_eq!(endings, BTreeSet::from(["e1a", "e1b", "e2", "e3", "e4"]));
    for decision in &c.decisions {
        for choice in &decision.choice {
            for optional in [true, false] {
                assert!(
                    chosen.iter().any(|p| p.optional == optional
                        && p.choices.get(&decision.id) == Some(&choice.id)),
                    "{}={} optional={optional} is not covered",
                    decision.id,
                    choice.id
                );
            }
        }
    }
    assert!(chosen.len() <= 12, "{} plans", chosen.len());
}

#[test]
fn the_representative_plans_reach_the_epilogue_and_are_deterministic() {
    let c = content();
    for plan in &representative(&all_plans(&c)) {
        let first = walk(plan, Reload::Never, false);
        check(&c, plan, &first);
        // The same plan twice says exactly the same thing.
        let second = walk(plan, Reload::Never, false);
        assert_eq!(first.hash, second.hash, "{}", plan.name());
        assert_eq!(first.typed, second.typed, "{}", plan.name());
        // Saved and loaded back at every checkpoint and slot, the game plays on unchanged.
        let reloaded = walk(plan, Reload::Checkpoints, false);
        assert!(
            reloaded.reloads >= 8,
            "{}: {} reloads",
            plan.name(),
            reloaded.reloads
        );
        assert_eq!(first.hash, reloaded.hash, "{}", plan.name());
    }
}

#[test]
fn a_game_saved_and_loaded_at_every_save_it_asks_for_plays_on_unchanged() {
    let c = content();
    let plans = representative(&all_plans(&c));
    // Autosaves are taken after almost every command, in the middle of menus too.
    for plan in plans.iter().take(2) {
        let plain = walk(plan, Reload::Never, false);
        let reloaded = walk(plan, Reload::EverySave, false);
        assert!(reloaded.reloads > 100, "{}", reloaded.reloads);
        assert_eq!(plain.hash, reloaded.hash, "{}", plan.name());
    }
}

#[test]
fn every_screen_reads_in_every_mode_without_a_missing_text() {
    let c = content();
    let plans = all_plans(&c);
    let plan = plans.first().unwrap();
    // English and French, in full mode, for screen readers, in ASCII, and both of the last.
    let walked = walk(plan, Reload::Never, true);
    check(&c, plan, &walked);
}

#[test]
fn the_difficulty_does_not_change_the_way_to_the_end() {
    let c = content();
    let plan = all_plans(&c).remove(0);
    for difficulty in Difficulty::ALL {
        let walked = Player::new(&plan, difficulty, Reload::Never, false).play();
        check(&c, &plan, &walked);
    }
}

#[test]
#[ignore = "the full set of 216 plans; run with `cargo test -p neon-engine --test campaign -- --ignored`"]
fn all_216_plans_reach_the_epilogue() {
    let c = content();
    let mut endings = BTreeSet::new();
    for plan in &all_plans(&c) {
        let first = walk(plan, Reload::Never, false);
        check(&c, plan, &first);
        let reloaded = walk(plan, Reload::Checkpoints, false);
        assert_eq!(first.hash, reloaded.hash, "{}", plan.name());
        endings.extend(first.endings);
    }
    assert_eq!(endings.len(), 5);
}
