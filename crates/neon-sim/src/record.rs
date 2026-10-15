//! What one game leaves: the record of a campaign played by a bot.
//!
//! The [`Tally`] is the observer the harness gives the driver. It reads only what a player
//! could read (the events of each step, the public view of the game and the intrusion under
//! way), counts it, and never changes the game.

use neon_engine::campaign::{CampaignGame, Difficulty};
use neon_engine::event::{Event, Role};
use neon_engine::prompt::Step;
use neon_engine::text::Arg;
use serde::{Deserialize, Serialize};

use crate::brain::Plan;
use crate::driver::{Config, Driver, Observer};

/// How an intrusion ended, and what it cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrusionRecord {
    /// The site.
    pub site: String,
    /// The turns played (the turn under way when the intrusion ended).
    pub turns: u32,
    /// The Trace at the last turn the harness saw.
    pub trace: u8,
    /// Whether the run burned out.
    pub burned: bool,
}

/// How a game ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum Outcome {
    /// The campaign was played to its epilogue.
    Finished {
        /// The ending reached (`e1a`, `e1b`, `e2`, `e3` or `e4`).
        ending: String,
    },
    /// The bot could not go on.
    Stuck {
        /// Why.
        why: String,
    },
}

/// One game: who played it, how it ended and what it cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRecord {
    /// The name of the plan (decisions and optional objectives) the bot followed.
    pub plan: String,
    /// The difficulty, by its name.
    pub difficulty: String,
    /// The seed of the game.
    pub seed: u64,
    /// How it ended.
    pub outcome: Outcome,
    /// The command lines typed.
    pub commands: u64,
    /// The credits announced as rewards.
    pub credits: i64,
    /// The highest Notoriety met.
    pub max_notoriety: u8,
    /// The intrusions, in order.
    pub intrusions: Vec<IntrusionRecord>,
}

/// The name of a difficulty, as typed on the command line and written in reports.
#[must_use]
pub fn difficulty_name(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Story => "story",
        Difficulty::Normal => "normal",
        Difficulty::Expert => "expert",
        Difficulty::Hardcore => "hardcore",
    }
}

/// The intrusion under way, as last seen.
#[derive(Debug, Clone)]
struct Live {
    site: String,
    turns: u32,
    trace: u8,
}

/// Counts what a game says and does.
#[derive(Debug, Default)]
struct Tally {
    credits: i64,
    ending: Option<String>,
    intrusions: Vec<IntrusionRecord>,
    live: Option<Live>,
    max_notoriety: u8,
}

impl Observer for Tally {
    fn observe(&mut self, game: &mut CampaignGame, _: &str, step: &Step) -> Result<(), String> {
        self.max_notoriety = self.max_notoriety.max(game.notoriety());
        let mut burned = false;
        for event in &step.events {
            let Event::Message { role, text, .. } = event else {
                continue;
            };
            let key = text.key.as_ref();
            if *role == Role::Reward
                && key == "campaign.out.credits"
                && let Some((_, Arg::Int(amount))) =
                    text.args.iter().find(|(name, _)| *name == "amount")
            {
                self.credits += amount;
            }
            if key == "campaign.hack.retry" || key == "campaign.hack.lost" {
                burned = true;
            }
            if let Some(id) = key
                .strip_prefix("ending.")
                .and_then(|rest| rest.strip_suffix(".title"))
            {
                self.ending = Some(id.to_owned());
            }
        }
        if let Some(intrusion) = game.intrusion() {
            self.live = Some(Live {
                site: intrusion.site().to_string(),
                turns: intrusion.state().turn(),
                trace: intrusion.state().trace(),
            });
        } else if let Some(live) = self.live.take() {
            self.intrusions.push(IntrusionRecord {
                site: live.site,
                turns: live.turns,
                trace: live.trace,
                burned,
            });
        }
        Ok(())
    }
}

/// Plays a campaign from a new game to its epilogue with the plan, and records it. A game that
/// cannot go on is a record like any other, with the reason.
#[must_use]
pub fn play_game(plan: &Plan, difficulty: Difficulty, seed: u64) -> GameRecord {
    let mut record = GameRecord {
        plan: plan.name(),
        difficulty: difficulty_name(difficulty).to_owned(),
        seed,
        outcome: Outcome::Stuck {
            why: "not played".to_owned(),
        },
        commands: 0,
        credits: 0,
        max_notoriety: 0,
        intrusions: Vec::new(),
    };
    let game = match CampaignGame::new(difficulty, seed) {
        Ok(game) => game,
        Err(error) => {
            record.outcome = Outcome::Stuck {
                why: format!("a new game does not start: {error}"),
            };
            return record;
        }
    };
    match Driver::new(game, plan.clone(), Tally::default(), Config::default()).play() {
        Ok(walk) => {
            let tally = walk.observer;
            record.commands = walk.typed.len() as u64;
            record.credits = tally.credits;
            record.max_notoriety = tally.max_notoriety;
            record.intrusions = tally.intrusions;
            record.outcome = match tally.ending {
                Some(ending) => Outcome::Finished { ending },
                None => Outcome::Stuck {
                    why: "the epilogue was reached without an ending".to_owned(),
                },
            };
        }
        Err(stuck) => {
            record.outcome = Outcome::Stuck { why: stuck.why };
        }
    }
    record
}
