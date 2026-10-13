//! Everything that changes during a run.

use super::mission::{MAX_NODES, MAX_PATROLS, Mission, NodeId};
use super::program::{PROGRAM_COUNT, Program, Supply};

/// How a run stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    /// Under way.
    Running,
    /// The player jacked out with the objective: the run is a success.
    Won,
    /// The Trace reached its cap: the run is lost.
    Burned,
}

/// The named stretches of the Trace gauge (glossary: calm, vigilance, alert, danger).
///
/// Vigilance has no effect, alert lets patrols see two nodes away, danger makes every breach
/// louder. The band after the last, burned, is the [`Status::Burned`] of the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TraceBand {
    /// Below 30.
    Calm,
    /// 30 to 49.
    Vigilance,
    /// 50 to 69.
    Alert,
    /// 70 and more.
    Danger,
}

impl TraceBand {
    /// The Trace at which vigilance starts.
    pub const VIGILANCE_AT: u8 = 30;
    /// The Trace at which patrols see two nodes away.
    pub const ALERT_AT: u8 = 50;
    /// The Trace at which breaches are louder.
    pub const DANGER_AT: u8 = 70;

    /// The band a Trace is in.
    #[must_use]
    pub const fn of(trace: u8) -> Self {
        if trace >= Self::DANGER_AT {
            Self::Danger
        } else if trace >= Self::ALERT_AT {
            Self::Alert
        } else if trace >= Self::VIGILANCE_AT {
            Self::Vigilance
        } else {
            Self::Calm
        }
    }

    /// Stable identifier, the glossary term of the band.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Calm => "calm",
            Self::Vigilance => "vigilance",
            Self::Alert => "alert",
            Self::Danger => "danger",
        }
    }
}

/// The whole state of a run. It is small and `Copy`, so a turn can be remembered by keeping
/// one, and two states are equal exactly when the runs are in the same place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunState {
    pub(super) position: NodeId,
    pub(super) trace: u8,
    /// The Trace at the start of the turn: Spoof never erases below it.
    pub(super) turn_floor: u8,
    pub(super) cycles: u8,
    /// From 1.
    pub(super) turn: u32,
    pub(super) status: Status,
    pub(super) cloak: bool,
    pub(super) ghost: bool,
    pub(super) loot_taken: u8,
    pub(super) progress: [u8; MAX_NODES],
    /// Per patrol, the turns it stays down.
    pub(super) down: [u8; MAX_PATROLS],
    /// Per program, the charges left (only read for limited supplies).
    pub(super) charges: [u8; PROGRAM_COUNT],
}

impl Mission {
    /// The state a run starts in: at the start node, no Trace, a whole turn of cycles.
    #[must_use]
    pub fn initial_state(&self) -> RunState {
        let mut charges = [0u8; PROGRAM_COUNT];
        for (program, slot) in Program::ALL.into_iter().zip(&mut charges) {
            if let Some(Supply::Limited(count)) = self.supply(program) {
                *slot = count;
            }
        }
        RunState {
            position: self.start,
            trace: 0,
            turn_floor: 0,
            cycles: self.cycles_per_turn,
            turn: 1,
            status: Status::Running,
            cloak: false,
            ghost: false,
            loot_taken: 0,
            progress: [0; MAX_NODES],
            down: [0; MAX_PATROLS],
            charges,
        }
    }

    /// The node a patrol stands on during a turn (from 1).
    #[must_use]
    pub fn patrol_position(&self, patrol: usize, turn: u32) -> Option<NodeId> {
        let route = &self.patrols.get(patrol)?.route;
        let length = u32::try_from(route.len())
            .ok()
            .filter(|length| *length > 0)?;
        let step = usize::try_from(turn.saturating_sub(1) % length).ok()?;
        route.get(step).copied()
    }
}

impl RunState {
    /// Where the player is.
    #[must_use]
    pub fn position(&self) -> NodeId {
        self.position
    }

    /// The Trace.
    #[must_use]
    pub fn trace(&self) -> u8 {
        self.trace
    }

    /// The band the Trace is in.
    #[must_use]
    pub fn band(&self) -> TraceBand {
        TraceBand::of(self.trace)
    }

    /// The Trace at the start of the turn: the lowest Spoof can take it to.
    #[must_use]
    pub fn turn_floor(&self) -> u8 {
        self.turn_floor
    }

    /// Cycles left this turn.
    #[must_use]
    pub fn cycles(&self) -> u8 {
        self.cycles
    }

    /// The turn, from 1.
    #[must_use]
    pub fn turn(&self) -> u32 {
        self.turn
    }

    /// How the run stands.
    #[must_use]
    pub fn status(&self) -> Status {
        self.status
    }

    /// Whether the run is still under way.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.status == Status::Running
    }

    /// Whether Cloak will cancel the scans of the end of this turn.
    #[must_use]
    pub fn cloak_active(&self) -> bool {
        self.cloak
    }

    /// Whether Ghost silences the actions that follow.
    #[must_use]
    pub fn ghost_active(&self) -> bool {
        self.ghost
    }

    /// The progress made against the ICE of a node.
    #[must_use]
    pub fn progress(&self, node: NodeId) -> u8 {
        self.progress.get(node.index()).copied().unwrap_or(0)
    }

    /// Whether the ICE of a node (if it has any) is open.
    #[must_use]
    pub fn is_open(&self, mission: &Mission, node: NodeId) -> bool {
        mission.node_cleared(node, self.progress)
    }

    /// Whether the loot of a node has been taken.
    #[must_use]
    pub fn is_looted(&self, node: NodeId) -> bool {
        self.loot_taken & node.bit() != 0
    }

    /// Whether every objective has been taken: `jackout` is open.
    #[must_use]
    pub fn objectives_done(&self, mission: &Mission) -> bool {
        self.loot_taken & mission.objective_mask == mission.objective_mask
    }

    /// Whether the intel that social engineering needs is known.
    #[must_use]
    pub fn has_intel(&self, mission: &Mission) -> bool {
        mission.intel_known || self.loot_taken & mission.intel_mask != 0
    }

    /// What is left of a program: `None` when the deck does not have it.
    #[must_use]
    pub fn supply(&self, mission: &Mission, program: Program) -> Option<Supply> {
        match mission.supply(program)? {
            Supply::Unlimited => Some(Supply::Unlimited),
            Supply::Limited(_) => Some(Supply::Limited(
                self.charges.get(program.index()).copied().unwrap_or(0),
            )),
        }
    }

    /// Whether the deck can give a use of the program now (in the deck, with a charge left).
    #[must_use]
    pub fn can_use(&self, mission: &Mission, program: Program) -> bool {
        !matches!(
            self.supply(mission, program),
            None | Some(Supply::Limited(0))
        )
    }

    /// The turns a patrol stays down (neutralised by a Virus).
    #[must_use]
    pub fn down_turns(&self, patrol: usize) -> u8 {
        self.down.get(patrol).copied().unwrap_or(0)
    }

    /// Where a patrol stands this turn.
    #[must_use]
    pub fn patrol_position(&self, mission: &Mission, patrol: usize) -> Option<NodeId> {
        mission.patrol_position(patrol, self.turn)
    }

    /// Where a patrol will stand next turn.
    #[must_use]
    pub fn patrol_next(&self, mission: &Mission, patrol: usize) -> Option<NodeId> {
        mission.patrol_position(patrol, self.turn.saturating_add(1))
    }

    /// Whether a patrol scans: it is not down, and its guardian node (if it has one) is not
    /// open.
    #[must_use]
    pub fn patrol_scans(&self, mission: &Mission, patrol: usize) -> bool {
        if self.down_turns(patrol) > 0 {
            return false;
        }
        match mission.patrol_anchor(patrol) {
            Some(anchor) => !self.is_open(mission, anchor),
            None => true,
        }
    }
}
