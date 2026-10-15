//! The monkey: inputs at random, valid and hostile, to see that nothing breaks.
//!
//! It plays through the same contract as a person (`Game::handle`) and knows nothing of the
//! campaign: at a command prompt it types a command that `complete` offers, with an argument
//! that `complete` offers, or a hostile line (empty, huge, odd characters, numbers out of
//! range); at a menu, a question or a text prompt it answers well or badly. While it plays it
//! checks what the engine promises whatever it is told:
//!
//! - no panic (invariant I13);
//! - the prompt of a step is the prompt of the game;
//! - a game can be saved and loaded again as it is: `snapshot → load → snapshot` gives the same
//!   text (I3 to I5; a question that waits for a yes or no is the one thing a save leaves out),
//!   and a save that has been damaged is refused without touching the game, or
//!   else loads into a game that holds this promise too;
//! - and at the end, whatever state it left the game in, the game can be left.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use neon_engine::Game;
use neon_engine::campaign::{CampaignGame, Difficulty};
use neon_engine::prompt::{Input, Prompt, Step};
use neon_engine::rng::Pcg32;
use serde::{Deserialize, Serialize};

/// The stream of the generator of the monkey, apart from the one of the game (the game's seed).
const STREAM: u64 = 0x6d6f_6e6b;
/// The inputs a game of the monkey is given, by default, before it leaves.
pub const DEFAULT_STEPS: usize = 300;
/// A save and load round trip is checked every this many inputs.
const ROUNDTRIP_EVERY: usize = 16;
/// A damaged save is tried every this many inputs.
const DAMAGE_EVERY: usize = 48;
/// The inputs the monkey is given to leave a game once it has had enough.
const LEAVE_INPUTS: usize = 60;
/// The inputs kept to say what led to a fault.
const TRAIL: usize = 6;
/// The longest line, in characters, it types.
const HUGE: usize = 4_000;

/// Lines nobody should be able to break the game with.
const HOSTILE: &[&str] = &[
    "",
    "   ",
    "\t",
    "\r",
    "\0",
    "\u{202e}move 1",
    "help help help",
    "move",
    "move -1",
    "move 0",
    "move 99999999999999999999",
    "breach 1",
    "breach 99 nothing",
    "use",
    "use !",
    "buy",
    "buy ../..",
    "talk",
    "talk \"",
    "hack",
    "hack ' OR 1=1",
    "save",
    "load",
    "é\u{301}é日本語🙂",
    "%s%s%s%n",
    "{}{0}{name}",
    "<?ice> <?trace>",
];

/// The characters of the random lines: letters, digits, marks, accents, a script and an emoji.
const ALPHABET: &[char] = &[
    'a', 'b', 'e', 'n', 'o', 'x', 'z', 'A', 'Z', '0', '1', '7', '9', ' ', ' ', '-', '_', '.', '/',
    '\\', '"', '\'', '%', '$', 'é', 'ß', '日', '🙂', '\t',
];

/// What went wrong in a game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "fault", rename_all = "snake_case")]
pub enum Fault {
    /// The engine panicked.
    Panic {
        /// What the panic said.
        message: String,
    },
    /// The game could not be left.
    NotQuittable {
        /// Where it was stuck.
        why: String,
    },
    /// A save that does not give the game it was made from, or a damaged one that changed the
    /// game it was refused by.
    Save {
        /// What differed.
        why: String,
    },
    /// A step that does not say what the game waits for.
    Contract {
        /// What differed.
        why: String,
    },
}

/// One game of the monkey.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonkeyRecord {
    /// The seed of the game (and of the monkey).
    pub seed: u64,
    /// The difficulty of the game, by its name.
    pub difficulty: String,
    /// The inputs sent to the game, the ones that leave it excluded.
    pub inputs: u64,
    /// The inputs that were typed inside an intrusion.
    pub intrusion_inputs: u64,
    /// The damaged saves the game refused.
    pub damaged_refused: u64,
    /// The damaged saves the game took for a valid one (a change that leaves a valid game).
    pub damaged_accepted: u64,
    /// The first thing that went wrong, if anything did.
    pub fault: Option<Fault>,
    /// The last inputs sent (the oldest first), to find what led to the fault.
    pub trail: Vec<String>,
}

/// Plays one game of the monkey: `steps` inputs at most, then it leaves.
#[must_use]
pub fn play_monkey(seed: u64, difficulty: Difficulty, steps: usize) -> MonkeyRecord {
    let mut record = MonkeyRecord {
        seed,
        difficulty: crate::record::difficulty_name(difficulty).to_owned(),
        inputs: 0,
        intrusion_inputs: 0,
        damaged_refused: 0,
        damaged_accepted: 0,
        fault: None,
        trail: Vec::new(),
    };
    let Ok(mut game) = CampaignGame::new(difficulty, seed) else {
        record.fault = Some(Fault::Contract {
            why: "a new game does not start".to_owned(),
        });
        return record;
    };
    let mut rng = Pcg32::new(seed, STREAM);
    let played = catch_unwind(AssertUnwindSafe(|| {
        session(&mut game, &mut rng, steps, &mut record)
    }));
    record.fault = match played {
        Ok(Ok(())) => None,
        Ok(Err(fault)) => Some(fault),
        Err(payload) => Some(Fault::Panic {
            message: panic_message(payload.as_ref()),
        }),
    };
    record
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic with no message".to_owned())
}

fn session(
    game: &mut CampaignGame,
    rng: &mut Pcg32,
    steps: usize,
    record: &mut MonkeyRecord,
) -> Result<(), Fault> {
    let first = game.start();
    agree(game, &first)?;
    for n in 1..=steps {
        if game.prompt() == Prompt::End {
            break;
        }
        let input = next_input(game, rng);
        record
            .trail
            .push(format!("{:?} -> {:?}", kind(&game.prompt()), input));
        if record.trail.len() > TRAIL {
            record.trail.remove(0);
        }
        let step = game.handle(input);
        record.inputs += 1;
        if game.intrusion().is_some() {
            record.intrusion_inputs += 1;
        }
        agree(game, &step)?;
        if n % ROUNDTRIP_EVERY == 0 {
            round_trip(game)?;
        }
        if n % DAMAGE_EVERY == 0 {
            damaged_load(game, rng, record)?;
        }
    }
    leave(game)
}

/// A step says what the game waits for.
fn agree(game: &CampaignGame, step: &Step) -> Result<(), Fault> {
    if step.prompt == game.prompt() {
        Ok(())
    } else {
        Err(Fault::Contract {
            why: format!(
                "the step waits for {}, the game for {}",
                kind(&step.prompt),
                kind(&game.prompt())
            ),
        })
    }
}

fn kind(prompt: &Prompt) -> &'static str {
    match prompt {
        Prompt::Command => "a command",
        Prompt::Choice(_) => "a menu choice",
        Prompt::Text { .. } => "a text",
        Prompt::Confirm { .. } => "a yes or no",
        Prompt::Continue => "a key to go on",
        Prompt::End => "nothing (the end)",
    }
}

// --------------------------------------------------------------------------- saves

/// `snapshot → load → snapshot`: the text does not change, nor what the game waits for.
fn round_trip(game: &mut CampaignGame) -> Result<(), Fault> {
    if game.prompt() == Prompt::End {
        return Ok(());
    }
    let waits = game.prompt();
    // A question the game waits an answer to is not part of a save: it is asked again.
    let saved_whole = !matches!(waits, Prompt::Confirm { .. });
    let text = game.snapshot().map_err(|e| Fault::Save {
        why: format!("a game cannot be saved: {e}"),
    })?;
    let step = game.load(&text).map_err(|e| Fault::Save {
        why: format!("its own save does not load: {e}"),
    })?;
    agree(game, &step)?;
    if saved_whole && game.prompt() != waits {
        return Err(Fault::Save {
            why: format!(
                "after the load the game waits for {}, not for {}",
                kind(&game.prompt()),
                kind(&waits)
            ),
        });
    }
    let again = game.snapshot().map_err(|e| Fault::Save {
        why: format!("a loaded game cannot be saved: {e}"),
    })?;
    if again == text {
        Ok(())
    } else {
        Err(Fault::Save {
            why: "the save of a loaded game is not the save it came from".to_owned(),
        })
    }
}

/// A save that has been damaged: refused, and the game is as it was; or taken, and then the game
/// it makes is a game like any other.
fn damaged_load(
    game: &mut CampaignGame,
    rng: &mut Pcg32,
    record: &mut MonkeyRecord,
) -> Result<(), Fault> {
    if game.prompt() == Prompt::End {
        return Ok(());
    }
    let text = game.snapshot().map_err(|e| Fault::Save {
        why: format!("a game cannot be saved: {e}"),
    })?;
    let damaged = damage(&text, rng);
    match game.load(&damaged) {
        Err(_) => {
            record.damaged_refused += 1;
            let after = game.snapshot().map_err(|e| Fault::Save {
                why: format!("a game cannot be saved after a refused load: {e}"),
            })?;
            if after == text {
                Ok(())
            } else {
                Err(Fault::Save {
                    why: "a save that was refused changed the game".to_owned(),
                })
            }
        }
        Ok(step) => {
            record.damaged_accepted += 1;
            agree(game, &step)?;
            round_trip(game)
        }
    }
}

/// The text of a save, spoiled in one of a few ways.
fn damage(text: &str, rng: &mut Pcg32) -> String {
    let chars: Vec<char> = text.chars().collect();
    let at = below(rng, chars.len());
    match below(rng, 4) {
        // Cut short.
        0 => chars.iter().take(at).collect(),
        // A character missing.
        1 => chars
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != at)
            .map(|(_, c)| *c)
            .collect(),
        // A character changed.
        2 => chars
            .iter()
            .enumerate()
            .map(|(i, c)| if i == at { pick_char(rng) } else { *c })
            .collect(),
        // Some foreign text where the save was.
        _ => {
            let head: String = chars.iter().take(at).collect();
            format!("{head}{}", random_text(rng, 12))
        }
    }
}

// ----------------------------------------------------------------------- the inputs

/// A number below `bound` (0 for none).
fn below(rng: &mut Pcg32, bound: usize) -> usize {
    u32::try_from(bound)
        .ok()
        .and_then(|bound| rng.next_below(bound))
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(0)
}

fn chance(rng: &mut Pcg32, percent: usize) -> bool {
    below(rng, 100) < percent
}

fn pick<'a, T>(rng: &mut Pcg32, items: &'a [T]) -> Option<&'a T> {
    items.get(below(rng, items.len()))
}

fn pick_char(rng: &mut Pcg32) -> char {
    pick(rng, ALPHABET).copied().unwrap_or('?')
}

fn random_text(rng: &mut Pcg32, longest: usize) -> String {
    let length = below(rng, longest + 1);
    (0..length).map(|_| pick_char(rng)).collect()
}

fn next_input(game: &CampaignGame, rng: &mut Pcg32) -> Input {
    match game.prompt() {
        Prompt::Command => Input::Line(command_line(game, rng)),
        Prompt::Choice(choice) => match below(rng, 100) {
            0..=69 => Input::Choice(below(rng, choice.options.len())),
            70..=79 => Input::Line((below(rng, choice.options.len()) + 1).to_string()),
            80..=89 => Input::Line(random_text(rng, 10)),
            90..=94 => Input::Cancel,
            _ => Input::Choice(choice.options.len() + below(rng, 5)),
        },
        Prompt::Confirm { .. } => match below(rng, 100) {
            0..=59 => Input::Confirm(chance(rng, 50)),
            60..=74 => Input::Cancel,
            75..=89 => Input::Line(random_text(rng, 4)),
            _ => Input::Continue,
        },
        Prompt::Text { max_chars, .. } => match below(rng, 100) {
            0..=59 => Input::Line(random_text(rng, max_chars)),
            60..=74 => Input::Line(random_text(rng, max_chars + 40)),
            75..=89 => Input::Line(String::new()),
            _ => Input::Cancel,
        },
        Prompt::Continue => match below(rng, 100) {
            0..=79 => Input::Continue,
            80..=89 => Input::Cancel,
            _ => Input::Line(random_text(rng, 6)),
        },
        // Never asked: the session stops at the end.
        Prompt::End => Input::Eof,
    }
}

/// A line at the command prompt: mostly a command the game offers, with an argument it offers.
/// `quit` and `exit` are left out: the monkey leaves when it has had enough, not by chance.
fn command_line(game: &CampaignGame, rng: &mut Pcg32) -> String {
    let roll = below(rng, 100);
    if roll < 65 {
        let names: Vec<String> = game
            .complete("")
            .into_iter()
            .filter(|name| name != "quit" && name != "exit")
            .collect();
        if let Some(name) = pick(rng, &names) {
            if chance(rng, 10) {
                let hostile = pick(rng, HOSTILE).copied().unwrap_or_default();
                return format!("{name} {hostile}");
            }
            if chance(rng, 75) {
                let args = game.complete(&format!("{name} "));
                if let Some(arg) = pick(rng, &args) {
                    return format!("{name} {arg}");
                }
            }
            return name.clone();
        }
    }
    if roll < 85 {
        return pick(rng, HOSTILE).copied().unwrap_or_default().to_owned();
    }
    if roll < 88 {
        return "x".repeat(HUGE);
    }
    random_text(rng, 40)
}

// ---------------------------------------------------------------------------- leaving

/// Leaves the game the way a person would: `quit` at the command prompt, yes to what is asked,
/// out of the menus, a key to go on.
fn leave(game: &mut CampaignGame) -> Result<(), Fault> {
    for _ in 0..LEAVE_INPUTS {
        let input = match game.prompt() {
            Prompt::End => return Ok(()),
            Prompt::Command => Input::Line("quit".to_owned()),
            Prompt::Confirm { .. } => Input::Confirm(true),
            Prompt::Continue => Input::Continue,
            Prompt::Text { default, .. } => Input::Line(default.unwrap_or_else(|| "x".to_owned())),
            Prompt::Choice(choice) => {
                if choice.cancel.is_some() {
                    Input::Cancel
                } else {
                    let open = choice
                        .options
                        .iter()
                        .position(|option| option.available.is_ok())
                        .unwrap_or(0);
                    Input::Choice(open)
                }
            }
        };
        let step = game.handle(input);
        agree(game, &step)?;
    }
    if game.prompt() == Prompt::End {
        Ok(())
    } else {
        Err(Fault::NotQuittable {
            why: format!(
                "still waiting for {} after {LEAVE_INPUTS} inputs",
                kind(&game.prompt())
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_game_of_the_monkey_is_the_same_game_every_time() {
        let first = play_monkey(3, Difficulty::Normal, 120);
        let second = play_monkey(3, Difficulty::Normal, 120);
        assert_eq!(first, second);
    }

    #[test]
    fn the_monkey_plays_inputs_and_ends_with_nothing_wrong() {
        for (seed, difficulty) in [
            (1, Difficulty::Story),
            (2, Difficulty::Normal),
            (3, Difficulty::Expert),
            (4, Difficulty::Hardcore),
        ] {
            let record = play_monkey(seed, difficulty, 200);
            assert_eq!(record.fault, None, "seed {seed}");
            assert!(record.inputs > 20, "{record:?}");
        }
    }

    #[test]
    fn a_damaged_save_is_shorter_changed_or_longer() {
        let mut rng = Pcg32::new(1, STREAM);
        let text = "0123456789abcdef";
        let damaged: Vec<String> = (0..80).map(|_| damage(text, &mut rng)).collect();
        let length = |d: &String| d.chars().count();
        assert!(damaged.iter().any(|d| length(d) < text.len()));
        assert!(damaged.iter().any(|d| length(d) == text.len() && d != text));
        assert!(damaged.iter().any(|d| length(d) > text.len()));
    }

    #[test]
    fn a_pick_in_nothing_is_nothing_and_a_number_below_zero_is_zero() {
        let mut rng = Pcg32::new(1, STREAM);
        assert_eq!(pick::<u8>(&mut rng, &[]), None);
        assert_eq!(below(&mut rng, 0), 0);
        assert!(below(&mut rng, 5) < 5);
    }
}
