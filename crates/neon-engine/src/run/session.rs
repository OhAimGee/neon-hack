//! A run under way: the state, and the undo of whole turns.

use thiserror::Error;

use super::action::{Action, ActionError, Fact};
use super::forecast::{Forecast, forecast};
use super::mission::Mission;
use super::rules::{apply, legal_actions};
use super::state::{RunState, Status};

/// How many turns the player can still take back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Undo {
    /// As many as wanted.
    Unlimited,
    /// This many; `Left(0)` is none.
    Left(u8),
}

impl Undo {
    /// No undo at all.
    pub const NONE: Self = Self::Left(0);

    /// Whether one can be taken now.
    #[must_use]
    pub fn is_available(self) -> bool {
        !matches!(self, Self::Left(0))
    }

    fn spend(self) -> Self {
        match self {
            Self::Unlimited => Self::Unlimited,
            Self::Left(count) => Self::Left(count.saturating_sub(1)),
        }
    }
}

/// Why an undo is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum UndoError {
    /// The run is over.
    #[error("the run is over")]
    RunOver,
    /// No undo left.
    #[error("no undo left")]
    NoneLeft,
    /// Nothing has been done since the run began.
    #[error("nothing to take back")]
    NothingToTakeBack,
}

/// A run: a mission, its state, and the memory the undo needs.
///
/// `undo` takes back the **whole turn** under way (S-5): the state returns to the start of
/// the turn. With nothing done yet this turn it returns to the start of the one before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    mission: Mission,
    state: RunState,
    /// The state at the start of the turn under way.
    turn_start: RunState,
    /// The starts of the turns before, oldest first.
    earlier: Vec<RunState>,
    acted: bool,
    undo: Undo,
}

impl Run {
    /// Begins a run.
    #[must_use]
    pub fn new(mission: Mission, undo: Undo) -> Self {
        let state = mission.initial_state();
        Self {
            mission,
            state,
            turn_start: state,
            earlier: Vec::new(),
            acted: false,
            undo,
        }
    }

    /// The mission.
    #[must_use]
    pub fn mission(&self) -> &Mission {
        &self.mission
    }

    /// The state.
    #[must_use]
    pub fn state(&self) -> &RunState {
        &self.state
    }

    /// The state at the start of the turn under way.
    #[must_use]
    pub fn turn_start(&self) -> &RunState {
        &self.turn_start
    }

    /// How many undos are left.
    #[must_use]
    pub fn undo_left(&self) -> Undo {
        self.undo
    }

    /// How the run stands.
    #[must_use]
    pub fn status(&self) -> Status {
        self.state.status()
    }

    /// What the rules accept now.
    #[must_use]
    pub fn legal_actions(&self) -> Vec<Action> {
        legal_actions(&self.mission, &self.state)
    }

    /// What the end of the turn would do.
    #[must_use]
    pub fn forecast(&self, turns: u8) -> Forecast {
        forecast(&self.mission, &self.state, turns)
    }

    /// Does an action and returns what happened.
    ///
    /// # Errors
    ///
    /// Returns the [`ActionError`] of a refused action; nothing changes then.
    pub fn act(&mut self, action: Action) -> Result<Vec<Fact>, ActionError> {
        let mut facts = Vec::new();
        let next = apply(&self.mission, &self.state, action, &mut facts)?;
        self.state = next;
        if next.is_running() {
            if action == Action::End {
                self.earlier.push(self.turn_start);
                self.turn_start = next;
                self.acted = false;
            } else {
                self.acted = true;
            }
        }
        Ok(facts)
    }

    /// Takes back the turn under way, or the one before when nothing was done in this one.
    ///
    /// # Errors
    ///
    /// Returns an [`UndoError`] when the run is over, no undo is left, or there is nothing to
    /// take back.
    pub fn undo(&mut self) -> Result<(), UndoError> {
        if !self.state.is_running() {
            return Err(UndoError::RunOver);
        }
        if !self.undo.is_available() {
            return Err(UndoError::NoneLeft);
        }
        if self.acted {
            self.state = self.turn_start;
            self.acted = false;
        } else if let Some(previous) = self.earlier.pop() {
            self.turn_start = previous;
            self.state = previous;
        } else {
            return Err(UndoError::NothingToTakeBack);
        }
        self.undo = self.undo.spend();
        Ok(())
    }
}
