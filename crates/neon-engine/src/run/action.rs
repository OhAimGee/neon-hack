//! What the player can do, what can go wrong, and what is said when it works.

use thiserror::Error;

use super::mission::NodeId;
use super::program::Program;
use super::state::Status;

/// A utility program: it acts on the run itself, not on the ICE of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Utility {
    /// Erases Trace gained earlier in the turn.
    Spoof,
    /// Cancels the scans of the end of the turn.
    Cloak,
    /// Silences the actions that follow.
    Ghost,
    /// Gives cycles for a little noise.
    Overclock,
}

impl Utility {
    /// Every utility, in a fixed order.
    pub const ALL: [Self; 4] = [Self::Spoof, Self::Cloak, Self::Ghost, Self::Overclock];

    /// The program that gives it.
    #[must_use]
    pub const fn program(self) -> Program {
        match self {
            Self::Spoof => Program::Spoof,
            Self::Cloak => Program::Cloak,
            Self::Ghost => Program::Ghost,
            Self::Overclock => Program::Overclock,
        }
    }

    /// The utility a program gives, if it is one.
    #[must_use]
    pub const fn of(program: Program) -> Option<Self> {
        match program {
            Program::Spoof => Some(Self::Spoof),
            Program::Cloak => Some(Self::Cloak),
            Program::Ghost => Some(Self::Ghost),
            Program::Overclock => Some(Self::Overclock),
            _ => None,
        }
    }
}

/// One thing the player does in a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Moves to a linked node whose ICE is open.
    Move(NodeId),
    /// Breaches the ICE of a linked node with a program.
    Breach {
        /// The node.
        node: NodeId,
        /// The breach program.
        program: Program,
    },
    /// Uses a utility.
    Use(Utility),
    /// Takes the loot of the node the player is on.
    Loot,
    /// Ends the turn: patrols move and scan, the ambient Trace rises.
    End,
    /// Leaves the run, once every objective is taken.
    Jackout,
}

/// Why an action is refused. A refused action changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ActionError {
    /// The run is over.
    #[error("the run is over")]
    RunOver,
    /// The action costs more cycles than are left.
    #[error("not enough cycles left")]
    NotEnoughCycles,
    /// The node is not linked to where the player is.
    #[error("that node is not linked to the current one")]
    NotLinked,
    /// Moving onto a node whose ICE is still up.
    #[error("the ICE of that node is still up")]
    IceBlocks,
    /// Breaching a node that has no ICE, or whose ICE is already open.
    #[error("that node has no ICE left to breach")]
    NoIce,
    /// A utility program named as a breach.
    #[error("{} is not a breach program", .0.id())]
    NotABreach(Program),
    /// The deck does not have the program.
    #[error("{} is not in the deck", .0.id())]
    NotInDeck(Program),
    /// The program has no charge left.
    #[error("{} has no charge left", .0.id())]
    NoCharges(Program),
    /// Social engineering without the intel.
    #[error("social engineering needs intel on the site")]
    NeedsIntel,
    /// Nothing to take here, or already taken.
    #[error("there is nothing to take here")]
    NothingToLoot,
    /// Jacking out before the objective is taken.
    #[error("the objective is not taken yet")]
    ObjectivesMissing,
    /// Cloak or Ghost already on this turn.
    #[error("{} is already active", .0.program().id())]
    AlreadyActive(Utility),
    /// Spoof when the turn has made no Trace to erase.
    #[error("there is no Trace gained this turn to erase")]
    NothingToErase,
}

/// What happened, in the order it happened. The lines of the game are made from these, and
/// so is the forecast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fact {
    /// The player moved.
    Moved {
        /// Where to.
        to: NodeId,
    },
    /// A breach made progress.
    Breached {
        /// The node.
        node: NodeId,
        /// The program.
        program: Program,
        /// What the program is worth against this ICE.
        power: u8,
        /// Progress before.
        from: u8,
        /// Progress after.
        to: u8,
    },
    /// The ICE of a node is open.
    Opened {
        /// The node.
        node: NodeId,
    },
    /// The Trace went up.
    TraceUp {
        /// By how much.
        added: u8,
    },
    /// A Virus put a patrol down.
    PatrolDown {
        /// The patrol.
        patrol: u8,
    },
    /// Cloak is on.
    CloakOn,
    /// Ghost is on.
    GhostOn,
    /// Spoof erased Trace.
    Erased {
        /// By how much.
        removed: u8,
    },
    /// Overclock gave cycles.
    ExtraCycles {
        /// How many.
        count: u8,
    },
    /// The loot of a node was taken.
    Looted {
        /// The node.
        node: NodeId,
    },
    /// A patrol moved at the end of the turn.
    PatrolMoved {
        /// The patrol.
        patrol: u8,
        /// Where it was.
        from: NodeId,
        /// Where it is.
        to: NodeId,
    },
    /// A patrol scanned the player and added Trace.
    Scanned {
        /// The patrol.
        patrol: u8,
        /// The node the patrol scanned from.
        at: NodeId,
        /// The node it scanned: where the player is.
        target: NodeId,
        /// How much Trace.
        added: u8,
    },
    /// A patrol would have scanned the player, and Cloak cancelled it.
    ScanCancelled {
        /// The patrol.
        patrol: u8,
        /// The node the patrol scanned from.
        at: NodeId,
        /// The node it would have scanned: where the player is.
        target: NodeId,
    },
    /// The ambient Trace rose.
    Ambient {
        /// By how much.
        added: u8,
    },
    /// A new turn begins.
    TurnStarted {
        /// Its number.
        turn: u32,
    },
    /// The run ended.
    Ended {
        /// How it ended: never [`Status::Running`].
        status: Status,
    },
}

/// Where facts go. `()` drops them, a `Vec` keeps them.
pub trait Sink {
    /// Receives one fact.
    fn fact(&mut self, fact: Fact);
}

impl Sink for () {
    fn fact(&mut self, _fact: Fact) {}
}

impl Sink for Vec<Fact> {
    fn fact(&mut self, fact: Fact) {
        self.push(fact);
    }
}
