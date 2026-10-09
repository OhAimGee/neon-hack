//! The story played through the commands: a greedy player who does what each quest asks.

use super::support::*;
use crate::content::schema::{ContactState, QuestStatus};
use crate::game::Game;
use crate::prompt::Prompt;
use crate::save::SaveRequest;
use crate::text::Lang;

/// Every line of the greedy player from the first command to the end of M03. It uses only
/// commands that are open when it types them (the driver says so when one is refused).
const GREEDY_CHAPTER_1: &[&str] = &[
    // M01: the tutorial, in the order the spec teaches it.
    "quests",
    "help",
    "net",
    "status",
    "talk echo7",
    "1",
    "hack localhost",
    "y",
    "laylow",
    // M02: the first big server.
    "hack corp-server-01",
    "y",
    // M03: R4Z0R, the stall, three sites, the first document.
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
];

#[test]
fn a_greedy_player_finishes_the_first_three_quests_with_typed_lines() {
    let mut driver = Driver::new();
    let mut checkpoints = 0;
    for line in GREEDY_CHAPTER_1 {
        let said = driver.line(line);
        assert!(!driver.errored(), "`{line}` was refused:\n{said}");
        // Every language reads every step, and the prompt is the one the state gives.
        for lang in Lang::ALL {
            let text = driver.rendered(lang);
            assert!(
                !text.contains("<missing:") && !text.contains("<?"),
                "{lang:?} `{line}`: {text}"
            );
        }
        assert_eq!(driver.last.prompt, driver.game.prompt(), "`{line}`");
        checkpoints += usize::from(driver.last.save == Some(SaveRequest::Checkpoint));
    }
    for quest in ["m01", "m02", "m03"] {
        assert_eq!(driver.status(quest), QuestStatus::Completed, "{quest}");
    }
    // M04 opened by itself, with the Data Miner met on the way.
    assert_eq!(driver.status("m04"), QuestStatus::Active);
    let miner = "miner".parse().unwrap();
    assert_eq!(
        driver.game.state.missions.contact(&miner),
        ContactState::Available
    );
    assert_eq!(driver.game.state.missions.tier, 3);
    assert_eq!(driver.last.prompt, Prompt::Command);

    // The economy of the three quests, to the credit: 100 to begin with, M01 (80), the first
    // breach of the big server (75) and M02 (150), the first breach of the market (100) and M03
    // (200), less the Stealth Module (150).
    assert_eq!(driver.credits(), 100 + 80 + 75 + 150 + 100 + 200 - 150);
    // Reputation: M01 10, M02 25, M03 40, and 10 for each of the two first breaches.
    assert_eq!(
        driver.game.state.missions.reputation.get(),
        10 + 25 + 40 + 10 + 10
    );
    // Notoriety: 4 per localhost, 8 per big server, 12 for the market.
    assert_eq!(driver.game.notoriety(), 4 + 8 + 4 + 8 + 12);
    // A checkpoint when the game began, then one for each main quest that opened (M02, M03,
    // M04) and one before the purchase.
    assert_eq!(checkpoints, 3 + 1, "{:?}", driver.saves);
    assert_eq!(
        driver
            .saves
            .iter()
            .filter(|s| **s == SaveRequest::Checkpoint)
            .count(),
        1 + 3 + 1
    );

    // The game is in a state that saves and loads back as it was.
    let text = driver.game.snapshot().unwrap();
    let loaded = crate::campaign::CampaignGame::from_save(&text).unwrap();
    assert_eq!(loaded.snapshot().unwrap(), text);
    assert_eq!(loaded.prompt(), driver.game.prompt());
}

#[test]
fn a_cutscene_plays_when_it_is_unlocked_and_is_marked_seen() {
    let mut driver = Driver::new();
    for line in GREEDY_CHAPTER_1 {
        driver.line(line);
    }
    // CS02 came with the end of M03: it played, and the world knows it was seen.
    let cutscene = "cs02".parse().unwrap();
    assert!(driver.game.state.missions.opened.contains(&cutscene));
    let cat = catalog(Lang::En);
    let said: String = driver
        .history
        .iter()
        .map(|event| render_event(event, &cat))
        .collect();
    assert!(said.contains("TODO cutscene.cs02.p01"), "{said}");
    assert!(said.contains("TODO cutscene.cs02.p04"), "{said}");
}

#[test]
fn the_message_of_the_first_quest_arrives_and_the_contract_offer_waits_for_chapter_3() {
    let mut driver = Driver::new();
    driver.finish_m01();
    assert!(
        driver
            .game
            .state
            .inbox
            .contains(&"mail-welcome".parse().unwrap())
    );
    assert_eq!(
        driver
            .game
            .state
            .missions
            .contact(&"r4z0r".parse().unwrap()),
        ContactState::Available
    );
    let said = driver.line("accept s06");
    assert!(said.contains("Not yet"), "{said}");
}

#[test]
fn a_contract_is_offered_then_accepted_by_its_command() {
    let mut driver = Driver::new();
    driver.jump_to(&["m01", "m02", "m03", "m04", "m05"]);
    let said = driver.feed(Vec::new());
    assert!(said.contains("offers a contract:"), "{said}");
    assert!(said.contains("Type `accept s06` to take it."), "{said}");
    assert_eq!(driver.status("s06"), QuestStatus::Available);
    // Offered contracts are told by `contacts` too.
    assert!(driver.line("contacts").contains("1 contract"));
    let said = driver.line("accept s06");
    assert!(said.contains("Contract accepted:"), "{said}");
    assert_eq!(driver.status("s06"), QuestStatus::Active);
    // Accepting twice is refused with the reason.
    let said = driver.line("accept s06");
    assert!(
        driver.errored() && said.contains("already accepted"),
        "{said}"
    );
    // A main quest cannot be accepted: it opens by itself.
    let said = driver.line("accept m06");
    assert!(driver.errored(), "{said}");
}

#[test]
fn a_paid_objective_is_paid_in_the_conversation_and_once() {
    let mut driver = Driver::new();
    driver.jump_to(&[
        "m01", "m02", "m03", "m04", "m05", "s06", "m06", "m07", "m08",
    ]);
    driver.feed(Vec::new());
    driver.game.state.missions.earned = crate::content::Credits::new(5_000);
    driver.line("accept s08");
    // S08 asks for a payment of size S at tier 5: 400 credits.
    driver.line("talk r4z0r");
    let menu = driver.prompt_text();
    assert!(menu.contains("Pay 400 credits"), "{menu}");
    let before = driver.credits();
    let pay = menu
        .lines()
        .find(|line| line.contains("Pay 400"))
        .and_then(|line| line.trim().split('.').next())
        .unwrap()
        .to_owned();
    let said = driver.line(&pay);
    assert!(said.contains("Paid 400 credits"), "{said}");
    assert_eq!(driver.credits(), before - 400);
    assert_eq!(driver.game.state.missions.payments.len(), 1);
    // The objective is done: the entry is gone and paying again is not possible.
    driver.line("0");
    driver.line("talk r4z0r");
    assert!(
        !driver.prompt_text().contains("Pay 400"),
        "{}",
        driver.prompt_text()
    );
}
