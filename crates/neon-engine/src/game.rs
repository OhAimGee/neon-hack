//! The contract between an engine and its frontends.
//!
//! A frontend is a throwaway view: it can be replaced in the middle of a game (the TUI
//! can hand over to the plain frontend) because everything it knows comes from
//! [`Step`]s and [`View`]s. The same action is always the same [`Input`], whether it
//! came from a typed line, a menu click or a shortcut.

use crate::command::Capabilities;
use crate::event::{Gauge, ScreenKind, Table};
use crate::prompt::{Input, Prompt, Step};
use crate::save::SaveError;
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

/// How a row of the side panel stands out. Never the only way to tell: the text of a row says
/// what its tone suggests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Ordinary information.
    Plain,
    /// A name that heads a few rows (a quest above its objectives).
    Strong,
    /// Something went well (a site pierced).
    Good,
    /// Something waits for the player (a message to read).
    Warning,
    /// An empty list, or an aside.
    Muted,
}

/// One row of a section of the side panel: one short text, which the frontend wraps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelRow {
    /// What the row says.
    pub text: Text,
    /// How it stands out.
    pub tone: Tone,
}

impl PanelRow {
    /// A row.
    #[must_use]
    pub fn new(text: Text, tone: Tone) -> Self {
        Self { text, tone }
    }
}

/// A titled list of the side panel: the quests, the net, the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelSection {
    /// The heading of the section.
    pub title: Text,
    /// Its rows, in order.
    pub rows: Vec<PanelRow>,
}

/// A read-only snapshot for the status bar and the side panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    /// Name of the player character.
    pub player: String,
    /// The gauges to display.
    pub gauges: Vec<GaugeReading>,
    /// What the side panel lists below the gauges, in order. Each row is also said by a
    /// command (`quests`, `net`, `shop`, `messages`), so the panel only repeats.
    pub sections: Vec<PanelSection>,
    /// Further items for the status bar, after the gauges (the turn and the cycles of an
    /// intrusion). Each is also said by a command; empty for a game that has none.
    pub status: Vec<Text>,
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
            save: None,
        }
    }

    /// The game as save text, for the frontend to write where the player's request says
    /// (see [`crate::save`]). Loading is the constructor of the concrete game, so that a
    /// failed load can never touch a game that is running.
    ///
    /// # Errors
    ///
    /// [`SaveError`] if the state cannot be written (a programming error).
    fn snapshot(&self) -> Result<String, SaveError>;

    /// Replaces the game under way by the one in `save`, and gives the step to carry on from
    /// (the same as [`Game::resume`] on the loaded game). Loading is the constructor of the
    /// concrete game, so a save that does not load changes nothing: the game is only
    /// replaced once the new one exists. What the frontend told the game about itself
    /// ([`Game::set_capabilities`]) is kept.
    ///
    /// # Errors
    ///
    /// [`SaveError`] if the text is damaged, from a newer game, from another game, or not a
    /// state this game can be in, or if the game cannot be replaced now (the default).
    fn load(&mut self, _save: &str) -> Result<Step, SaveError> {
        Err(SaveError::Invalid(
            "this game cannot load a save while it runs".to_owned(),
        ))
    }

    /// A read-only snapshot for the interface.
    fn view(&self) -> View;

    /// Tells the game what the frontend that runs it can do, so that the commands only that
    /// frontend can run (`panel`) are open, listed and completed. Called once, before the
    /// first prompt is shown. A game that has no such command ignores it.
    fn set_capabilities(&mut self, _capabilities: Capabilities) {}

    /// A screen the game can draw again at any time, as the command that opened it would
    /// show it now. A frontend that keeps a screen open beside the log asks for it after each
    /// step; it changes nothing in the game. `None` when the game has no such screen, or it
    /// cannot be shown now.
    fn screen(&self, _kind: ScreenKind) -> Option<Table> {
        None
    }

    /// Completions of the text before the cursor, without spoilers: only what the player
    /// could already use.
    fn complete(&self, line: &str) -> Vec<String>;
}
