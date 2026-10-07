//! The contract between an engine and its frontends.
//!
//! A frontend is a throwaway view: it can be replaced in the middle of a game (the TUI
//! can hand over to the plain frontend) because everything it knows comes from
//! [`Step`]s and [`View`]s. The same action is always the same [`Input`], whether it
//! came from a typed line, a menu click or a shortcut.

use crate::event::Gauge;
use crate::prompt::{Input, Prompt, Step};
use crate::text::Text;

/// One reading of a gauge, for the status bar and the side panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GaugeReading {
    /// Which gauge.
    pub gauge: Gauge,
    /// Current value.
    pub value: i32,
    /// Largest value.
    pub max: i32,
    /// Name of the level the gauge is in. Always shown in words next to the number.
    pub band: Text,
}

/// A read-only snapshot for the status bar and the side panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    /// Name of the player character.
    pub player: String,
    /// The gauges to display.
    pub gauges: Vec<GaugeReading>,
    /// Active objectives, one short text each.
    pub objectives: Vec<Text>,
}

/// A game, as seen by a frontend.
pub trait Game {
    /// The first step of a NEW game: the prologue, then the first prompt. Whoever owns the
    /// game calls it once and hands the step to a frontend; a frontend never calls it itself.
    fn start(&mut self) -> Step;

    /// Advances the game by one input. The only way to make it progress.
    fn handle(&mut self, input: Input) -> Step;

    /// What the game is waiting for, derived from its state (so it is also right just
    /// after a load).
    fn prompt(&self) -> Prompt;

    /// The step to attach a frontend to a game that is already under way: nothing new to
    /// say, just the current prompt. A frontend that takes over from another one (the TUI
    /// handing over to the plain interface) or a loaded game starts from this, not from
    /// [`Game::start`], so the prologue is never replayed.
    fn resume(&self) -> Step {
        Step {
            events: Vec::new(),
            prompt: self.prompt(),
            save_requested: false,
        }
    }

    /// A read-only snapshot for the interface.
    fn view(&self) -> View;

    /// Completions of the text before the cursor, without spoilers: only what the player
    /// could already use.
    fn complete(&self, line: &str) -> Vec<String>;
}
