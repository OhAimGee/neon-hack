//! What a frontend reads of an intrusion: a snapshot, never a stream of events.
//!
//! Every field has its text equivalent in the lines of the game (`map`, `status`, the turn
//! banner and the forecast), which is what lets the plain frontend be the reference and the
//! panel of the full-screen one a mere view (`docs/spec/run-screen.md` § 1).

use crate::game::GaugeReading;
use crate::run::{LootKind, NodeId, Program, Undo};
use crate::text::Text;

/// The cycles of the turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cycles {
    /// Left this turn.
    pub left: u8,
    /// The turn gives this many.
    pub max: u8,
}

/// Where a node stands for the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    /// The player is on it.
    Here,
    /// Nothing guards it, or its ICE is open.
    Open,
    /// Its ICE is up.
    Closed,
}

/// The ICE of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefenseView {
    /// The glossary term of its family (`family_network`...).
    pub family: &'static str,
    /// The progress that opens it.
    pub strength: u8,
    /// The progress made.
    pub progress: u8,
}

/// What a node holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootState {
    /// Nothing.
    None,
    /// To take.
    Visible(LootKind),
    /// Taken already.
    Taken(LootKind),
}

/// One node of the graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeView {
    /// The node: its number is stable during the run.
    pub id: NodeId,
    /// Its name, 10 columns at most.
    pub name: Text,
    /// Where it stands for the player.
    pub state: NodeState,
    /// Its ICE, if it has any.
    pub defense: Option<DefenseView>,
    /// The nodes it is linked to.
    pub links: Vec<NodeId>,
    /// What it holds.
    pub loot: LootState,
}

/// A patrol and where it goes next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatrolView {
    /// Its number, from 1.
    pub id: u8,
    /// Where it stands this turn.
    pub at: NodeId,
    /// Where it stands next turn.
    pub next: NodeId,
    /// The turns it stays down (a Virus put it down), 0 when it scans.
    pub down_turns: u8,
}

/// How many uses of a program are left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Charges {
    /// As many as wanted.
    Unlimited,
    /// This many.
    Left(u8),
}

/// A program of the deck.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramView {
    /// The program.
    pub program: Program,
    /// Its name, 8 columns at most in the panel.
    pub name: Text,
    /// The glossary term of the family it is strong against; `None` for a utility.
    pub family: Option<&'static str>,
    /// Progress it adds against ICE of its family.
    pub power: u8,
    /// Cycles it costs.
    pub cycles: u8,
    /// Trace it makes, with the difficulty applied.
    pub noise: u8,
    /// What is left of it.
    pub charges: Charges,
}

/// An utility that is on until the end of the turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveUtility {
    /// Cloak: the scans of the end of the turn are cancelled.
    Cloak,
    /// Ghost: the actions that follow make no noise.
    Ghost,
}

/// An intrusion, as the interface shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunView {
    /// The site.
    pub site: Text,
    /// The turn, from 1.
    pub turn: u32,
    /// The Trace, with the name of its band.
    pub trace: GaugeReading,
    /// The cycles of the turn.
    pub cycles: Cycles,
    /// The nodes, in the order of their numbers.
    pub nodes: Vec<NodeView>,
    /// The patrols, by number.
    pub patrols: Vec<PatrolView>,
    /// The deck.
    pub deck: Vec<ProgramView>,
    /// Utilities that are on.
    pub active: Vec<ActiveUtility>,
    /// What ending the turn would do, as lines (the same ones the terminal says).
    pub forecast: Vec<Text>,
    /// The Trace ending the turn would add.
    pub forecast_total: i32,
    /// The undos left.
    pub undo_left: Undo,
    /// How much of the loot is taken, and how much there is.
    pub loot_held: (u8, u8),
}
