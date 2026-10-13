//! The tactical intrusion: the pure rules of a run.
//!
//! A run is played on a small graph of nodes. Each node may be guarded by ICE of one family;
//! the player moves from node to node, breaches ICE with the programs of a deck, and makes
//! noise doing so. Patrols walk fixed routes and scan the player at the end of every turn,
//! and the ambient Trace rises every turn: waiting is never free, so every run ends. The run
//! is burned when the Trace reaches its cap.
//!
//! This module is the rules and nothing else: no text, no prompt, no command. The frontends
//! of the game (lots R4.4 and R4.5) turn [`Fact`]s into lines and [`Run`] into a view.
//!
//! * [`Mission`] is the static data of one run, checked once by [`MissionBuilder::build`].
//! * [`RunState`] is everything that changes; it is small, `Copy` and hashable.
//! * [`apply`] is the one rules function: a legal action turns a state into the next one and
//!   says what happened as [`Fact`]s, in the order a player reads them.
//! * [`forecast()`] resolves the end of the turn through that same function (invariant I10),
//!   so what is announced is what happens, to the digit.
//! * [`Run`] is a session: a state, the whole-turn undo, and the counters around them.
//!
//! The values of the programs are those of the audit; the balance is tuned later with the
//! simulator, not here (`docs/spec/run-screen.md`, `docs/spec/solver.md`).

mod action;
mod forecast;
mod mission;
mod program;
mod rules;
mod session;
mod state;

#[cfg(test)]
mod tests;

pub use action::{Action, ActionError, Fact, Sink, Utility};
pub use forecast::{Forecast, ForecastTurn, forecast};
pub use mission::{
    Ice, LootKind, MAX_CYCLES, MAX_NODES, MAX_PATROLS, MAX_STRENGTH, Mission, MissionBuilder,
    MissionError, NodeId, Pace,
};
pub use program::{Family, PROGRAM_COUNT, Program, ProgramSpec, Supply};
pub use rules::{apply, legal_actions};
pub use session::{Run, Undo, UndoError};
pub use state::{RunState, Status, TraceBand};

/// The Trace a Spoof charge erases at most.
pub const SPOOF_AMOUNT: u8 = 10;
/// The raw Trace one patrol scan adds.
pub const SCAN_NOISE: u8 = 5;
/// The raw noise every breach adds in the danger band.
pub const DANGER_EXTRA_NOISE: u8 = 2;
/// The turns a Virus keeps a patrol down.
pub const VIRUS_TURNS: u8 = 2;
/// The cycles an Overclock gives.
pub const OVERCLOCK_CYCLES: u8 = 2;
