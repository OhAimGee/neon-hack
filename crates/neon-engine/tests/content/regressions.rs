//! Regression tests of the review of the first version of the language prototype.

use crate::common::{Game, id, q, shipped};
use crate::optimist::{Plan, play_from};
use neon_engine::content::engine::Outcome;
use neon_engine::content::schema::{FlagValue, QuestStatus};
use neon_engine::content::{Content, Credits, Diagnostic, Fact, File, Sources};

fn broken(edit: impl FnOnce(&mut Sources)) -> Vec<Diagnostic> {
    let mut src = Sources::embedded();
    edit(&mut src);
    match Content::from_sources(&src) {
        Ok(_) => panic!("the broken copy was accepted"),
        Err(e) => e.0,
    }
}

fn has(errors: &[Diagnostic], file: File, needle: &str) -> bool {
    errors
        .iter()
        .any(|d| d.file == file && d.message.contains(needle))
}

/// 1. Two payments queued in the same round are checked against the same balance.
#[test]
fn the_optimistic_player_cannot_overspend_within_a_round() {
    let mut g = Game::chapter4(shipped());
    g.s.flags
        .insert(id("angel_state"), FlagValue::Str("free".into()));
    g.send(&[]);
    g.send(&[
        Fact::QuestAccepted { quest: q("s08") },
        Fact::QuestAccepted { quest: q("s13") },
    ]);
    assert_eq!(g.status("s08"), QuestStatus::Active);
    assert_eq!(g.status("s13"), QuestStatus::Active);
    // 1000 credits: S08 asks 400, S13 asks 800; both fit alone, not together.
    let have = i64::from(g.s.credits(&g.c).get());
    let earned = i64::from(g.s.earned.get()) + 1000 - have;
    g.s.earned = Credits::new(u32::try_from(earned).unwrap());
    assert_eq!(g.s.credits(&g.c), Credits::new(1000));
    let report = play_from(
        &g.c,
        &Plan {
            optional: true,
            ..Plan::default()
        },
        g.s.clone(),
        Vec::new(),
    );
    let first_round: u32 = report.log[0]
        .iter()
        .filter_map(|f| {
            if let Fact::Paid { amount, .. } = f {
                Some(amount.get())
            } else {
                None
            }
        })
        .sum();
    assert!(
        first_round <= 1000,
        "the first round pays {first_round} with 1000 credits"
    );
    assert!(
        !report.overdrawn,
        "the player spent more than the {} credits earned",
        report.state.earned
    );
}

/// 2. A quest cannot be locked by itself, nor by a quest that waits for it.
#[test]
fn unlock_quest_edges_are_checked_like_prerequisites() {
    let errors = broken(|s| {
        s.quests = s.quests.replacen(
            "tier = 1\nreward",
            "tier = 1\nunlock = { quest = \"m01\" }\nreward",
            1,
        );
    });
    assert!(has(&errors, File::Quests, "locked by itself"));
    let errors = broken(|s| {
        s.quests = s.quests.replacen(
            "tier = 1\nreward",
            "tier = 1\nunlock = { quest = \"m02\" }\nreward",
            1,
        );
    });
    assert!(has(&errors, File::Quests, "written after it"));
    assert!(has(&errors, File::Quests, "prerequisite cycle"));
}

/// 3. Two documents that yield each other have no source.
#[test]
fn documents_that_only_yield_each_other_have_no_source() {
    let errors = broken(|s| {
        s.catalog.push_str(
            "\n[[readable]]\nid = \"doc_a\"\nkind = \"document\"\nyields = \"doc_b\"\n\
             [[readable]]\nid = \"doc_b\"\nkind = \"document\"\nyields = \"doc_a\"\n",
        );
    });
    assert!(has(
        &errors,
        File::Catalog,
        "readable `doc_a` has no source"
    ));
    assert!(has(
        &errors,
        File::Catalog,
        "readable `doc_b` has no source"
    ));
}

/// 4. A restart reopens the quest at its own turn, not at the end of the batch.
#[test]
fn a_restart_reopens_at_its_own_turn() {
    let mut g = Game::chapter4(shipped());
    assert_eq!(g.status("m09"), QuestStatus::Active);
    g.tick();
    // One batch: the restart of turn 10, then a breach of turn 11.
    g.send(&[
        Fact::QuestRestarted {
            quest: q("m09"),
            at: 10,
        },
        Fact::SiteCompromised {
            site: id("freeport-01"),
            at: 11,
        },
    ]);
    let run = g.s.quests.get(&q("m09")).unwrap();
    assert_eq!(run.opened_at, 10);
    assert!(
        run.done.contains(&(0, 0)),
        "the breach of turn 11 counts after the restart of turn 10"
    );
}

/// 5. A reward plus a `grant` effect of the same quest pay under distinct keys.
#[test]
fn every_reward_output_has_its_own_key() {
    let mut src = Sources::embedded();
    src.quests = src.quests.replacen(
        "    { kind = \"unlock\", readable = \"mail-welcome\" },",
        "    { kind = \"unlock\", readable = \"mail-welcome\" },\n    { kind = \"grant\", credits = \"S\" },\n    { kind = \"grant\", reputation = \"S\" },",
        1,
    );
    let c = Content::from_sources(&src).unwrap_or_else(|e| panic!("{e}"));
    let mut g = Game::with(c);
    for cmd in ["quests", "help", "scan", "status", "laylow"] {
        g.send(&[Fact::CommandUsed { command: id(cmd) }]);
    }
    g.talk("echo7", 1);
    g.compromise("localhost");
    g.tick();
    assert_eq!(g.status("m01"), QuestStatus::Completed);
    let keys: Vec<&String> = g
        .log
        .iter()
        .filter_map(|o| {
            if let Outcome::Reward { key, .. } = o {
                Some(key)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(keys.len(), 3, "the reward and the two grants: {keys:?}");
    let mut sorted = keys.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 3, "keys must be unique: {keys:?}");
    assert!(g.s.claimed.contains("quest-m01"));
}

/// 6. An acceptance cannot be stored before the quest is offered.
#[test]
fn a_quest_cannot_be_accepted_before_it_is_offered() {
    let mut g = Game::chapter4(shipped());
    assert_eq!(g.status("s07"), QuestStatus::Unavailable);
    g.send(&[Fact::QuestAccepted { quest: q("s07") }]);
    assert!(g.s.accepted.is_empty());
    // Phoenix is then saved: S07 is offered, and must wait for a real acceptance.
    g.s.granted.insert(id("f15"));
    g.send(&[
        Fact::Read { id: id("f15") },
        Fact::DecisionMade {
            decision: id("d2"),
            choice: id("save"),
        },
    ]);
    assert_eq!(g.status("s07"), QuestStatus::Available);
    g.tick();
    assert_eq!(g.status("s07"), QuestStatus::Available);
}

/// 7. `foo-bar` and `foo_bar` would share a catalog key.
#[test]
fn ids_that_normalize_to_the_same_text_key_are_refused() {
    let errors = broken(|s| {
        s.catalog
            .push_str("\n[[item]]\nid = \"stealth-module\"\nlegacy_name = \"Twin\"\nprice = 1\n");
    });
    assert!(has(
        &errors,
        File::Texts,
        "`item.stealth_module.name` is derived from both"
    ));
}
