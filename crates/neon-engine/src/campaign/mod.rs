//! The campaign game: the hub of the real game, playable through both frontends.
//!
//! [`CampaignGame`] owns the content ([`Content`]) and a [`CampaignState`] (the mission state
//! of the content language plus what the hub adds), and implements [`Game`] and [`SaveState`]
//! like the demo does. Every command is declared in one table ([`commands`]), opened by data
//! (`data/world/unlocks.toml`), and works on lists with stable numbers ([`resolver`]). What a
//! command changes is a [`Fact`] given to the content language, which answers with
//! [`Outcome`](crate::content::Outcome)s that [`play`] turns into events with derived text keys.
//!
//! Intrusions are resolved by rule, instantly ([`hack`]): the tactical run replaces them in lot
//! R4 without changing the campaign.
//!
//! The game clock is the clock of the mission state: it advances by one turn at every command
//! that changes the world, which then emits [`Fact::Tick`] so that quests opened in a turn can
//! be concluded in the next one.

mod commands;
mod dispatch;
mod hack;
mod hub;
mod keys;
mod play;
mod resolver;
mod state;
mod talk;
mod trade;

pub use commands::command_names;
pub use state::{Band, CampaignState, Difficulty, SiteStatus};

use thiserror::Error;

use crate::command::Availability;
use crate::content::schema::QuestStatus;
use crate::content::{Content, Fact};
use crate::event::{Event, Gauge};
use crate::game::{Game, GaugeReading, View};
use crate::prompt::{Input, Prompt, Step};
use crate::save::{self, SaveError, SaveMeta, SaveRequest};
use crate::text::Text;

use state::embedded_content;

const BANNER: &[&str] = &["== N E O N   H A C K =="];

/// Why a campaign could not be started.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CampaignError {
    /// The embedded content does not load (the tests of the crate make this impossible in a
    /// build that passes them).
    #[error("the campaign content does not load:\n{0}")]
    Content(String),
}

/// The campaign, as a frontend plays it.
#[derive(Debug, Clone)]
pub struct CampaignGame {
    content: &'static Content,
    state: CampaignState,
    /// The game is over (quit, or the input closed). Not saved: a loaded game is never over.
    over: bool,
    /// A checkpoint is wanted at the end of this step (a main quest opened, a decision or an
    /// irreversible purchase is about to be made). Not saved.
    checkpoint: bool,
}

impl CampaignGame {
    /// A new campaign. The first prompt asks for the handle; `seed` is saved for the tactical
    /// run and unused by the hub, which is deterministic.
    ///
    /// # Errors
    ///
    /// [`CampaignError::Content`] when the embedded content does not load.
    pub fn new(difficulty: Difficulty, seed: u64) -> Result<Self, CampaignError> {
        let content = embedded_content().map_err(CampaignError::Content)?;
        Ok(Self {
            content,
            state: CampaignState::new(content, difficulty, seed),
            over: false,
            checkpoint: false,
        })
    }

    /// Loads a campaign from save text.
    ///
    /// # Errors
    ///
    /// [`SaveError`] when the text is damaged, from a newer game, from another game, or not a
    /// state the campaign can be in. A failed load changes nothing: this builds a new game.
    pub fn from_save(text: &str) -> Result<Self, SaveError> {
        let (_, mut state) = save::decode::<CampaignState>(text)?;
        let content = embedded_content().map_err(SaveError::Invalid)?;
        state.sync_lists(content);
        Ok(Self {
            content,
            state,
            over: false,
            checkpoint: false,
        })
    }

    /// The state, for the tests and tools that read a game.
    #[must_use]
    pub fn state(&self) -> &CampaignState {
        &self.state
    }

    /// The content the game runs on.
    #[must_use]
    pub fn content(&self) -> &Content {
        self.content
    }

    /// The notoriety, 0 to 100.
    #[must_use]
    pub fn notoriety(&self) -> u8 {
        self.state.notoriety()
    }

    /// The credits in hand.
    #[must_use]
    pub fn credits(&self) -> u32 {
        self.state.credits(self.content).get()
    }

    /// The game clock, in turns.
    #[must_use]
    pub fn clock(&self) -> u32 {
        self.state.clock()
    }

    /// Builds a step whose prompt is derived from the state, so it can never disagree with
    /// [`Game::prompt`].
    fn step(&self, mut events: Vec<Event>, save: Option<SaveRequest>) -> Step {
        events.iter_mut().for_each(keys::gloss_event);
        Step {
            events,
            prompt: self.prompt(),
            save,
        }
    }

    /// Whether the command is open, or why not (the rules of `unlocks.toml`).
    fn availability(&self, spec: &crate::command::CommandSpec) -> Availability {
        dispatch::availability(self.content, &self.state, spec)
    }

    /// The facts of a turn that changes the world: the turn is the next one of the clock.
    fn tick(&self) -> (u32, Vec<Fact>) {
        let at = self.state.next_turn();
        (at, vec![Fact::Tick { at }])
    }
}

impl Game for CampaignGame {
    fn start(&mut self) -> Step {
        let events = vec![Event::Decor {
            art: BANNER,
            alt: Text::new("campaign.banner.alt"),
        }];
        self.step(events, None)
    }

    fn handle(&mut self, input: Input) -> Step {
        if self.over {
            return self.step(Vec::new(), None);
        }
        if input == Input::Eof {
            self.over = true;
            return self.step(vec![Event::system(Text::new("ui.eof"))], None);
        }
        let before = self.state.clone();
        let mut events = Vec::new();
        let asked = self.advance(&input, &mut events);
        // A checkpoint outranks an autosave: the frontend keeps the newest of each kind apart,
        // and the next change autosaves again.
        let save = if self.checkpoint {
            self.checkpoint = false;
            Some(SaveRequest::Checkpoint)
        } else {
            asked.or_else(|| (self.state != before).then_some(SaveRequest::Autosave))
        };
        self.step(events, save)
    }

    fn prompt(&self) -> Prompt {
        if self.over {
            return Prompt::End;
        }
        let mut prompt = self.flow_prompt();
        keys::gloss_prompt(&mut prompt);
        prompt
    }

    fn resume(&self) -> Step {
        self.step(self.situation(), None)
    }

    fn snapshot(&self) -> Result<String, SaveError> {
        let meta = SaveMeta {
            player: self.state.handle.clone(),
            turn: self.state.clock(),
        };
        save::encode(&meta, &self.state)
    }

    fn view(&self) -> View {
        let notoriety = self.state.notoriety();
        let mut band = play::band_text(Band::of(notoriety));
        keys::gloss(&mut band);
        let mut objectives = self.view_objectives();
        objectives.iter_mut().for_each(keys::gloss);
        View {
            player: self.state.handle.clone(),
            gauges: vec![GaugeReading {
                gauge: Gauge::Notoriety,
                value: i32::from(notoriety),
                max: 100,
                band,
            }],
            objectives,
        }
    }

    fn complete(&self, line: &str) -> Vec<String> {
        self.complete_line(line)
    }
}

impl CampaignGame {
    /// The pending objectives of the active quests, for the side panel: the short title of
    /// each quest, then what is left to do.
    fn view_objectives(&self) -> Vec<Text> {
        let c = self.content;
        let mut lines = Vec::new();
        for id in &self.state.quest_log {
            if self.state.missions.status(id) != QuestStatus::Active {
                continue;
            }
            lines.push(keys::quest_title_short(id));
            for line in crate::content::eval::journal(c, &self.state.missions, id) {
                if line.state == crate::content::eval::ObjectiveState::Pending && !line.optional {
                    lines.push(
                        Text::new("campaign.view.objective")
                            .with_text("text", keys::quest_objective(id, line.index + 1)),
                    );
                }
            }
        }
        lines
    }

    /// A line the game says at the start of a session on a game under way.
    fn situation(&self) -> Vec<Event> {
        if self.state.handle.is_empty() {
            return Vec::new();
        }
        let missions = &self.state.missions;
        let active = self
            .state
            .quest_log
            .iter()
            .filter(|id| missions.status(id) == QuestStatus::Active)
            .count();
        let unread = self
            .state
            .inbox
            .iter()
            .filter(|id| !missions.opened.contains(*id))
            .count();
        let mut events = vec![Event::system(
            Text::new("campaign.resume")
                .with_term("net", "net")
                .with_str("handle", self.state.handle.clone())
                .with_int("active", count(active))
                .with_int("unread", count(unread)),
        )];
        events.extend(self.state.quest_log.iter().filter_map(|id| {
            (missions.status(id) == QuestStatus::Active).then(|| {
                Event::system(
                    Text::new("campaign.situation.active")
                        .with_text("title", keys::quest_title(id)),
                )
            })
        }));
        events
    }
}

/// A count as the `i64` a text argument holds.
fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests;
