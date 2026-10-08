//! The semantics of the language, on the quests of the bible.

use crate::common::{Game, id, q};
use crate::optimist::{Plan, play};
use neon_engine::content::engine::{Outcome, available_topics, offered_choices};
use neon_engine::content::eval::{ObjectiveState, journal};
use neon_engine::content::ids::{ContactId, DecisionId};
use neon_engine::content::schema::{ContactState, QuestStatus};
use neon_engine::content::{Content, Credits, Fact, Reputation, Sources};

/// M01 to M03, played by hand: the tutorial, the first infiltration, the information networks.
fn through_m03(g: &mut Game) {
    for cmd in ["quests", "help", "scan", "status", "laylow"] {
        g.send(&[Fact::CommandUsed { command: id(cmd) }]);
    }
    g.talk("echo7", 1);
    g.compromise("localhost");
    g.tick();
    assert_eq!(g.status("m01"), QuestStatus::Completed);
    assert_eq!(g.s.tier, 2);
    g.compromise("corp-server-01");
    g.tick();
    assert_eq!(g.status("m02"), QuestStatus::Completed);
    // M03: talk, buy, three systems, reputation 50, and the secret document.
    g.talk("r4z0r", 1);
    g.send(&[Fact::ItemBought {
        item: id("stealth_module"),
    }]);
    for site in ["localhost", "corp-server-01", "nexus-mainframe"] {
        g.compromise(site);
    }
    assert_eq!(
        g.status("m03"),
        QuestStatus::Active,
        "the secret objective is still missing"
    );
    let view = journal(&g.c, &g.s, &q("m03"));
    assert_eq!(view.get(4).map(|v| v.state), Some(ObjectiveState::Hidden));
    g.send(&[Fact::Decrypted {
        id: id("doc_phase2"),
    }]);
    assert_eq!(g.status("m03"), QuestStatus::Completed);
}

#[test]
fn m01_to_m03_unlock_the_contacts_and_pay_the_symbolic_rewards() {
    let mut g = Game::new();
    through_m03(&mut g);
    for who in ["r4z0r", "phoenix", "miner"] {
        assert_eq!(
            g.s.contact(&id::<ContactId>(who)),
            ContactState::Available,
            "{who}"
        );
    }
    // M01 M@P1 = 80 credits, M02 M@P2 = 150, M03 M@P3 = 200, breaches S: 75 (corp) + 100 (nexus).
    assert_eq!(g.s.earned, Credits::new(100 + 80 + 150 + 200 + 75 + 100));
    assert_eq!(g.s.reputation, Reputation::new(10 + 25 + 40 + 10 + 10));
    assert_eq!(
        g.status("m04"),
        QuestStatus::Active,
        "main quests open by themselves"
    );
    assert_eq!(g.s.tier, 3);
}

#[test]
fn m04_reads_the_state_so_the_past_counts_but_never_completes_it_at_once() {
    let mut g = Game::new();
    // Everything M04 asks for is done before M04 exists (the loot is unique: it cannot be
    // done again), and the heat is fine.
    for (site, file) in [
        ("corp-server-01", "employee_records"),
        ("corp-server-01", "financial_data"),
        ("underground-market", "black_ledger"),
    ] {
        g.extract(site, file);
    }
    g.send(&[Fact::Decrypted {
        id: id("doc_ledger"),
    }]);
    through_m03(&mut g);
    assert_eq!(
        g.status("m04"),
        QuestStatus::Active,
        "opened, not completed, in the refresh of M03"
    );
    assert!(g.log.contains(&Outcome::QuestOpened(q("m04"))));
    g.tick();
    assert_eq!(
        g.status("m04"),
        QuestStatus::Completed,
        "one turn later the stock objectives hold"
    );
    assert_eq!(g.s.tier, 4);
}

#[test]
fn m04_heat_is_a_condition_checked_when_concluding() {
    let mut g = Game::new();
    through_m03(&mut g);
    g.tick();
    g.heat(70);
    for (site, file) in [
        ("corp-server-01", "employee_records"),
        ("corp-server-01", "financial_data"),
        ("underground-market", "black_ledger"),
    ] {
        g.extract(site, file);
    }
    g.send(&[Fact::Decrypted {
        id: id("doc_ledger"),
    }]);
    assert_eq!(
        g.status("m04"),
        QuestStatus::Active,
        "heat 70 blocks the conclusion"
    );
    g.heat(20);
    assert_eq!(
        g.status("m04"),
        QuestStatus::Completed,
        "and cooling down concludes it, no failure"
    );
}

#[test]
fn m05_any_of_buy_and_event_objectives_count_only_after_the_opening() {
    let mut g = Game::new();
    through_m03(&mut g);
    // Nexus was breached during M03: it does not count for M05, which opened after.
    g.send(&[Fact::FileExtracted {
        site: id("nexus-mainframe"),
        file: id("neural_maps"),
    }]);
    for (site, file) in [
        ("corp-server-01", "employee_records"),
        ("corp-server-01", "financial_data"),
        ("underground-market", "black_ledger"),
    ] {
        g.extract(site, file);
    }
    g.send(&[Fact::Decrypted {
        id: id("doc_ledger"),
    }]);
    g.tick();
    assert_eq!(g.status("m05"), QuestStatus::Active);
    g.send(&[Fact::ItemBought {
        item: id("quantum_key"),
    }]);
    g.tick();
    assert_eq!(
        g.status("m05"),
        QuestStatus::Active,
        "one of the three items is enough, but the breach is still to do"
    );
    g.compromise("nexus-mainframe");
    g.tick();
    assert_eq!(g.status("m05"), QuestStatus::Completed);
    assert_eq!(g.s.tier, 4);
    for who in ["aura", "angel"] {
        assert_eq!(g.s.contact(&id::<ContactId>(who)), ContactState::Available);
    }
    assert_eq!(g.s.contact(&id::<ContactId>("broker")), ContactState::Busy);
    // L credits and XL reputation at tier 4: 2 x 500 and +100.
    assert!(g.log.contains(&Outcome::Reward {
        key: "quest-m05".into(),
        credits: Credits::new(1000),
        reputation: Reputation::new(100)
    }));
}

#[test]
fn a_reward_is_paid_once_however_often_the_state_is_refreshed() {
    let mut g = Game::new();
    through_m03(&mut g);
    let earned = g.s.earned;
    for _ in 0..3 {
        assert!(g.send(&[]).is_empty());
        g.tick();
    }
    assert_eq!(g.s.earned, earned, "M04 is not done, nothing more was paid");
    let before = g.s.clone();
    assert!(neon_engine::content::refresh(&g.c, &mut g.s).is_empty());
    assert_eq!(g.s, before);
}

#[test]
fn contracts_are_offered_then_accepted() {
    let mut g = Game::chapter4(shipped());
    assert_eq!(g.status("s08"), QuestStatus::Available, "offered by R4Z0R");
    assert_eq!(
        g.status("m09"),
        QuestStatus::Active,
        "a main quest opens by itself"
    );
    g.tick();
    g.tick();
    assert_eq!(
        g.status("s08"),
        QuestStatus::Available,
        "still waiting for the player"
    );
    g.send(&[Fact::QuestAccepted { quest: q("s08") }]);
    assert_eq!(g.status("s08"), QuestStatus::Active);
}

fn shipped() -> Content {
    crate::common::shipped()
}

#[test]
fn s07_exists_only_for_the_saved_phoenix() {
    let mut g = Game::chapter4(shipped());
    assert_eq!(g.status("s07"), QuestStatus::Unavailable);
    // D2 = save needs the blueprints F15 to be read.
    let d2 = id::<DecisionId>("d2");
    assert!(
        offered_choices(&g.c, &g.s, &d2)
            .iter()
            .all(|c| c.id.as_str() != "save")
    );
    g.send(&[Fact::DecisionMade {
        decision: d2.clone(),
        choice: id("save"),
    }]);
    assert!(g.log.contains(&Outcome::DecisionRejected {
        decision: d2.clone(),
        choice: id("save")
    }));
    assert!(
        g.s.decisions.is_empty(),
        "a rejected decision leaves no trace"
    );
    g.send(&[Fact::Read { id: id("f15") }]);
    g.s.granted.insert(id("f15"));
    g.send(&[
        Fact::Read { id: id("f15") },
        Fact::DecisionMade {
            decision: d2,
            choice: id("save"),
        },
    ]);
    assert_eq!(g.status("s07"), QuestStatus::Available);
}

#[test]
fn s07_stays_closed_when_phoenix_is_abandoned() {
    let mut g = Game::chapter4(shipped());
    g.send(&[Fact::DecisionMade {
        decision: id("d2"),
        choice: id("abandon"),
    }]);
    assert_eq!(g.status("s07"), QuestStatus::Unavailable);
    assert_eq!(g.s.contact(&id::<ContactId>("phoenix")), ContactState::Dead);
}

#[test]
fn an_offered_contract_is_withdrawn_when_its_condition_turns_false() {
    let mut g = Game::chapter4(shipped());
    assert_eq!(g.status("s08"), QuestStatus::Available);
    // The giver turns hostile (D1 = betray, here forced) before the player accepts.
    g.s.contacts
        .insert(id::<ContactId>("r4z0r"), ContactState::Hostile);
    let out = g.send(&[]);
    assert!(out.contains(&Outcome::QuestWithdrawn(q("s08"))));
    assert_eq!(g.status("s08"), QuestStatus::Unavailable);
}

#[test]
fn s13_follows_the_state_of_neon_angel() {
    let free = Game::chapter4(shipped());
    assert_eq!(
        free.status("s13"),
        QuestStatus::Unavailable,
        "angel_state is still `unknown`"
    );
    let mut g = Game::chapter4(shipped());
    g.s.flags.insert(
        id("angel_state"),
        neon_engine::content::schema::FlagValue::Str("free".into()),
    );
    g.send(&[]);
    assert_eq!(g.status("s13"), QuestStatus::Available);
}

fn accepted_s12() -> Game {
    let mut g = Game::chapter4(shipped());
    g.send(&[Fact::QuestAccepted { quest: q("s12") }]);
    assert_eq!(g.status("s12"), QuestStatus::Active);
    g
}

#[test]
fn s12_fails_when_the_heat_peaks_and_the_insider_is_burnt_exactly_once() {
    let mut g = accepted_s12();
    g.tick();
    g.heat(75);
    assert_eq!(g.status("s12"), QuestStatus::Failed);
    assert_eq!(
        g.s.contact(&id::<ContactId>("insider")),
        ContactState::Compromised
    );
    assert_eq!(
        g.log
            .iter()
            .filter(|o| **o == Outcome::QuestFailed(q("s12")))
            .count(),
        1
    );
    g.heat(0);
    g.compromise("sentinel-hub");
    assert_eq!(g.status("s12"), QuestStatus::Failed, "FAILED is final");
    assert!(g.s.claimed.contains("quest-s12-failed") && !g.s.claimed.contains("quest-s12"));
}

#[test]
fn s12_fails_when_the_finale_starts_first() {
    let mut g = accepted_s12();
    g.s.tier = 6;
    g.s.quests.insert(
        q("m12"),
        neon_engine::content::state::QuestRun {
            status: QuestStatus::Completed,
            opened_at: 0,
            done: std::collections::BTreeSet::default(),
        },
    );
    g.send(&[]);
    assert_eq!(g.status("m13"), QuestStatus::Active);
    assert_eq!(g.status("s12"), QuestStatus::Failed);
}

#[test]
fn s12_succeeds_and_the_peak_is_measured_from_its_opening() {
    let mut g = Game::chapter4(shipped());
    g.heat(90); // before the quest: irrelevant
    g.send(&[Fact::QuestAccepted { quest: q("s12") }]);
    g.tick();
    g.heat(0);
    g.compromise("sentinel-hub");
    g.extract("sentinel-hub", "maintenance_schedule");
    g.tick();
    assert_eq!(g.status("s12"), QuestStatus::Completed);
    assert!(
        g.s.flag(&g.c, &id("adv"))
            .is_some_and(|v| format!("{v:?}").contains("maintenance_window"))
    );
}

#[test]
fn a_lost_peak_on_a_quest_that_cannot_fail_waits_for_a_restart() {
    let mut src = Sources::embedded();
    src.quests = src.quests.replace(
        "[[quest.objective]]\nkind = \"pay\"\namount = \"S\"\n\n# ====",
        "[[quest.objective]]\nkind = \"pay\"\namount = \"S\"\n[[quest.objective]]\nkind = \"heat_peak_below\"\nbelow = 40\n\n# ====",
    );
    let c = Content::from_sources_without_texts(&src).unwrap_or_else(|e| panic!("{e}"));
    let mut g = Game::chapter4(c);
    g.send(&[Fact::QuestAccepted { quest: q("s08") }]);
    g.tick();
    g.heat(55);
    g.s.earned = g.s.earned.saturating_add(Credits::new(10_000));
    g.send(&[Fact::SiteMarked {
        site: id("halcyon-clinic-7"),
        mark: neon_engine::content::schema::SiteMark::Backdoor,
    }]);
    g.extract("halcyon-clinic-7", "raid_orders");
    let at = g.turn();
    g.send(&[Fact::Paid {
        amount: Credits::new(400),
        at,
    }]);
    g.heat(0);
    assert_eq!(
        g.status("s08"),
        QuestStatus::Active,
        "the peak of 55 is held against it: blocked, not failed"
    );
    let at = g.turn();
    g.send(&[Fact::QuestRestarted {
        quest: q("s08"),
        at,
    }]);
    assert!(g.log.contains(&Outcome::QuestRestarted(q("s08"))));
    let at = g.turn();
    g.send(&[Fact::Paid {
        amount: Credits::new(400),
        at,
    }]);
    g.tick();
    assert_eq!(
        g.status("s08"),
        QuestStatus::Completed,
        "after the restart the peak is clean"
    );
}

#[test]
fn the_retainer_halves_the_price_of_the_broker() {
    let c = shipped();
    let choices = [
        ("d1", "pay"),
        ("d2", "abandon"),
        ("d3", "burn"),
        ("echo", "wait"),
    ];
    let mut plan = Plan {
        optional: true,
        ..Plan::default()
    };
    for (d, ch) in choices {
        plan.choices.insert(id(d), id(ch));
    }
    let with = play(&c, &plan);
    plan.skip.insert(q("s11"));
    let without = play(&c, &plan);
    let big =
        |r: &crate::optimist::Report, n: u32| r.state.payments.iter().any(|(_, a)| a.get() == n);
    assert!(big(&without, 1600), "L at tier 5 = 2 x 800");
    assert!(
        big(&with, 800) && !big(&with, 1600),
        "with the retainer the guard selects M"
    );
}

#[test]
fn topics_use_the_same_conditions_and_effects() {
    let mut g = Game::new();
    let r4z0r = id::<ContactId>("r4z0r");
    assert!(
        available_topics(&g.c, &g.s, &r4z0r).is_empty(),
        "offline: no topic at all"
    );
    g.s.contacts.insert(r4z0r.clone(), ContactState::Available);
    let ids = |g: &Game| {
        available_topics(&g.c, &g.s, &r4z0r)
            .iter()
            .map(|t| t.id.to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&g), ["how_to_earn"]);
    g.s.quests.insert(
        q("m04"),
        neon_engine::content::state::QuestRun {
            status: QuestStatus::Active,
            opened_at: 0,
            done: std::collections::BTreeSet::default(),
        },
    );
    assert_eq!(ids(&g), ["how_to_earn", "ledger_rumor"]);
    let trust = g.s.trust(&g.c, &r4z0r);
    let fact = Fact::TopicChosen {
        contact: r4z0r.clone(),
        topic: id("ledger_rumor"),
    };
    g.send(std::slice::from_ref(&fact));
    g.send(&[fact]);
    assert_eq!(g.s.trust(&g.c, &r4z0r), trust + 5, "its effect fires once");
    assert_eq!(
        ids(&g),
        ["how_to_earn"],
        "a chosen topic is not offered again"
    );
}

#[test]
fn the_content_is_deterministic() {
    let c = shipped();
    let plan = Plan {
        optional: true,
        ..Plan::default()
    };
    let (a, b) = (play(&c, &plan), play(&c, &plan));
    assert_eq!(a.state, b.state);
    assert_eq!(a.outcomes, b.outcomes);
}
