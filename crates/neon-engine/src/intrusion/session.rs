//! An intrusion under way, with the words: the rules of [`crate::run`] and what is said of them.

use thiserror::Error;

use crate::campaign::Difficulty;
use crate::content::Content;
use crate::content::ids::SiteId;
use crate::event::{Event, Severity};
use crate::run::{Action, Mission, Run, RunState, Status, Undo, UndoError};
use crate::text::Text;

use super::speak::{Voice, key_of};
use super::view::RunView;

/// Why an intrusion cannot begin.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StartError {
    /// No such site.
    #[error("there is no site called `{0}`")]
    UnknownSite(String),
    /// The site has no intrusion (the content tests rule this out for the shipped sites).
    #[error("the site `{0}` has no intrusion")]
    NoIntrusion(String),
}

/// An intrusion on a site.
#[derive(Debug, Clone)]
pub struct Intrusion {
    site: SiteId,
    /// The site as it is written in a text key.
    key: String,
    difficulty: Difficulty,
    run: Run,
}

impl Intrusion {
    /// Begins the intrusion on a site, at the pace and with the undos of the difficulty.
    ///
    /// # Errors
    ///
    /// [`StartError`] when the site does not exist or has no intrusion.
    pub fn new(
        content: &Content,
        site: &SiteId,
        difficulty: Difficulty,
    ) -> Result<Self, StartError> {
        if content.site(site).is_none() {
            return Err(StartError::UnknownSite(site.as_str().to_owned()));
        }
        let mission = content
            .mission(site, difficulty.pace())
            .ok_or_else(|| StartError::NoIntrusion(site.as_str().to_owned()))?;
        Ok(Self::from_mission(site.clone(), difficulty, mission))
    }

    /// An intrusion on a mission built elsewhere (the tests build small ones).
    #[must_use]
    pub fn from_mission(site: SiteId, difficulty: Difficulty, mission: Mission) -> Self {
        Self {
            key: key_of(site.as_str()),
            run: Run::new(mission, difficulty.undo()),
            site,
            difficulty,
        }
    }

    fn voice(&self) -> Voice<'_> {
        Voice {
            mission: self.run.mission(),
            site: &self.key,
            site_id: self.site.as_str(),
            horizon: if self.difficulty == Difficulty::Story {
                2
            } else {
                1
            },
        }
    }

    /// The site.
    #[must_use]
    pub fn site(&self) -> &SiteId {
        &self.site
    }

    /// The difficulty the intrusion is played at.
    #[must_use]
    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    /// The run, for the tests and tools that read an intrusion.
    #[must_use]
    pub fn run(&self) -> &Run {
        &self.run
    }

    /// The state of the run.
    #[must_use]
    pub fn state(&self) -> &RunState {
        self.run.state()
    }

    /// Whether the run is over (won or burned).
    #[must_use]
    pub fn is_over(&self) -> bool {
        self.run.status() != Status::Running
    }

    /// How much loot is still there to be taken.
    #[must_use]
    pub fn loot_left(&self) -> u8 {
        let (taken, total) = self.voice().loot_count(self.run.state());
        total.saturating_sub(taken)
    }

    /// What the intrusion says when it begins: the site, the turn, the forecast.
    #[must_use]
    pub fn opening(&self) -> Vec<Event> {
        let mut events = vec![Event::system(
            Text::new("run.start").with_text("name", Text::raw(self.site.as_str())),
        )];
        events.extend(self.situation());
        events
    }

    /// Where the player stands: the banner of the turn and its forecast. What a game that is
    /// taken over, or loaded, says first; never the opening.
    #[must_use]
    pub fn situation(&self) -> Vec<Event> {
        let voice = self.voice();
        let state = self.run.state();
        let mut events = vec![voice.turn_banner(state)];
        events.extend(voice.forecast_events(state));
        events
    }

    /// Does an action. Returns whether it was done; a refused action says why and changes
    /// nothing.
    pub fn act(&mut self, action: Action, events: &mut Vec<Event>) -> bool {
        let before = *self.run.state();
        match self.run.act(action) {
            Ok(facts) => {
                let after = *self.run.state();
                let voice = self.voice();
                events.extend(if action == Action::End {
                    voice.after_end(&before, &after, &facts)
                } else {
                    voice.after_action(&before, &after, &facts)
                });
                true
            }
            Err(error) => {
                events.push(Voice::refusal(error));
                false
            }
        }
    }

    /// Takes back the turn under way (or the one before).
    pub fn undo(&mut self, events: &mut Vec<Event>) {
        match self.run.undo() {
            Ok(()) => {
                let voice = self.voice();
                let state = self.run.state();
                let done = match self.run.undo_left() {
                    Undo::Unlimited => Text::new("run.undo.done_unlimited"),
                    Undo::Left(count) => Text::new("run.undo.done").with_int("left", i64::from(count)),
                };
                events.push(Event::system(
                    done.with_int("number", i64::from(state.turn()))
                        .with_int("value", voice.shown(state.trace()))
                        .with_int("max", i64::from(voice.mission.trace_cap()))
                        .with_text("band", Voice::band(state.trace(), false)),
                ));
                events.extend(self.situation());
            }
            Err(UndoError::RunOver) => events.push(Event::error(Text::new("run.error.over"))),
            Err(UndoError::NoneLeft) => events.push(Event::error(Text::new("run.undo.none"))),
            Err(UndoError::NothingToTakeBack) => {
                events.push(Event::error(Text::new("run.undo.nothing")));
            }
        }
    }

    /// `map`.
    #[must_use]
    pub fn map(&self) -> Vec<Event> {
        self.voice().map(self.run.state(), self.run.undo_left())
    }

    /// `status`.
    #[must_use]
    pub fn status(&self) -> Vec<Event> {
        self.voice().status(self.run.state(), self.run.undo_left())
    }

    /// The result and the report that close the intrusion; nothing while it is under way.
    #[must_use]
    pub fn closing(&self) -> Vec<Event> {
        let state = self.run.state();
        let voice = self.voice();
        let result = match state.status() {
            Status::Running => return Vec::new(),
            Status::Won => Event::system(
                Text::new("run.result.won").with_text("name", Text::raw(self.site.as_str())),
            ),
            Status::Burned => Event::alert(
                Severity::Danger,
                Text::new("run.result.burned")
                    .with_int("max", i64::from(voice.mission.trace_cap())),
            ),
        };
        vec![result, Event::Screen(voice.report(state))]
    }

    /// What an interface draws its panel from.
    #[must_use]
    pub fn view(&self) -> RunView {
        self.voice().view(self.run.state(), self.run.undo_left())
    }
}
