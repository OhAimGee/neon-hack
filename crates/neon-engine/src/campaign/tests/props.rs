//! Properties of the whole game over random input: valid lines, hostile lines, menus answered
//! at random. Whatever the player types, the game must stay coherent, bounded, saveable,
//! deterministic and quittable.

use proptest::prelude::*;

use super::support::*;
use crate::campaign::{CampaignGame, Difficulty};
use crate::content::Content;
use crate::content::money::CREDITS_CAP;
use crate::content::schema::{Block, Effect};
use crate::event::Event;
use crate::game::Game;
use crate::prompt::{Input, Prompt};
use crate::text::{Catalog, Lang, RenderMode, render};

// ------------------------------------------------------------------------------- strategies

const COMMANDS: &[&str] = &[
    "help", "h", "status", "st", "quests", "journal", "q", "accept", "contacts", "talk", "contact",
    "messages", "inbox", "read", "archives", "decrypt", "net", "sites", "hack", "shop", "buy",
    "laylow", "hint", "save", "quit", "exit", "load", "panel", "plain", "export", "bogus", "", "?",
    "HACK", "Quests",
];

const ARGUMENTS: &[&str] = &[
    "",
    "",
    "",
    " 1",
    " 2",
    " 3",
    " 4",
    " 0",
    " 99",
    " -1",
    " 4294967296",
    " echo7",
    " r4z0r",
    " m01",
    " m02",
    " s06",
    " localhost",
    " corp-server-01",
    " underground-market",
    " ???",
    " stealth",
    " ghost",
    " lie_low",
    " doc_phase2",
    " f05",
    " mail-welcome",
    " zz",
    " m",
    " é",
    " 1 2 3",
    " hack",
    " help",
];

fn line() -> impl Strategy<Value = Input> {
    (
        prop::sample::select(COMMANDS),
        prop::sample::select(ARGUMENTS),
    )
        .prop_map(|(command, argument)| Input::Line(format!("{command}{argument}")))
}

fn input() -> impl Strategy<Value = Input> {
    prop_oneof![
        60 => line(),
        14 => (0u8..5).prop_map(|n| Input::Line(n.to_string())),
        6 => any::<bool>().prop_map(Input::Confirm),
        4 => (0usize..5).prop_map(Input::Choice),
        3 => Just(Input::Cancel),
        1 => Just(Input::Continue),
        // No `<`: the game echoes an unknown command, and a typed `<?` would look like the
        // renderer's marker for a missing argument.
        2 => "[^<\\pC]{0,12}".prop_map(Input::Line),
    ]
}

/// How far the game has been played by commands before the random part begins.
fn start() -> impl Strategy<Value = u8> {
    prop_oneof![Just(0u8), Just(1), Just(2)]
}

fn begin(depth: u8) -> Driver {
    let mut driver = Driver::new();
    match depth {
        0 => {}
        1 => driver.finish_m01(),
        _ => driver.finish_m02(),
    }
    driver
}

// ------------------------------------------------------------------------------- helpers

/// The most credits a campaign can ever earn: the start purse, every reward, every first
/// breach and every grant of an effect, whatever the conditions. Nothing the player types can
/// put more in the purse than the data allows.
fn credit_ceiling(c: &Content) -> u64 {
    let mut total = u64::from(c.rules.start_credits.get());
    let grants = |blocks: &[Block], tier: u8| -> u64 {
        blocks
            .iter()
            .flat_map(|block| &block.then)
            .filter_map(|effect| match effect {
                Effect::Grant(grant) => grant.credits.map(|size| c.resolve(size, tier).0),
                _ => None,
            })
            .map(|credits| u64::from(credits.get()))
            .sum()
    };
    for quest in &c.quests {
        total += quest
            .reward
            .as_ref()
            .and_then(|reward| reward.credits)
            .map_or(0, |size| u64::from(c.resolve(size, quest.tier).0.get()));
        for blocks in [&quest.on_open, &quest.on_complete, &quest.on_fail] {
            total += grants(blocks, quest.tier);
        }
    }
    for site in &c.sites {
        total += site.first_breach.map_or(0, |size| {
            u64::from(c.resolve(size, site.unlock.tier.unwrap_or(1)).0.get())
        });
    }
    for decision in &c.decisions {
        let tier = c.quest(&decision.quest).map_or(1, |quest| quest.tier);
        for choice in &decision.choice {
            total += grants(&choice.then, tier);
        }
    }
    for topic in &c.topics {
        total += grants(&topic.then, 1);
    }
    total
}

fn no_missing_text(events: &[Event], catalogs: &[Catalog]) -> Result<(), String> {
    for catalog in catalogs {
        for event in events {
            let text = render_event(event, catalog);
            if text.contains("<missing:") || text.contains("<?") {
                return Err(format!("{:?}: {text}", catalog.lang()));
            }
        }
    }
    Ok(())
}

/// The invariants that hold after every step.
fn check(game: &CampaignGame, ceiling: u64) -> Result<(), TestCaseError> {
    let state = game.state();
    let c = game.content();
    prop_assert!(game.notoriety() <= 100);
    prop_assert!(game.credits() <= CREDITS_CAP);
    prop_assert!(
        u64::from(state.missions.earned.get()) <= ceiling,
        "earned more than the data allows"
    );
    prop_assert!(game.credits() <= state.missions.earned.get());
    prop_assert!(!state.missions.overdrawn(c), "the purse went negative");
    Ok(())
}

// ------------------------------------------------------------------------------- properties

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Whatever is typed: no panic, the prompt is the one the state gives, the invariants
    /// hold, every language reads every line, and the game saves and loads back as it was.
    #[test]
    fn random_play_stays_coherent_bounded_and_saveable(
        depth in start(),
        inputs in prop::collection::vec(input(), 0..40),
    ) {
        let catalogs: Vec<Catalog> = Lang::ALL.map(catalog).into();
        let mut driver = begin(depth);
        let ceiling = credit_ceiling(driver.game.content());
        let (mut quests, mut contacts, mut inbox, mut archive) = (
            driver.game.state().quest_log.clone(),
            driver.game.state().contact_book.clone(),
            driver.game.state().inbox.clone(),
            driver.game.state().archive.clone(),
        );
        let mut earned = driver.game.state().missions.earned.get();
        for input in inputs {
            if driver.last.prompt == Prompt::End {
                break;
            }
            driver.send(input);
            prop_assert_eq!(&driver.last.prompt, &driver.game.prompt());
            if let Prompt::Choice(choice) = &driver.last.prompt {
                prop_assert!(choice.cancel.is_some(), "a menu can always be left");
            }
            check(&driver.game, ceiling)?;
            prop_assert!(no_missing_text(&driver.last.events, &catalogs).is_ok(), "{:?}", no_missing_text(&driver.last.events, &catalogs));
            // Numbers never move: the lists only grow, at their end.
            let state = driver.game.state();
            prop_assert!(state.quest_log.starts_with(&quests));
            prop_assert!(state.contact_book.starts_with(&contacts));
            prop_assert!(state.inbox.starts_with(&inbox));
            prop_assert!(state.archive.starts_with(&archive));
            quests.clone_from(&state.quest_log);
            contacts.clone_from(&state.contact_book);
            inbox.clone_from(&state.inbox);
            archive.clone_from(&state.archive);
            prop_assert!(state.missions.earned.get() >= earned, "credits are never taken back");
            earned = state.missions.earned.get();
            // Saves and loads back as it was.
            let text = driver.game.snapshot().unwrap();
            let loaded = CampaignGame::from_save(&text).unwrap();
            prop_assert_eq!(loaded.snapshot().unwrap(), text);
            // A game that is over is saved as it was at its last command: a loaded game is
            // never over.
            if driver.last.prompt != Prompt::End {
                prop_assert_eq!(loaded.prompt(), driver.game.prompt());
            }
        }
    }

    /// The same inputs give the same game, step for step.
    #[test]
    fn the_same_lines_give_the_same_game(
        depth in start(),
        inputs in prop::collection::vec(input(), 0..30),
    ) {
        let (mut a, mut b) = (begin(depth), begin(depth));
        for input in inputs {
            if a.last.prompt == Prompt::End {
                break;
            }
            let (first, second) = (a.game.handle(input.clone()), b.game.handle(input));
            a.last = first.clone();
            prop_assert_eq!(first, second);
        }
        prop_assert_eq!(a.game.snapshot().unwrap(), b.game.snapshot().unwrap());
    }

    /// From any state the player can get back to the command line and leave: a menu or a
    /// question never traps.
    #[test]
    fn the_game_can_always_be_left(
        depth in start(),
        inputs in prop::collection::vec(input(), 0..40),
    ) {
        let mut driver = begin(depth);
        for input in inputs {
            if driver.last.prompt == Prompt::End {
                break;
            }
            driver.send(input);
        }
        if driver.last.prompt != Prompt::End {
            for _ in 0..4 {
                if driver.last.prompt == Prompt::Command {
                    break;
                }
                driver.send(Input::Cancel);
            }
            prop_assert_eq!(&driver.last.prompt, &Prompt::Command, "Escape leaves any menu");
            driver.send(Input::Line("quit".to_owned()));
            driver.send(Input::Confirm(true));
            prop_assert_eq!(&driver.last.prompt, &Prompt::End);
        }
    }

    /// Closing the input ends the game from any state.
    #[test]
    fn the_end_of_the_input_ends_the_game_anywhere(
        depth in start(),
        inputs in prop::collection::vec(input(), 0..30),
    ) {
        let mut driver = begin(depth);
        for input in inputs {
            if driver.last.prompt == Prompt::End {
                break;
            }
            driver.send(input);
        }
        driver.send(Input::Eof);
        prop_assert_eq!(&driver.last.prompt, &Prompt::End);
    }

    /// Hostile saves: a damaged file is refused or loaded as a state the game can play on,
    /// never a panic.
    #[test]
    fn a_hostile_save_never_panics(
        depth in start(),
        seed in any::<u64>(),
        edits in prop::collection::vec(any::<u16>(), 1..6),
        lines in prop::collection::vec(input(), 0..8),
    ) {
        let mut driver = begin(depth);
        driver.lines(&["talk echo7"]);
        let text = driver.game.snapshot().unwrap();
        let hostile = mutate(&text, seed, &edits);
        if let Ok(mut game) = CampaignGame::from_save(&hostile) {
            // A state the loader accepts is one the game can play on and save again.
            let first = game.snapshot().unwrap();
            prop_assert_eq!(CampaignGame::from_save(&first).unwrap().snapshot().unwrap(), first);
            for input in lines {
                if game.prompt() == Prompt::End {
                    break;
                }
                let step = game.handle(input);
                prop_assert_eq!(&step.prompt, &game.prompt());
            }
            game.snapshot().unwrap();
        }
    }
}

/// Damages save text in a repeatable way: lines dropped, doubled, truncated or given a
/// hostile value.
fn mutate(text: &str, seed: u64, edits: &[u16]) -> String {
    const VALUES: &[&str] = &[
        "-1",
        "99999999999999999999",
        "\"x\"",
        "[]",
        "true",
        "{}",
        "\"m01\"",
        "1.5",
        "\"\"",
        "[[1, 2]]",
        "\"a\\u0000b\"",
    ];
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    for (index, edit) in edits.iter().enumerate() {
        if lines.is_empty() {
            break;
        }
        let at = usize::from(*edit) % lines.len();
        let pick = usize::try_from(seed >> (index * 7 % 56)).unwrap_or(0) % VALUES.len();
        match edit % 5 {
            0 => {
                lines.remove(at);
            }
            1 => {
                let copy = lines[at].clone();
                lines.insert(at, copy);
            }
            2 => lines.truncate(at),
            3 => {
                if let Some((key, _)) = lines[at].split_once(" = ") {
                    lines[at] = format!("{key} = {}", VALUES[pick]);
                }
            }
            _ => {
                if let Some(line) = lines.get_mut(at) {
                    let cut = line.len() / 2;
                    if line.is_char_boundary(cut) {
                        line.truncate(cut);
                    }
                }
            }
        }
    }
    lines.join("\n") + "\n"
}

#[test]
fn the_ceiling_is_not_empty_and_a_fresh_game_is_far_below_it() {
    let driver = begin(0);
    let ceiling = credit_ceiling(driver.game.content());
    assert!(ceiling > 5_000, "{ceiling}");
    assert_eq!(driver.credits(), 100);
}

#[test]
fn rendering_a_text_that_is_not_in_the_catalog_is_visible() {
    // The properties above rely on a missing key showing as `<missing:...>`.
    let cat = catalog(Lang::En);
    let text = crate::text::Text::new("campaign.nope");
    assert!(render(&text, &cat, RenderMode::FULL).contains("<missing:"));
    let _ = Difficulty::Normal;
}
