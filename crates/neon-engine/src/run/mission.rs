//! The static data of one run, and the builder that checks it.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::program::{Family, PROGRAM_COUNT, Program, Supply};

/// Most nodes a run may have: a node is one bit of several masks.
pub const MAX_NODES: usize = 8;
/// Most patrols a run may have.
pub const MAX_PATROLS: usize = 4;
/// Strongest ICE there can be.
pub const MAX_STRENGTH: u8 = 6;
/// Most cycles a turn may give.
pub const MAX_CYCLES: u8 = 8;
/// Longest period of the patrols (the least common multiple of their route lengths).
const MAX_PERIOD: u32 = 16;
/// Highest Trace cap: the Trace is a `u8` and must never wrap.
const MAX_TRACE_CAP: u8 = 200;
/// The distance of two nodes that are not linked at all.
const FAR: u8 = u8::MAX;

/// A node of the graph of a run: its position in the order the mission lists them.
///
/// Frontends show [`NodeId::number`], which starts at 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(u8);

impl NodeId {
    /// The node at this position, if a run can have so many nodes.
    #[must_use]
    pub fn new(index: u8) -> Option<Self> {
        (usize::from(index) < MAX_NODES).then_some(Self(index))
    }

    /// Position in the list of nodes, from 0.
    #[must_use]
    pub fn index(self) -> usize {
        usize::from(self.0)
    }

    /// The number the player reads and types, from 1.
    #[must_use]
    pub fn number(self) -> u8 {
        self.0.saturating_add(1)
    }

    /// The node with this number (from 1), if a run can have so many nodes.
    #[must_use]
    pub fn from_number(number: u8) -> Option<Self> {
        number.checked_sub(1).and_then(Self::new)
    }

    /// The single bit this node is in the masks.
    pub(super) fn bit(self) -> u8 {
        1 << self.0
    }
}

/// What guards a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ice {
    /// The family of programs that is strong against it.
    pub family: Family,
    /// The progress that opens it, from 1 to [`MAX_STRENGTH`].
    pub strength: u8,
}

/// What a node holds to be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LootKind {
    /// Taking it all is what a run is for: `jackout` is open once it is.
    Objective,
    /// Gives the intel that social engineering needs.
    Intel,
    /// Worth taking, never required.
    Extra,
}

impl LootKind {
    /// Whether the run needs it taken.
    #[must_use]
    pub const fn is_objective(self) -> bool {
        matches!(self, Self::Objective)
    }
}

/// How hard a difficulty preset makes the rules bite: the only two knobs it turns inside a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pace {
    /// The multiplier of every Trace gain, in quarters (4 is plain).
    pub gain_quarters: u8,
    /// Cycles added to every turn.
    pub bonus_cycles: u8,
}

impl Pace {
    /// The rules as written.
    pub const NORMAL: Self = Self {
        gain_quarters: 4,
        bonus_cycles: 0,
    };
}

/// A patrol: a cyclic route walked one step a turn, and what keeps it scanning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Patrol {
    /// During the turn `t` (from the first), it stands on `route[t % len]`.
    pub(super) route: Vec<NodeId>,
    /// A guardian process: it stops scanning for good once the ICE of this node is open.
    pub(super) anchor: Option<NodeId>,
}

/// One node of the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Node {
    pub(super) ice: Option<Ice>,
    pub(super) loot: Option<LootKind>,
}

/// Why a mission cannot be played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MissionError {
    /// A run needs somewhere to start.
    #[error("the mission has no node")]
    NoNode,
    /// More than [`MAX_NODES`].
    #[error("the mission has more than {MAX_NODES} nodes")]
    TooManyNodes,
    /// More than [`MAX_PATROLS`].
    #[error("the mission has more than {MAX_PATROLS} patrols")]
    TooManyPatrols,
    /// A link, a patrol or the start names a node that does not exist.
    #[error("node {0} does not exist")]
    UnknownNode(u8),
    /// A node cannot be linked to itself.
    #[error("node {0} is linked to itself")]
    SelfLink(u8),
    /// ICE has a strength from 1 to [`MAX_STRENGTH`].
    #[error("the ICE of node {0} has a strength out of 1 to {MAX_STRENGTH}")]
    BadStrength(u8),
    /// The player cannot get to this node.
    #[error("node {0} cannot be reached from the start")]
    Unreachable(u8),
    /// Nothing to take that a run is for.
    #[error("the mission has no objective loot")]
    NoObjective,
    /// A patrol walks somewhere.
    #[error("a patrol has an empty route")]
    EmptyRoute,
    /// An anchor is the node of the ICE that a guardian patrol belongs to.
    #[error("the anchor of a patrol, node {0}, has no ICE")]
    AnchorWithoutIce(u8),
    /// The routes repeat together every so many turns, and that number must stay small.
    #[error("the patrol routes repeat only every {0} turns, more than {MAX_PERIOD}")]
    PeriodTooLong(u32),
    /// A limited supply holds 1 to [`Supply::MAX_LIMITED`] charges.
    #[error("{} has a supply out of 1 to {}", .0.id(), Supply::MAX_LIMITED)]
    BadSupply(Program),
    /// Spoof and Overclock must run out: the first would make Trace meaningless, the second
    /// would make cycles so.
    #[error("{} cannot be unlimited", .0.id())]
    UnlimitedUtility(Program),
    /// A turn gives 1 to [`MAX_CYCLES`] cycles.
    #[error("a turn must give 1 to {MAX_CYCLES} cycles")]
    BadCycles,
    /// The gain multiplier is in quarters, from 1 (a quarter) to 8 (double).
    #[error("the Trace gain must be 1 to 8 quarters")]
    BadGain,
    /// Without ambient Trace a run could be waited out for ever.
    #[error("the ambient Trace must be at least 1, or a run might never end")]
    NoAmbient,
    /// The Trace cap is 1 to 200.
    #[error("the Trace cap must be 1 to {MAX_TRACE_CAP}")]
    BadTraceCap,
}

/// The static data of one run: the graph, the patrols, the deck and the pace.
///
/// A mission can only be made by [`MissionBuilder::build`], so every one is playable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mission {
    pub(super) nodes: Vec<Node>,
    /// Per node, the bits of the nodes it is linked to.
    pub(super) links: [u8; MAX_NODES],
    /// Per pair of nodes, the fewest steps between them.
    pub(super) distance: [[u8; MAX_NODES]; MAX_NODES],
    pub(super) start: NodeId,
    pub(super) patrols: Vec<Patrol>,
    pub(super) deck: [Option<Supply>; PROGRAM_COUNT],
    pub(super) cycles_per_turn: u8,
    pub(super) gain_quarters: u8,
    pub(super) ambient: u8,
    pub(super) trace_cap: u8,
    pub(super) intel_known: bool,
    pub(super) period: u8,
    pub(super) objective_mask: u8,
    pub(super) intel_mask: u8,
}

/// The mission under construction. Errors are kept and reported by [`MissionBuilder::build`],
/// so a mission reads as one chain of calls.
#[derive(Debug, Clone)]
pub struct MissionBuilder {
    nodes: Vec<Node>,
    edges: Vec<(u8, u8)>,
    start: u8,
    patrols: Vec<(Vec<u8>, Option<u8>)>,
    deck: [Option<Supply>; PROGRAM_COUNT],
    cycles_per_turn: u8,
    gain_quarters: u8,
    ambient: u8,
    trace_cap: u8,
    intel_known: bool,
    error: Option<MissionError>,
}

impl Default for MissionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MissionBuilder {
    /// An empty mission: 3 cycles a turn, the plain Trace gain, an ambient Trace of 2 and a
    /// cap of 100.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            start: 0,
            patrols: Vec::new(),
            deck: [None; PROGRAM_COUNT],
            cycles_per_turn: 3,
            gain_quarters: 4,
            ambient: 2,
            trace_cap: 100,
            intel_known: false,
            error: None,
        }
    }

    fn fail(&mut self, error: MissionError) {
        self.error.get_or_insert(error);
    }

    /// Adds a node and returns it. Past [`MAX_NODES`] the mission cannot be built.
    pub fn node(&mut self, ice: Option<Ice>, loot: Option<LootKind>) -> NodeId {
        let index = self.nodes.len();
        let Some(id) = u8::try_from(index).ok().and_then(NodeId::new) else {
            self.fail(MissionError::TooManyNodes);
            return NodeId(0);
        };
        if let Some(ice) = ice
            && !(1..=MAX_STRENGTH).contains(&ice.strength)
        {
            self.fail(MissionError::BadStrength(id.0));
        }
        self.nodes.push(Node { ice, loot });
        id
    }

    /// Links two nodes both ways.
    pub fn link(&mut self, a: NodeId, b: NodeId) -> &mut Self {
        self.edges.push((a.0, b.0));
        self
    }

    /// Where the player starts (the first node by default).
    pub fn start(&mut self, node: NodeId) -> &mut Self {
        self.start = node.0;
        self
    }

    /// Adds a patrol that walks `route` one step a turn, starting from its first node. With an
    /// `anchor` it is the guardian process of that node and stops when the node's ICE is open.
    pub fn patrol(&mut self, route: &[NodeId], anchor: Option<NodeId>) -> &mut Self {
        self.patrols.push((
            route.iter().map(|node| node.0).collect(),
            anchor.map(|node| node.0),
        ));
        self
    }

    /// Puts a program in the deck with the charges it has by default.
    pub fn program(&mut self, program: Program) -> &mut Self {
        let supply = program.spec().supply;
        self.program_with(program, supply)
    }

    /// Puts a program in the deck with the given supply.
    pub fn program_with(&mut self, program: Program, supply: Supply) -> &mut Self {
        if let Some(slot) = self.deck.get_mut(program.index()) {
            *slot = Some(supply);
        }
        self
    }

    /// Puts several programs in the deck with their default charges.
    pub fn programs(&mut self, programs: &[Program]) -> &mut Self {
        for &program in programs {
            self.program(program);
        }
        self
    }

    /// Cycles a turn gives.
    pub fn cycles_per_turn(&mut self, cycles: u8) -> &mut Self {
        self.cycles_per_turn = cycles;
        self
    }

    /// The multiplier of every Trace gain, in quarters: 4 is plain, 2 is half, 5 a quarter more.
    pub fn gain_quarters(&mut self, quarters: u8) -> &mut Self {
        self.gain_quarters = quarters;
        self
    }

    /// The raw Trace every end of turn adds.
    pub fn ambient(&mut self, raw: u8) -> &mut Self {
        self.ambient = raw;
        self
    }

    /// The Trace at which the run is burned.
    pub fn trace_cap(&mut self, cap: u8) -> &mut Self {
        self.trace_cap = cap;
        self
    }

    /// Whether the briefing already gave the intel that social engineering needs.
    pub fn intel_known(&mut self, known: bool) -> &mut Self {
        self.intel_known = known;
        self
    }

    /// Checks everything and makes the mission.
    ///
    /// # Errors
    ///
    /// Returns the first [`MissionError`] found: a mission that could not be played is never
    /// made.
    pub fn build(&self) -> Result<Mission, MissionError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        let count = self.nodes.len();
        if count == 0 {
            return Err(MissionError::NoNode);
        }
        let known = |index: u8| {
            if usize::from(index) < count {
                NodeId::new(index).ok_or(MissionError::UnknownNode(index))
            } else {
                Err(MissionError::UnknownNode(index))
            }
        };
        let start = known(self.start)?;
        let mut links = [0u8; MAX_NODES];
        for &(a, b) in &self.edges {
            let (a, b) = (known(a)?, known(b)?);
            if a == b {
                return Err(MissionError::SelfLink(a.0));
            }
            for (from, to) in [(a, b), (b, a)] {
                if let Some(mask) = links.get_mut(from.index()) {
                    *mask |= to.bit();
                }
            }
        }
        let distance = distances(count, links);
        for node in 0..count {
            let reachable = distance
                .get(start.index())
                .and_then(|row| row.get(node))
                .is_some_and(|steps| *steps != FAR);
            if !reachable {
                return Err(MissionError::Unreachable(
                    u8::try_from(node).unwrap_or(u8::MAX),
                ));
            }
        }
        let mut objective_mask = 0u8;
        let mut intel_mask = 0u8;
        for (index, node) in self.nodes.iter().enumerate() {
            let Some(id) = u8::try_from(index).ok().and_then(NodeId::new) else {
                continue;
            };
            match node.loot {
                Some(LootKind::Objective) => objective_mask |= id.bit(),
                Some(LootKind::Intel) => intel_mask |= id.bit(),
                Some(LootKind::Extra) | None => {}
            }
        }
        if objective_mask == 0 {
            return Err(MissionError::NoObjective);
        }
        if self.patrols.len() > MAX_PATROLS {
            return Err(MissionError::TooManyPatrols);
        }
        let mut patrols = Vec::new();
        let mut period = 1u32;
        for (route, anchor) in &self.patrols {
            if route.is_empty() {
                return Err(MissionError::EmptyRoute);
            }
            let route = route
                .iter()
                .map(|&index| known(index))
                .collect::<Result<Vec<_>, _>>()?;
            let anchor = anchor.map(known).transpose()?;
            if let Some(anchor) = anchor
                && self
                    .nodes
                    .get(anchor.index())
                    .is_none_or(|node| node.ice.is_none())
            {
                return Err(MissionError::AnchorWithoutIce(anchor.0));
            }
            let length = u32::try_from(route.len()).unwrap_or(u32::MAX);
            period = lcm(period, length);
            if period > MAX_PERIOD {
                return Err(MissionError::PeriodTooLong(period));
            }
            patrols.push(Patrol { route, anchor });
        }
        for (program, supply) in Program::ALL.into_iter().zip(self.deck) {
            match supply {
                Some(Supply::Limited(charges)) if !(1..=Supply::MAX_LIMITED).contains(&charges) => {
                    return Err(MissionError::BadSupply(program));
                }
                Some(Supply::Unlimited)
                    if matches!(program, Program::Spoof | Program::Overclock) =>
                {
                    return Err(MissionError::UnlimitedUtility(program));
                }
                _ => {}
            }
        }
        if !(1..=MAX_CYCLES).contains(&self.cycles_per_turn) {
            return Err(MissionError::BadCycles);
        }
        if !(1..=8).contains(&self.gain_quarters) {
            return Err(MissionError::BadGain);
        }
        if self.ambient == 0 {
            return Err(MissionError::NoAmbient);
        }
        if !(1..=MAX_TRACE_CAP).contains(&self.trace_cap) {
            return Err(MissionError::BadTraceCap);
        }
        Ok(Mission {
            nodes: self.nodes.clone(),
            links,
            distance,
            start,
            patrols,
            deck: self.deck,
            cycles_per_turn: self.cycles_per_turn,
            gain_quarters: self.gain_quarters,
            ambient: self.ambient,
            trace_cap: self.trace_cap,
            intel_known: self.intel_known,
            period: u8::try_from(period).unwrap_or(1),
            objective_mask,
            intel_mask,
        })
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn lcm(a: u32, b: u32) -> u32 {
    a / gcd(a, b) * b
}

/// The fewest steps between every pair of nodes, by breadth-first search from each.
fn distances(count: usize, links: [u8; MAX_NODES]) -> [[u8; MAX_NODES]; MAX_NODES] {
    let mut distance = [[FAR; MAX_NODES]; MAX_NODES];
    for (source, row) in distance.iter_mut().enumerate().take(count) {
        if let Some(own) = row.get_mut(source) {
            *own = 0;
        }
        let mut queue = std::collections::VecDeque::from([source]);
        while let Some(here) = queue.pop_front() {
            let steps = row.get(here).copied().unwrap_or(FAR);
            let mask = links.get(here).copied().unwrap_or(0);
            for next in 0..count {
                let linked = mask & (1 << next) != 0;
                if let Some(cell) = row.get_mut(next)
                    && linked
                    && *cell == FAR
                {
                    *cell = steps.saturating_add(1);
                    queue.push_back(next);
                }
            }
        }
    }
    distance
}

impl Mission {
    /// The same mission with only the programs that never run out: the *minimal deck* of the
    /// difficulty margin (decision S-10). Everything with a limited supply is left out.
    #[must_use]
    pub fn unlimited_only(&self) -> Self {
        let mut mission = self.clone();
        for slot in &mut mission.deck {
            if matches!(slot, Some(Supply::Limited(_))) {
                *slot = None;
            }
        }
        mission
    }

    /// The same mission where every loot is an objective: a run is won once everything on the
    /// site is taken. A plan for it is a plan for this mission too (it only jacks out last),
    /// which is how an automatic intrusion takes the files the quests ask for.
    #[must_use]
    pub fn taking_everything(&self) -> Self {
        let mut mission = self.clone();
        let mut mask = 0u8;
        for (index, node) in mission.nodes.iter_mut().enumerate() {
            if node.loot.is_some() {
                node.loot = Some(LootKind::Objective);
                if let Some(id) = u8::try_from(index).ok().and_then(NodeId::new) {
                    mask |= id.bit();
                }
            }
        }
        mission.objective_mask = mask;
        mission.intel_mask = 0;
        mission
    }

    /// How many nodes there are.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Every node, in order.
    pub fn nodes(&self) -> impl Iterator<Item = NodeId> + use<> {
        let count = self.nodes.len();
        (0..count).filter_map(|index| u8::try_from(index).ok().and_then(NodeId::new))
    }

    /// Where the player starts.
    #[must_use]
    pub fn start(&self) -> NodeId {
        self.start
    }

    /// The ICE that guards a node, if any.
    #[must_use]
    pub fn ice(&self, node: NodeId) -> Option<Ice> {
        self.nodes.get(node.index()).and_then(|node| node.ice)
    }

    /// What a node holds to be taken, if anything.
    #[must_use]
    pub fn loot(&self, node: NodeId) -> Option<LootKind> {
        self.nodes.get(node.index()).and_then(|node| node.loot)
    }

    /// Whether two nodes are linked.
    #[must_use]
    pub fn is_linked(&self, a: NodeId, b: NodeId) -> bool {
        self.links
            .get(a.index())
            .is_some_and(|mask| mask & b.bit() != 0)
    }

    /// The nodes a node is linked to, in order.
    pub fn neighbours(&self, node: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes()
            .filter(move |other| self.is_linked(node, *other))
    }

    /// The fewest steps from one node to another.
    #[must_use]
    pub fn distance(&self, a: NodeId, b: NodeId) -> u8 {
        self.distance
            .get(a.index())
            .and_then(|row| row.get(b.index()))
            .copied()
            .unwrap_or(FAR)
    }

    /// How many patrols there are.
    #[must_use]
    pub fn patrol_count(&self) -> usize {
        self.patrols.len()
    }

    /// The route of a patrol, from the node it stands on during the first turn.
    #[must_use]
    pub fn patrol_route(&self, patrol: usize) -> &[NodeId] {
        self.patrols.get(patrol).map_or(&[], |patrol| &patrol.route)
    }

    /// The ICE node a patrol is the guardian process of, if any.
    #[must_use]
    pub fn patrol_anchor(&self, patrol: usize) -> Option<NodeId> {
        self.patrols.get(patrol).and_then(|patrol| patrol.anchor)
    }

    /// What the deck gives of a program: `None` when it is not in it.
    #[must_use]
    pub fn supply(&self, program: Program) -> Option<Supply> {
        self.deck.get(program.index()).copied().flatten()
    }

    /// Cycles a turn gives.
    #[must_use]
    pub fn cycles_per_turn(&self) -> u8 {
        self.cycles_per_turn
    }

    /// The Trace at which the run is burned.
    #[must_use]
    pub fn trace_cap(&self) -> u8 {
        self.trace_cap
    }

    /// The raw ambient Trace of an end of turn, before the gain multiplier.
    #[must_use]
    pub fn ambient(&self) -> u8 {
        self.ambient
    }

    /// Whether the briefing gave the intel up front.
    #[must_use]
    pub fn intel_known(&self) -> bool {
        self.intel_known
    }

    /// A Trace gain after the multiplier of the mission, rounded up: a noise of 1 never
    /// vanishes.
    #[must_use]
    pub fn gain(&self, raw: u8) -> u8 {
        let scaled = (u16::from(raw) * u16::from(self.gain_quarters)).div_ceil(4);
        u8::try_from(scaled).unwrap_or(u8::MAX)
    }

    /// Whether the ICE of a node (if any) is open, given the progress made on every node.
    pub(super) fn node_cleared(&self, node: NodeId, progress: [u8; MAX_NODES]) -> bool {
        let needed = self.ice(node).map_or(0, |ice| ice.strength);
        progress.get(node.index()).copied().unwrap_or(0) >= needed
    }
}
