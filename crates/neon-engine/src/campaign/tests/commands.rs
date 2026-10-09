//! The commands of the hub, one by one: what they do, what they refuse, and what they say.

use super::support::*;
use crate::campaign::{CampaignGame, Difficulty};
use crate::content::schema::{ContactState, QuestStatus};
use crate::game::Game;
use crate::prompt::{Input, Prompt};
use crate::save::SaveRequest;
use crate::text::Lang;

/// The names `help` lists, in order.
fn listed(driver: &mut Driver) -> Vec<String> {
    driver.line("help");
    let crate::event::Event::Screen(table) = &driver.last.events[0] else {
        panic!("help is a table: {:?}", driver.last.events);
    };
    table
        .rows
        .iter()
        .map(|row| render_cell(row.first().unwrap()))
        .collect()
}

fn render_cell(text: &crate::text::Text) -> String {
    crate::text::render(text, &catalog(Lang::En), crate::text::RenderMode::FULL)
}

// ------------------------------------------------------------------------------- the start

#[test]
fn a_new_game_asks_the_handle_then_tells_the_prologue_and_keeps_a_checkpoint() {
    let mut driver = Driver::raw();
    assert!(matches!(driver.last.prompt, Prompt::Text { .. }));
    assert!(driver.saves.is_empty());
    driver.line("  Neo\tn  ");
    let said = driver.text();
    assert!(said.contains("Welcome to the net, Neon."), "{said}");
    assert!(said.contains("Neo-Tokyo, 2087."), "{said}");
    assert!(
        !said.contains("New quest:"),
        "the quest is told at the end: {said}"
    );
    assert_eq!(driver.last.prompt, Prompt::Continue);
    assert_eq!(driver.game.state.handle, "Neon");
    driver.send(Input::Continue);
    assert_eq!(driver.last.prompt, Prompt::Continue);
    assert!(driver.text().contains("Nexus Corp runs the city"));
    driver.send(Input::Continue);
    let said = driver.text();
    assert!(said.contains("Encrypted channel #7"), "{said}");
    assert!(said.contains("echo7 > Finally."), "{said}");
    assert!(said.contains("Handle on file: Neon."), "{said}");
    assert!(matches!(
        driver.last.prompt,
        Prompt::Confirm { default: true, .. }
    ));
    // No checkpoint before the opening is over; then one keeps the very beginning.
    assert!(!driver.saves.contains(&SaveRequest::Checkpoint));
    driver.send(Input::Confirm(false));
    let said = driver.text();
    assert!(said.contains("New quest:"), "{said}");
    assert!(said.contains("Type `help` to list the commands"), "{said}");
    assert_eq!(driver.last.prompt, Prompt::Command);
    assert_eq!(driver.saves.last(), Some(&SaveRequest::Checkpoint));
    assert_eq!(
        driver
            .saves
            .iter()
            .filter(|save| **save == SaveRequest::Checkpoint)
            .count(),
        1
    );
}

#[test]
fn the_handle_is_cleaned_like_the_demos_and_never_empty() {
    let mut driver = Driver::raw();
    driver.line(&"x".repeat(50));
    assert_eq!(driver.game.state.handle.chars().count(), 20);
    let mut driver = Driver::raw();
    driver.send(Input::Cancel);
    assert_eq!(driver.game.state.handle, "Neon");
    let mut driver = Driver::raw();
    driver.line("  \u{7}  ");
    assert_eq!(driver.game.state.handle, "Neon");
}

#[test]
fn resuming_tells_the_situation_and_never_replays_the_welcome() {
    let mut driver = Driver::new();
    driver.finish_m01();
    let text = driver.game.snapshot().unwrap();
    let resumed = CampaignGame::from_save(&text).unwrap();
    let step = resumed.resume();
    assert_eq!(step.prompt, Prompt::Command);
    let cat = catalog(Lang::En);
    let said: String = step
        .events
        .iter()
        .map(|event| render_event(event, &cat))
        .collect();
    assert!(
        said.contains("Back on the net, Neon: 1 active quest, 1 unread message."),
        "{said}"
    );
    assert!(!said.contains("Welcome"), "{said}");
}

// ------------------------------------------------------------------------- help and unlocks

#[test]
fn help_lists_exactly_the_commands_that_are_open_and_each_one_runs() {
    let mut driver = Driver::new();
    let open = listed(&mut driver);
    assert_eq!(
        open,
        [
            "help", "status", "quit", "quests", "contacts", "talk", "net", "hack", "laylow",
            "hint", "save", "tutorial"
        ]
    );
    for name in &open {
        let said = driver.line(name);
        assert!(!said.contains("Unknown command"), "{name}: {said}");
        assert!(!said.contains("Not yet"), "{name}: {said}");
        // Leave any menu or confirmation a command opened.
        if driver.last.prompt != Prompt::Command {
            driver.send(Input::Cancel);
        }
        if driver.last.prompt != Prompt::Command {
            driver.send(Input::Confirm(false));
        }
    }
}

#[test]
fn the_commands_that_are_not_open_yet_say_when_they_open() {
    let mut driver = Driver::new();
    let open = listed(&mut driver);
    for name in [
        "shop", "buy", "messages", "read", "archives", "decrypt", "accept",
    ] {
        assert!(!open.contains(&name.to_owned()), "{name} is listed");
        let said = driver.line(name);
        assert!(driver.errored(), "{name}: {said}");
        assert!(said.contains("Not yet: "), "{name}: {said}");
        assert!(!said.contains("Unknown"), "{name}: {said}");
    }
    assert!(driver.line("shop").contains("Not yet: find R4Z0R."));
    assert!(
        driver
            .line("messages")
            .contains("Not yet: wait for a first message.")
    );
    assert!(
        driver
            .line("accept s06")
            .contains("wait for a contract to be offered")
    );
    // A locked command is never completed.
    assert!(!driver.game.complete("sh").contains(&"shop".to_owned()));
    assert!(driver.game.complete("sh").is_empty());
}

#[test]
fn finishing_the_first_quest_opens_the_stall_the_inbox_and_the_archives() {
    let mut driver = Driver::new();
    driver.finish_m01();
    let open = listed(&mut driver);
    for name in ["shop", "buy", "messages", "read", "archives", "decrypt"] {
        assert!(
            open.contains(&name.to_owned()),
            "{name} is not listed: {open:?}"
        );
    }
    // The first contract is not on offer yet.
    assert!(!open.contains(&"accept".to_owned()));
    assert!(driver.game.complete("sh").contains(&"shop".to_owned()));
}

#[test]
fn a_command_name_is_a_word_whatever_its_case_and_has_aliases() {
    let mut driver = Driver::new();
    assert!(driver.line("HELP").contains("Commands"));
    assert!(driver.line("  h ").contains("Commands"));
    assert!(driver.line("st").contains("Handle: Neon"));
    for alias in ["journal", "q", "QUESTS"] {
        assert!(driver.line(alias).contains("== Journal =="), "{alias}");
    }
    assert!(driver.line("").is_empty());
    assert!(driver.line("   ").is_empty());
    let said = driver.line("bogus now");
    assert!(driver.errored());
    assert!(said.contains("Unknown command: bogus."), "{said}");
}

#[test]
fn help_explains_one_command_with_its_usage_and_refuses_a_locked_one() {
    let mut driver = Driver::new();
    let said = driver.line("help hack");
    assert!(said.contains("Start a run on a known site"), "{said}");
    assert!(said.contains("Usage: `hack <site>`"), "{said}");
    let said = driver.line("help status");
    assert!(said.contains("Show your situation"));
    assert!(!said.contains("Usage"), "{said}");
    let said = driver.line("help shop");
    assert!(said.contains("Not yet: find R4Z0R."), "{said}");
    let said = driver.line("help nonsense");
    assert!(said.contains("Unknown word: nonsense."), "{said}");
}

#[test]
fn commands_of_the_interface_are_declared_but_answer_for_it() {
    let mut driver = Driver::new();
    assert!(driver.line("panel").contains("Only in full screen."));
    assert!(driver.line("plain").contains("Only in full screen."));
    for name in ["load", "export"] {
        let said = driver.line(name);
        assert!(driver.errored(), "{name}: {said}");
        assert!(
            said.contains("interface does not offer it yet"),
            "{name}: {said}"
        );
    }
    // They are declared in the table all the same.
    let names: Vec<&str> = crate::campaign::command_names().collect();
    for name in ["load", "panel", "plain", "export"] {
        assert!(names.contains(&name));
    }
}

#[test]
fn arguments_missing_ambiguous_or_unknown_are_told_before_anything_runs() {
    let mut driver = Driver::new();
    let said = driver.line("talk");
    assert!(
        said.contains("Missing argument. Usage: `talk <contact>`."),
        "{said}"
    );
    let said = driver.line("talk 7");
    assert!(
        said.contains("Unknown contact: 7. See `contacts`."),
        "{said}"
    );
    let said = driver.line("hack nowhere");
    assert!(said.contains("Unknown site: nowhere. See `net`."), "{said}");
    let before = driver.game.state.clone();
    driver.line("hack");
    driver.line("talk 7");
    driver.line("quests 9");
    assert_eq!(
        driver.game.state, before,
        "a refused command changes nothing"
    );
    driver.finish_m02();
    let said = driver.line("quests m");
    assert!(
        said.contains("\"m\" fits several quests: 1. m01, 2. m02, 3. m03."),
        "{said}"
    );
}

// ------------------------------------------------------------------------------------ status

#[test]
fn status_gives_the_situation_in_words() {
    let mut driver = Driver::new();
    let said = driver.line("status");
    for line in [
        "Handle: Neon",
        "Level: 1",
        "Credits: 100",
        "Notoriety: 0/100 (Discreet)",
        "Reputation: 0",
        "Chapter: 1",
        "Difficulty: Normal",
        "Quests in progress: 1",
        "Hints left for the current quest: 3",
    ] {
        assert!(said.contains(line), "{line} is missing from:\n{said}");
    }
    let french = driver.rendered(Lang::Fr);
    assert!(french.contains("Notoriété : 0/100 (Discret)"), "{french}");
    assert!(french.contains("Difficulté : Normal"), "{french}");
}

#[test]
fn the_difficulty_is_chosen_at_the_start_and_sets_the_hints() {
    for (difficulty, budget) in [
        (Difficulty::Story, 6),
        (Difficulty::Normal, 3),
        (Difficulty::Expert, 1),
        (Difficulty::Hardcore, 0),
    ] {
        let mut driver = Driver::raw_with(difficulty);
        driver.opening(false);
        let mut given = 0;
        for _ in 0..10 {
            driver.line("hint");
            if !driver.errored() {
                given += 1;
            }
        }
        assert_eq!(given, budget, "{difficulty:?}");
    }
    assert!(Difficulty::Story.allows_skip());
    assert!(!Difficulty::Normal.allows_skip());
}

// ------------------------------------------------------------------------------------ quests

#[test]
fn the_journal_lists_what_the_player_has_met_with_stable_numbers() {
    let mut driver = Driver::new();
    let said = driver.line("quests");
    assert!(said.contains("[1] m01 - "), "{said}");
    assert!(said.contains(" - active - 1"), "{said}");
    driver.finish_m01();
    let said = driver.line("quests");
    // M02 joined the journal after M01: it did not move M01.
    assert!(
        said.contains("[1] m01 - ") && said.contains("completed"),
        "{said}"
    );
    assert!(
        said.contains("[2] m02 - ") && said.contains("active"),
        "{said}"
    );
    assert_eq!(driver.game.state.quest_log.len(), 2);
}

#[test]
fn a_quest_is_read_with_the_progress_of_its_objectives() {
    let mut driver = Driver::new();
    let said = driver.line("quests m01");
    assert!(said.contains("Quest: "), "{said}");
    assert!(said.contains("(active, chapter 1)"), "{said}");
    // Nothing is done yet: seven objectives still to do.
    assert_eq!(said.matches("[ ] ").count(), 7, "{said}");
    driver.lines(&["help", "status"]);
    let said = driver.line("quests 1");
    assert_eq!(said.matches("[x] ").count(), 3, "{said}");
    assert_eq!(said.matches("[ ] ").count(), 4, "{said}");
    // The counted objective of M03 shows its progress.
    driver.finish_m02();
    driver.lines(&["hack localhost", "y"]);
    let said = driver.line("quests m03");
    assert!(said.contains("(1/3)"), "{said}");
    // The secret objective is not told.
    assert!(said.contains("[ ] ???"), "{said}");
}

#[test]
fn a_main_quest_opening_asks_for_a_checkpoint() {
    let mut driver = Driver::new();
    driver.saves.clear();
    driver.finish_m01();
    assert!(
        driver.saves.contains(&SaveRequest::Checkpoint),
        "opening M02 keeps a checkpoint: {:?}",
        driver.saves
    );
}

// ----------------------------------------------------------------------------- contacts, talk

#[test]
fn contacts_are_listed_as_they_are_met() {
    let mut driver = Driver::new();
    let said = driver.line("contacts");
    assert!(
        said.contains("[1] echo7 - ECHO-7 - friendly - available"),
        "{said}"
    );
    assert!(!said.contains("r4z0r"), "R4Z0R is not met yet: {said}");
    driver.finish_m01();
    let said = driver.line("contacts");
    assert!(
        said.contains("[2] r4z0r - R4Z0R - neutral - available"),
        "{said}"
    );
    // R4Z0R is offered the next quest? no: ECHO-7 gives M02, it is not an offer to accept.
    assert!(said.contains("[1] echo7"), "{said}");
}

#[test]
fn a_conversation_is_a_menu_of_topics_that_ends_by_itself() {
    let mut driver = Driver::new();
    let trust_before = driver
        .game
        .state
        .missions
        .trust(driver.game.content, &"echo7".parse().unwrap());
    let said = driver.line("talk echo7");
    assert!(
        said.contains("[Reward]") || !said.contains("Error"),
        "{said}"
    );
    let menu = driver.prompt_text();
    assert!(menu.contains("Conversation with ECHO-7"), "{menu}");
    assert!(menu.contains("1. "), "{menu}");
    assert!(menu.contains("0. End the conversation"), "{menu}");
    // +3 trust for the conversation itself.
    let trust = driver
        .game
        .state
        .missions
        .trust(driver.game.content, &"echo7".parse().unwrap());
    assert_eq!(trust, trust_before + 3);
    let said = driver.line("1");
    assert!(said.contains("echo7 > "), "the contact answers: {said}");
    // Nothing is left to say: the conversation ends by itself.
    assert!(
        said.contains("There is nothing more to say for now."),
        "{said}"
    );
    assert_eq!(driver.last.prompt, Prompt::Command);
    // A topic is discussed once; the menu says so and refuses it.
    driver.line("talk echo7");
    assert!(
        driver.prompt_text().contains("(already discussed)"),
        "{}",
        driver.prompt_text()
    );
    let said = driver.line("1");
    assert!(
        driver.errored() && said.contains("already discussed"),
        "{said}"
    );
    driver.line("0");
    assert_eq!(driver.last.prompt, Prompt::Command);
}

#[test]
fn a_topic_that_is_not_open_yet_is_listed_without_being_told() {
    let mut driver = Driver::new();
    driver.finish_m01();
    driver.line("talk r4z0r");
    let menu = driver.prompt_text();
    // `how_to_earn` is open, `ledger_rumor` waits for M04: it shows as ??? with its reason.
    assert!(menu.contains("??? (nothing to say yet)"), "{menu}");
    let said = driver.line("2");
    assert!(
        driver.errored() && said.contains("nothing to say yet"),
        "{said}"
    );
    assert!(driver.line("hello").contains("Choose one of the listed"));
    assert!(matches!(driver.last.prompt, Prompt::Choice(_)));
}

#[test]
fn nobody_unreachable_can_be_talked_to_and_a_contact_with_nothing_to_say_is_told() {
    let mut driver = Driver::new();
    driver.finish_m01();
    driver
        .game
        .state
        .missions
        .contacts
        .insert("r4z0r".parse().unwrap(), ContactState::Hostile);
    let said = driver.line("talk r4z0r");
    assert!(driver.errored(), "{said}");
    assert!(
        said.contains("is unavailable: cannot be reached now (hostile)"),
        "{said}"
    );
    // A contact with no topic at all (whoever the writers have not given one yet): a
    // conversation that goes nowhere.
    let content = driver.game.content;
    let mute = content
        .contacts
        .iter()
        .find(|contact| {
            content
                .topics
                .iter()
                .all(|topic| topic.contact != contact.id)
        })
        .expect("some contact has no topic yet")
        .id
        .clone();
    driver
        .game
        .state
        .missions
        .contacts
        .insert(mute.clone(), ContactState::Available);
    driver.game.state.sync_lists(driver.game.content);
    let said = driver.line(&format!("talk {mute}"));
    assert!(said.contains(" has nothing to say right now."), "{said}");
    assert_eq!(driver.last.prompt, Prompt::Command);
}

#[test]
fn paying_a_contact_is_an_entry_of_the_conversation() {
    let mut driver = Driver::new();
    driver.jump_to_chapter4();
    // Open S08 (R4Z0R's raid): its last objective is a payment.
    driver.feed(Vec::new());
    driver.line("accept s08");
    assert_eq!(driver.status("s08"), QuestStatus::Active);
    driver.line("talk r4z0r");
    let menu = driver.prompt_text();
    assert!(menu.contains("Pay "), "the contact can be paid: {menu}");
    // The payment is a menu entry, listed with its price; it is refused when too dear.
    driver.game.state.missions.earned = crate::content::Credits::new(0);
    driver.game.state.service_spend = 0;
    let menu = driver.prompt_text();
    assert!(menu.contains("short)"), "{menu}");
}

#[test]
fn a_payment_stays_on_offer_when_another_way_already_settled_the_objective() {
    // M10 settles its second objective with any of four ways, and the decision D1 offers
    // "pay" only to a player who paid. The ledger taken in M04 is one of the ways: it must
    // not take the payment away, or paying the Broker would be impossible.
    let mut driver = Driver::new();
    driver.jump_to(&[
        "m01", "m02", "m03", "m04", "m05", "s06", "m06", "m07", "m08", "m09",
    ]);
    driver.game.state.missions.earned = crate::content::Credits::new(5_000);
    // The end of M09 hands the Broker over.
    driver
        .game
        .state
        .missions
        .contacts
        .insert("broker".parse().unwrap(), ContactState::Available);
    driver.feed(vec![crate::content::Fact::FileExtracted {
        site: "underground-market".parse().unwrap(),
        file: "black_ledger".parse().unwrap(),
    }]);
    assert_eq!(driver.status("m10"), QuestStatus::Active);
    driver.line("talk broker");
    let menu = driver.prompt_text();
    assert!(
        menu.contains("Pay "),
        "the Broker can still be paid: {menu}"
    );
    let pay = menu
        .lines()
        .find(|line| line.contains("Pay "))
        .and_then(|line| line.trim().split('.').next())
        .unwrap()
        .to_owned();
    let before = driver.credits();
    driver.line(&pay);
    assert!(driver.credits() < before);
    // Paid once: the entry is gone, and the decision now offers the payment.
    driver.line("0");
    driver.line("talk broker");
    let menu = driver.prompt_text();
    assert!(!menu.contains("Pay "), "{menu}");
    assert!(menu.contains("Decide: "), "{menu}");
}

#[test]
fn the_neural_link_is_raised_one_level_at_a_time_for_the_companions_only() {
    let mut driver = Driver::new();
    // The command opens with the fourth chapter.
    let said = driver.line("link echo7");
    assert!(said.contains("Not yet: reach chapter 4."), "{said}");
    assert!(!driver.game.complete("li").contains(&"link".to_owned()));
    driver.jump_to_chapter4();
    driver.game.state.missions.flags.insert(
        "chapter".parse().unwrap(),
        crate::content::schema::FlagValue::Int(4),
    );
    driver.feed(Vec::new());
    assert!(driver.game.complete("li").contains(&"link".to_owned()));
    // Only the contacts a quest asks a link with are offered.
    assert_eq!(driver.game.complete("link "), ["echo7", "aura"]);
    let said = driver.line("link phoenix");
    assert!(
        driver.errored() && said.contains("no neural link possible"),
        "{said}"
    );
    let echo7 = "echo7".parse().unwrap();
    for level in 1..=3 {
        let said = driver.line("link echo7");
        assert!(
            said.contains(&format!("Neural link with ECHO-7: level {level}.")),
            "{said}"
        );
        assert_eq!(driver.game.state.missions.links.get(&echo7), Some(&level));
    }
    // At the highest level the contact is listed with its reason and nothing happens.
    let said = driver.line("link echo7");
    assert!(
        driver.errored() && said.contains("the link is at its highest level"),
        "{said}"
    );
    assert_eq!(driver.game.complete("link "), ["aura"]);
    assert_eq!(driver.game.state.missions.links.get(&echo7), Some(&3));
}

#[test]
fn a_decision_is_put_to_the_player_by_the_contact_that_owns_it() {
    let mut driver = Driver::new();
    // M08 is Neon Angel's quest and carries D2: it opens with M07 done.
    driver.jump_to(&["m01", "m02", "m03", "m04", "m05", "s06", "m06", "m07"]);
    driver.feed(Vec::new());
    assert_eq!(driver.status("m08"), QuestStatus::Active);
    driver.saves.clear();
    driver.line("talk angel");
    let menu = driver.prompt_text();
    assert!(menu.contains("Decide: "), "{menu}");
    let decide = menu
        .lines()
        .find(|line| line.contains("Decide: "))
        .and_then(|line| line.trim().split('.').next())
        .unwrap()
        .to_owned();
    driver.line(&decide);
    // A checkpoint was made before the decision, and the options are offered.
    assert!(
        driver.saves.contains(&SaveRequest::Checkpoint),
        "{:?}",
        driver.saves
    );
    let Prompt::Choice(choice) = &driver.last.prompt else {
        panic!("{:?}", driver.last.prompt);
    };
    assert!(choice.options.len() >= 2);
    // The checkpoint holds the menu of the decision: loading it comes back before the choice.
    let text = driver.game.snapshot().unwrap();
    assert_eq!(
        CampaignGame::from_save(&text).unwrap().prompt(),
        driver.last.prompt
    );
    driver.line("abandon");
    assert!(
        driver
            .game
            .state
            .missions
            .decisions
            .contains_key(&"d2".parse().unwrap())
    );
}

// --------------------------------------------------------------------- messages and archives

#[test]
fn messages_arrive_unread_and_are_read_once() {
    let mut driver = Driver::new();
    driver.finish_m01();
    let said = driver.line("messages");
    assert!(
        said.contains("[1] mail-welcome - ") && said.contains("unread"),
        "{said}"
    );
    let clock = driver.game.clock();
    let said = driver.line("read 1");
    assert!(said.contains("Welcome to the underground"), "{said}");
    assert!(said.contains("Hey, rookie."), "{said}");
    assert!(driver.game.clock() > clock, "reading changes the world");
    assert!(driver.line("messages").contains("read"), "read now");
    let clock = driver.game.clock();
    driver.line("read mail-welcome");
    assert_eq!(driver.game.clock(), clock, "reading again changes nothing");
    assert!(
        driver
            .line("read 2")
            .contains("Unknown message: 2. See `messages`.")
    );
}

#[test]
fn a_document_is_decrypted_from_the_list_and_gives_a_fragment() {
    let mut driver = Driver::new();
    driver.finish_m01();
    let said = driver.line("archives");
    assert!(
        said.contains("[1] doc_phase2 - ") && said.contains("encrypted"),
        "{said}"
    );
    let said = driver.line("archives 1");
    assert!(
        driver.errored() && said.contains("encrypted: see `decrypt`"),
        "{said}"
    );
    let said = driver.line("decrypt 1");
    assert!(said.contains("Document decrypted: "), "{said}");
    assert!(said.contains("New fragment in the archives: "), "{said}");
    let said = driver.line("archives");
    assert!(
        said.contains("[1] doc_phase2 - ") && said.contains("decrypted"),
        "{said}"
    );
    assert!(
        said.contains("[2] f05 - ") && said.contains("unread"),
        "{said}"
    );
    driver.line("archives f05");
    assert!(
        driver
            .line("archives")
            .contains("[2] f05 - Intercepted Transmission - read")
    );
    assert!(driver.line("decrypt 1").contains("already decrypted"));
    assert!(driver.line("decrypt 2").contains("not encrypted"));
}

#[test]
fn a_document_that_needs_an_item_says_which() {
    let mut driver = Driver::new();
    driver.finish_m01();
    driver.game.state.missions.tier = 6;
    driver.feed(Vec::new());
    let said = driver.line("decrypt doc_vault");
    assert!(driver.errored(), "{said}");
    assert!(said.contains("needs Quantum Processing Chip"), "{said}");
}

// ---------------------------------------------------------------------------- the net, hack

#[test]
fn the_net_lists_every_site_and_hides_the_unknown_ones() {
    let mut driver = Driver::new();
    let said = driver.line("net");
    assert!(said.contains("[1] localhost - known - 1"), "{said}");
    assert_eq!(said.matches("??? - unknown").count(), 0, "{said}");
    assert!(said.contains("15 more sites still unknown."), "{said}");
    assert!(!said.contains("corp-server-01"), "{said}");
    let said = driver.line("net 2");
    assert!(driver.errored() && said.contains("not known yet"), "{said}");
    let said = driver.line("net 1");
    assert!(said.contains("Site localhost: known, level 1."), "{said}");
    assert!(said.contains("Loot: 0/1 file taken."), "{said}");
}

#[test]
fn a_site_becomes_known_through_its_relay_and_its_level() {
    let mut driver = Driver::new();
    driver.lines(&["hack localhost", "y"]);
    // Level 2 is not reached yet: the big server stays unknown.
    assert!(driver.line("net").contains("more sites still unknown."));
    driver.finish_m01();
    let said = driver.line("net");
    assert!(said.contains("corp-server-01 - known - 2"), "{said}");
    assert!(said.contains("localhost - pierced - 1"), "{said}");
    // A relay that is not pierced keeps its sites out of reach (nexus-mainframe needs corp).
    assert!(
        driver
            .line("hack nexus-mainframe")
            .contains("Unknown site: nexus-mainframe.")
    );
    assert!(driver.line("hack 3").contains("not known yet"));
}

#[test]
fn hack_asks_first_and_can_be_called_off() {
    let mut driver = Driver::new();
    let said = driver.line("hack localhost");
    assert_eq!(said, "");
    assert!(
        driver
            .prompt_text()
            .contains("Start the run on the site localhost?")
    );
    let before = driver.game.state.missions.clone();
    driver.line("n");
    assert!(driver.text().contains("Run cancelled."));
    assert_eq!(driver.game.state.missions, before);
    assert_eq!(driver.last.prompt, Prompt::Command);
}

#[test]
fn an_intrusion_takes_the_files_costs_notoriety_and_pays_the_first_breach_once() {
    let mut driver = Driver::new();
    driver.finish_m01();
    let credits = driver.credits();
    let reputation = driver.game.state.missions.reputation.get();
    let notoriety = driver.game.notoriety();
    driver.line("hack corp-server-01");
    let said = driver.line("y");
    assert!(
        said.contains("Run on corp-server-01: 2 files taken."),
        "{said}"
    );
    assert!(said.contains("Notoriety "), "{said}");
    assert!(said.contains("[Reward] +"), "{said}");
    assert_eq!(driver.game.notoriety(), notoriety + 8, "level 2 costs 8");
    assert!(driver.credits() > credits);
    assert!(driver.game.state.missions.reputation.get() > reputation);
    assert_eq!(driver.game.state.missions.extracted.len(), 2 + 1);
    // Again: nothing to take, nothing paid, and the world still notices.
    let (credits, reputation) = (
        driver.credits(),
        driver.game.state.missions.reputation.get(),
    );
    driver.line("hack corp-server-01");
    let said = driver.line("y");
    assert!(said.contains("nothing new to take"), "{said}");
    assert!(!said.contains("[Reward]"), "{said}");
    assert_eq!(driver.credits(), credits);
    assert_eq!(driver.game.state.missions.reputation.get(), reputation);
    assert_eq!(driver.game.notoriety(), notoriety + 16);
}

#[test]
fn crossing_a_band_of_notoriety_is_an_alert() {
    let mut driver = Driver::new();
    driver.finish_m02();
    driver.game.state.base_notoriety = 27;
    driver.feed(Vec::new());
    driver.line("hack corp-server-01");
    let said = driver.line("y");
    assert!(said.contains("[ALERT] Notoriety up: Watched."), "{said}");
    assert!(said.contains("Notoriety 27 -> 35 (Watched)"), "{said}");
}

// -------------------------------------------------------------------------- shop, buy, laylow

#[test]
fn the_shop_lists_every_item_with_its_state_in_words() {
    let mut driver = Driver::new();
    driver.finish_m01();
    let said = driver.line("shop");
    assert!(
        said.contains("[1] ghost_protocol - Ghost Protocol - 80 credits - available"),
        "{said}"
    );
    assert!(
        said.contains("[2] stealth_module - Stealth Module v2.0 - 150 credits - available"),
        "{said}"
    );
    assert!(
        said.contains("[3] quantum_key - Quantum Encryption Key - 200 credits - level 3 needed"),
        "{said}"
    );
    assert!(said.contains("level 5 needed"), "{said}");
}

#[test]
fn a_purchase_is_confirmed_after_a_checkpoint_and_cannot_be_repeated() {
    let mut driver = Driver::new();
    driver.finish_m01();
    // A declined purchase costs nothing.
    driver.lines(&["buy 2", "n"]);
    assert!(driver.text().contains("Purchase cancelled."));
    assert!(driver.game.state.missions.owned.is_empty());
    driver.saves.clear();
    let credits = driver.credits();
    driver.line("buy ghost");
    // The checkpoint is made while the question is asked, before anything is spent.
    assert_eq!(driver.saves, [SaveRequest::Checkpoint]);
    assert_eq!(driver.credits(), credits);
    assert!(
        driver
            .prompt_text()
            .contains("Buy Ghost Protocol for 80 credits?"),
        "{}",
        driver.prompt_text()
    );
    let said = driver.line("y");
    assert!(said.contains("[Reward] Bought: Ghost Protocol."), "{said}");
    assert_eq!(driver.credits(), credits - 80);
    assert_eq!(driver.saves.last(), Some(&SaveRequest::Autosave));
    let said = driver.line("buy 1");
    assert!(driver.errored() && said.contains("owned"), "{said}");
    assert_eq!(driver.game.state.missions.owned.len(), 1);
}

#[test]
fn a_purchase_is_refused_for_want_of_credits_level_or_market() {
    let mut driver = Driver::new();
    driver.finish_m01();
    driver.lines(&["buy 1", "y"]);
    let said = driver.line("buy 2");
    assert!(driver.errored() && said.contains("short"), "{said}");
    let said = driver.line("buy quantum_key");
    assert!(said.contains("level 3 needed"), "{said}");
    // Too well known, the market closes whatever the purse.
    driver.game.state.missions.earned = crate::content::Credits::new(5_000);
    driver.game.state.base_notoriety = 85;
    driver.feed(Vec::new());
    assert!(
        driver
            .line("shop")
            .contains("the market is closed: too much Notoriety")
    );
    assert!(driver.line("buy 2").contains("the market is closed"));
}

#[test]
fn laylow_sells_cover_that_lowers_the_notoriety_for_credits() {
    let mut driver = Driver::new();
    driver.finish_m01();
    driver.lines(&["hack localhost", "y", "hack localhost", "y"]);
    let notoriety = driver.game.notoriety();
    assert!(notoriety >= 12);
    let said = driver.line("laylow");
    assert!(
        said.contains("[1] lie_low - Lie low - 25 credits - -10 - available"),
        "{said}"
    );
    let credits = driver.credits();
    let said = driver.line("laylow 1");
    assert!(said.contains("Lie low: Notoriety lowered by 10."), "{said}");
    assert_eq!(driver.game.notoriety(), notoriety - 10);
    assert_eq!(driver.credits(), credits - 25);
    // The services are not payments of the story.
    assert!(driver.game.state.missions.payments.is_empty());
    driver.game.state.base_notoriety = 0;
    driver.feed(Vec::new());
    let said = driver.line("laylow 1");
    assert!(
        driver.errored() && said.contains("nothing to hide"),
        "{said}"
    );
}

// ------------------------------------------------------------------------- hint, save, quit

#[test]
fn a_hint_comes_from_echo7_in_two_levels_and_is_counted() {
    let mut driver = Driver::new();
    let said = driver.line("hint");
    assert!(
        said.contains("echo7 > Start by taking stock, rookie."),
        "{said}"
    );
    assert!(said.contains("Hints left for this quest: 2."), "{said}");
    let said = driver.line("hint");
    assert!(
        said.contains("Type `quests`."),
        "the solution comes second: {said}"
    );
    driver.line("hint");
    let said = driver.line("hint");
    assert!(
        driver.errored() && said.contains("No hint left for this quest"),
        "{said}"
    );
    // The count is per quest and part of the saved state.
    let text = driver.game.snapshot().unwrap();
    assert!(text.contains("[state.hints]"), "{text}");
    assert_eq!(
        CampaignGame::from_save(&text).unwrap().state().hints,
        driver.game.state.hints
    );
}

#[test]
fn a_hint_has_something_to_be_about() {
    let mut driver = Driver::new();
    driver.game.state.missions.quests.clear();
    driver.feed(Vec::new());
    driver.game.state.missions.quests.clear();
    let said = driver.line("hint");
    assert!(
        said.contains("There is no quest in progress to give a hint about."),
        "{said}"
    );
}

#[test]
fn save_asks_for_a_slot_and_quit_saves_at_the_command_line() {
    let mut driver = Driver::new();
    driver.send(Input::Line("save".to_owned()));
    assert_eq!(driver.last.save, Some(SaveRequest::Slot(1)));
    driver.send(Input::Line("save 4".to_owned()));
    assert_eq!(driver.last.save, Some(SaveRequest::Slot(4)));
    for bad in ["save 0", "save 10", "save two"] {
        driver.send(Input::Line(bad.to_owned()));
        assert!(driver.errored(), "{bad}");
        assert_eq!(driver.last.save, None, "{bad}");
    }
    driver.line("quit");
    assert!(matches!(
        driver.last.prompt,
        Prompt::Confirm { default: false, .. }
    ));
    // Changing one's mind keeps playing.
    driver.send(Input::Confirm(false));
    assert_eq!(driver.last.prompt, Prompt::Command);
    driver.line("quit");
    driver.send(Input::Confirm(true));
    assert_eq!(driver.last.prompt, Prompt::End);
    assert_eq!(driver.last.save, Some(SaveRequest::Autosave));
    assert!(driver.text().contains("Goodbye, Neon."));
    // Nothing is accepted after the end.
    driver.line("help");
    assert_eq!(driver.last.prompt, Prompt::End);
    assert!(driver.last.events.is_empty());
}

#[test]
fn closing_the_input_ends_the_game_at_any_depth_without_saving() {
    for script in [
        &[][..],
        &["Neon", "talk echo7"][..],
        &["Neon", "hack localhost"][..],
    ] {
        let mut driver = Driver::raw();
        for line in script {
            driver.line(line);
        }
        driver.send(Input::Eof);
        assert_eq!(driver.last.prompt, Prompt::End);
        assert_eq!(driver.last.save, None);
        assert!(driver.text().contains("Input closed"));
    }
}

#[test]
fn a_changed_state_asks_for_an_autosave_and_a_reading_does_not() {
    let mut driver = Driver::new();
    driver.saves.clear();
    driver.line("help");
    driver.line("status");
    // `help` and `status` are the first uses of commands the first quest counts: that is a
    // change of the state, so it is saved once; asking again changes nothing.
    let after_first = driver.saves.len();
    driver.line("help");
    driver.line("status");
    driver.line("contacts");
    assert_eq!(driver.saves.len(), after_first);
    driver.lines(&["hack localhost", "y"]);
    assert_eq!(driver.saves.last(), Some(&SaveRequest::Autosave));
}

// ---------------------------------------------------------------------------- completion

#[test]
fn completion_proposes_what_can_be_used_now() {
    let mut driver = Driver::new();
    assert_eq!(driver.game.complete("qu"), ["quit", "quests"]);
    assert_eq!(driver.game.complete("jo"), ["journal"]);
    assert!(driver.game.complete("zz").is_empty());
    assert_eq!(driver.game.complete("talk e"), ["echo7"]);
    assert_eq!(driver.game.complete("hack l"), ["localhost"]);
    // An unknown site is never proposed, even by its prefix.
    assert!(driver.game.complete("hack c").is_empty());
    assert!(driver.game.complete("hack ?").is_empty());
    driver.line("talk echo7");
    assert_eq!(driver.game.complete(""), ["topic:advice"]);
    assert_eq!(driver.game.complete("TOPIC"), ["topic:advice"]);
}

#[test]
fn texts_read_in_both_languages_without_a_missing_key() {
    let mut driver = Driver::new();
    for line in [
        "help",
        "help hack",
        "status",
        "quests",
        "quests m01",
        "contacts",
        "net",
        "net 1",
        "hint",
        "talk echo7",
        "1",
        "0",
    ] {
        driver.line(line);
        for lang in Lang::ALL {
            let text = driver.rendered(lang);
            assert!(!text.contains("<missing:"), "{lang:?} {line}: {text}");
            assert!(!text.contains("<?"), "{lang:?} {line}: {text}");
        }
    }
}

#[test]
fn an_intrusion_leaves_the_marks_the_quests_in_progress_ask_for_on_that_site() {
    use crate::content::schema::SiteMark;

    let mut driver = Driver::new();
    driver.jump_to(&["m01", "m02", "m03", "m04", "m05", "s06", "m06"]);
    for relay in ["localhost", "corp-server-01", "underground-market"] {
        driver
            .game
            .state
            .missions
            .compromised
            .insert(relay.parse().unwrap(), 1);
    }
    driver.feed(Vec::new());
    // M07 asks for a backdoor on the research lab and a virus on the banking network.
    assert_eq!(driver.status("m07"), QuestStatus::Active);
    driver.lines(&["hack research-lab", "y"]);
    let marks = &driver.game.state.missions.marks;
    assert!(marks.contains(&("research-lab".parse().unwrap(), SiteMark::Backdoor)));
    assert!(!marks.contains(&("research-lab".parse().unwrap(), SiteMark::Virus)));
    // A site nobody asks anything of is left unmarked.
    driver.lines(&["hack corp-server-01", "y"]);
    assert!(
        driver
            .game
            .state
            .missions
            .marks
            .iter()
            .all(|(site, _)| site.as_str() == "research-lab")
    );
    driver.lines(&["hack banking-network", "y"]);
    let marks = &driver.game.state.missions.marks;
    assert!(marks.contains(&("banking-network".parse().unwrap(), SiteMark::Virus)));
}
