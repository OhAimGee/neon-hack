//! The lists an intrusion's commands name things from: the nodes by their number, the programs
//! of the deck by their identifier. A thing that cannot be used by the command stays listed
//! with its reason, so a number never moves.

use crate::command::{ArgKind, CommandSpec, Listing, Resolver, Row};
use crate::run::{Mission, NodeId, Program, RunState, Supply};
use crate::text::Text;

/// The live lists of an intrusion.
pub(super) struct Lists<'a> {
    pub(super) mission: &'a Mission,
    pub(super) state: &'a RunState,
}

impl Resolver for Lists<'_> {
    fn list(&self, kind: ArgKind, command: &CommandSpec) -> Listing {
        match kind {
            ArgKind::Node => Listing {
                see: "map",
                rows: self.nodes(command.name),
            },
            ArgKind::Program => Listing {
                see: "status",
                rows: self.programs(command.name),
            },
            _ => Listing {
                see: "help",
                rows: Vec::new(),
            },
        }
    }
}

impl Lists<'_> {
    fn nodes(&self, command: &str) -> Vec<Row> {
        self.mission
            .nodes()
            .map(|node| {
                let number = node.number().to_string();
                let names = [number.as_str()];
                match self.node_refusal(command, node) {
                    None => Row::open(number.as_str(), &names),
                    Some(reason) => Row::closed(number.as_str(), &names, reason),
                }
            })
            .collect()
    }

    fn node_refusal(&self, command: &str, node: NodeId) -> Option<Text> {
        let (mission, state) = (self.mission, self.state);
        match command {
            "move" if state.position() == node => Some(Text::new("run.reason.here")),
            "move" | "breach" if !mission.is_linked(state.position(), node) => {
                Some(Text::new("run.reason.not_linked"))
            }
            "move" if !state.is_open(mission, node) => Some(Text::new("run.reason.ice_up")),
            "breach" if mission.ice(node).is_none() || state.is_open(mission, node) => {
                Some(Text::new("run.reason.no_ice"))
            }
            _ => None,
        }
    }

    fn programs(&self, command: &str) -> Vec<Row> {
        Program::ALL
            .into_iter()
            .filter(|program| self.state.supply(self.mission, *program).is_some())
            .map(|program| {
                let names = [program.id()];
                match self.program_refusal(command, program) {
                    None => Row::open(program.id(), &names),
                    Some(reason) => Row::closed(program.id(), &names, reason),
                }
            })
            .collect()
    }

    fn program_refusal(&self, command: &str, program: Program) -> Option<Text> {
        let state = self.state;
        match command {
            "breach" if !program.is_breach() => return Some(Text::new("run.reason.not_a_breach")),
            "use" if program.is_breach() => return Some(Text::new("run.reason.not_a_utility")),
            _ => {}
        }
        if matches!(
            state.supply(self.mission, program),
            Some(Supply::Limited(0))
        ) {
            return Some(Text::new("run.reason.no_charges"));
        }
        match program {
            Program::Cloak if state.cloak_active() => Some(Text::new("run.reason.active")),
            Program::Ghost if state.ghost_active() => Some(Text::new("run.reason.active")),
            Program::Spoof if state.trace() <= state.turn_floor() => {
                Some(Text::new("run.reason.nothing_to_erase"))
            }
            Program::Social if !state.has_intel(self.mission) => {
                Some(Text::new("run.reason.needs_intel"))
            }
            _ => None,
        }
    }
}
