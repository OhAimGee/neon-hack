//! Run model of the spike: graph, ICE families, programs, Trace, patrols.
//!
//! Starting values come from `docs/design/game-systems-audit.md`, section 3.3. Every value
//! or rule that the spike had to choose or change is marked `CHOICE:` in a comment, and
//! listed again in the README.
//!
//! The whole rules engine is [`step_with`]: one function, one code path. `preview`
//! (see [`forecast`]) and resolution both call it (invariant I10).

use std::fmt;

pub const MAX_NODES: usize = 8;
pub const MAX_SENTINELS: usize = 4;
pub const PROGRAM_COUNT: usize = 11;
/// Marker for a program without charge limit in a kit.
pub const UNLIMITED: u8 = u8::MAX;
/// Largest charge count the packed state can hold (2 bits).
pub const MAX_CHARGES: u8 = 3;

/// Trace bands of the audit: 30 vigilance (no mechanical effect), 50 alert, 70 danger.
pub const BAND_ALERT: u8 = 50;
pub const BAND_DANGER: u8 = 70;
/// Trace removed by one Spoof charge.
pub const SPOOF_AMOUNT: u8 = 10;
/// Raw Trace added by one sentinel scan.
pub const SCAN_NOISE: u8 = 5;
/// Raw extra noise per breach in the danger band.
pub const DANGER_EXTRA_NOISE: u8 = 2;
/// Turns a sentinel stays neutralised by a Virus.
pub const VIRUS_TURNS: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family {
    Network,
    Encryption,
    Human,
    AiGuardian,
}

pub const FAMILY_COUNT: usize = 4;

impl Family {
    pub const ALL: [Family; FAMILY_COUNT] = [
        Family::Network,
        Family::Encryption,
        Family::Human,
        Family::AiGuardian,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn name(self) -> &'static str {
        match self {
            Family::Network => "Network",
            Family::Encryption => "Encryption",
            Family::Human => "Human",
            Family::AiGuardian => "AiGuardian",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Program {
    BruteForce,
    Exploit,
    Backdoor,
    Decrypt,
    Quantum,
    Social,
    Virus,
    Spoof,
    Cloak,
    Ghost,
    Overclock,
}

/// Static description of a program (audit table, section 3.3).
#[derive(Clone, Copy, Debug)]
pub struct ProgramSpec {
    pub family: Option<Family>,
    pub power: u8,
    pub cycles: u8,
    pub noise: u8,
    /// Default charges per run ([`UNLIMITED`] for none).
    pub charges: u8,
}

impl Program {
    pub const ALL: [Program; PROGRAM_COUNT] = [
        Program::BruteForce,
        Program::Exploit,
        Program::Backdoor,
        Program::Decrypt,
        Program::Quantum,
        Program::Social,
        Program::Virus,
        Program::Spoof,
        Program::Cloak,
        Program::Ghost,
        Program::Overclock,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn is_breach(self) -> bool {
        (self as usize) <= Program::Virus as usize
    }

    pub const fn name(self) -> &'static str {
        match self {
            Program::BruteForce => "brute",
            Program::Exploit => "exploit",
            Program::Backdoor => "backdoor",
            Program::Decrypt => "decrypt",
            Program::Quantum => "quantum",
            Program::Social => "social",
            Program::Virus => "virus",
            Program::Spoof => "spoof",
            Program::Cloak => "cloak",
            Program::Ghost => "ghost",
            Program::Overclock => "overclock",
        }
    }

    pub const fn spec(self) -> ProgramSpec {
        const fn s(
            family: Option<Family>,
            power: u8,
            cycles: u8,
            noise: u8,
            charges: u8,
        ) -> ProgramSpec {
            ProgramSpec {
                family,
                power,
                cycles,
                noise,
                charges,
            }
        }
        match self {
            Program::BruteForce => s(Some(Family::Network), 2, 1, 5, UNLIMITED),
            Program::Exploit => s(Some(Family::Network), 3, 2, 2, 2),
            // CHOICE: the "next run starts from this node" effect of Backdoor crosses
            // runs, so inside one run it is only a weak, quiet, unlimited breach.
            Program::Backdoor => s(Some(Family::Network), 1, 1, 1, UNLIMITED),
            Program::Decrypt => s(Some(Family::Encryption), 2, 1, 3, UNLIMITED),
            Program::Quantum => s(Some(Family::Encryption), 4, 2, 2, 2),
            Program::Social => s(Some(Family::Human), 3, 1, 1, 2),
            Program::Virus => s(Some(Family::AiGuardian), 2, 2, 4, 1),
            Program::Spoof => s(None, 0, 1, 0, 2),
            Program::Cloak => s(None, 0, 1, 0, 2),
            Program::Ghost => s(None, 0, 1, 0, 1),
            Program::Overclock => s(None, 0, 0, 3, 1),
        }
    }

    /// Power of a breach program against an ICE of `family`: off-family power is 1.
    pub const fn power_against(self, family: Family) -> u8 {
        let spec = self.spec();
        match spec.family {
            Some(f) if f as usize == family as usize => spec.power,
            _ => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loot {
    None,
    /// Required to win: the mission ends with `jackout` once all of them are taken.
    Objective,
    /// Gives the Intel that Social engineering needs.
    Intel,
}

/// Difficulty presets of audit 3.8 (only the two parameters that act inside a run).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Story,
    Normal,
    Expert,
    Hardcore,
}

impl Difficulty {
    /// Trace gain multiplier in quarters (4 = x1).
    pub const fn gain_q4(self) -> u8 {
        match self {
            Difficulty::Story => 2,
            Difficulty::Normal => 4,
            Difficulty::Expert => 5,
            Difficulty::Hardcore => 6,
        }
    }

    pub const fn bonus_cycles(self) -> u8 {
        match self {
            Difficulty::Story => 1,
            _ => 0,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Difficulty::Story => "Story",
            Difficulty::Normal => "Normal",
            Difficulty::Expert => "Expert",
            Difficulty::Hardcore => "Hardcore",
        }
    }
}

/// A Sentinel: static (route of length 1) or patrol (cyclic route, one step per turn).
#[derive(Clone, Debug)]
pub struct SentinelDef {
    /// Cyclic list of nodes; the position during turn `t` is `route[t % len]`.
    pub route: Vec<u8>,
    /// CHOICE: a static sentinel is the process of a guardian node and stops scanning for
    /// good once that node's ICE is cleared. Patrols have no anchor.
    pub anchor: Option<u8>,
}

/// A mission = the static data of one run. Hot fields are plain arrays.
#[derive(Clone, Debug)]
pub struct Mission {
    pub name: String,
    pub tier: u8,
    pub difficulty: Difficulty,
    pub node_names: Vec<&'static str>,
    pub n: usize,
    /// Strength 0 = no ICE on the node.
    pub ice_strength: [u8; MAX_NODES],
    pub ice_family: [Family; MAX_NODES],
    pub loot: [Loot; MAX_NODES],
    pub adj: [u8; MAX_NODES],
    pub dist: [[u8; MAX_NODES]; MAX_NODES],
    pub start: u8,
    pub sentinels: Vec<SentinelDef>,
    /// Per program: 0 = not in the kit, [`UNLIMITED`], or a charge count (1..=3).
    pub kit: [u8; PROGRAM_COUNT],
    pub cycles_per_turn: u8,
    pub gain_q4: u8,
    /// Raw ambient Trace per end of turn.
    pub ambient: u8,
    /// The run is burned when Trace reaches this value (100 in the audit).
    pub trace_cap: u8,
    /// Intel known from the briefing (otherwise it must be looted).
    pub intel_known: bool,
    /// Least common multiple of the route lengths.
    pub period: u8,
    pub objective_mask: u8,
    pub intel_mask: u8,
    /// Apply the canonical action filters of [`legal_actions`] (on by default).
    pub canonical_actions: bool,
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Mission under construction.
#[derive(Clone, Debug)]
pub struct MissionBuilder {
    m: Mission,
    edges: Vec<(u8, u8)>,
    bonus: u8,
}

impl MissionBuilder {
    pub fn new(name: &str, tier: u8, difficulty: Difficulty) -> Self {
        let base_cycles = match tier {
            0..=2 => 3,
            3..=4 => 4,
            _ => 5,
        };
        let m = Mission {
            name: name.to_string(),
            tier,
            difficulty,
            node_names: Vec::new(),
            n: 0,
            ice_strength: [0; MAX_NODES],
            ice_family: [Family::Network; MAX_NODES],
            loot: [Loot::None; MAX_NODES],
            adj: [0; MAX_NODES],
            dist: [[u8::MAX; MAX_NODES]; MAX_NODES],
            start: 0,
            sentinels: Vec::new(),
            kit: [0; PROGRAM_COUNT],
            cycles_per_turn: base_cycles + difficulty.bonus_cycles(),
            gain_q4: difficulty.gain_q4(),
            ambient: 2,
            trace_cap: 100,
            intel_known: false,
            period: 1,
            objective_mask: 0,
            intel_mask: 0,
            canonical_actions: true,
        };
        MissionBuilder {
            m,
            edges: Vec::new(),
            bonus: 0,
        }
    }

    /// Adds `bonus` to the strength of every ICE added from now on (capped at 6): used to
    /// scale a mission up to measure how the search grows with the optimal Trace.
    pub fn strength_bonus(&mut self, bonus: u8) -> &mut Self {
        self.bonus = bonus;
        self
    }

    /// Adds a node and returns its index.
    pub fn node(&mut self, name: &'static str, ice: Option<(Family, u8)>, loot: Loot) -> u8 {
        let i = self.m.n;
        assert!(i < MAX_NODES, "too many nodes");
        self.m.node_names.push(name);
        if let Some((family, strength)) = ice {
            let strength = (strength + self.bonus).min(6);
            assert!((1..=6).contains(&strength), "strength is 1..=6");
            self.m.ice_family[i] = family;
            self.m.ice_strength[i] = strength;
        }
        self.m.loot[i] = loot;
        match loot {
            Loot::Objective => self.m.objective_mask |= 1 << i,
            Loot::Intel => self.m.intel_mask |= 1 << i,
            Loot::None => {}
        }
        self.m.n += 1;
        u8::try_from(i).unwrap_or(0)
    }

    pub fn edge(&mut self, a: u8, b: u8) -> &mut Self {
        self.edges.push((a, b));
        self
    }

    pub fn sentinel(&mut self, route: &[u8], anchor: Option<u8>) -> &mut Self {
        assert!(self.m.sentinels.len() < MAX_SENTINELS, "too many sentinels");
        self.m.sentinels.push(SentinelDef {
            route: route.to_vec(),
            anchor,
        });
        self
    }

    /// Puts a program in the kit with its default charges.
    pub fn program(&mut self, p: Program) -> &mut Self {
        self.m.kit[p.index()] = p.spec().charges;
        self
    }

    pub fn program_charges(&mut self, p: Program, charges: u8) -> &mut Self {
        assert!((1..=MAX_CHARGES).contains(&charges));
        self.m.kit[p.index()] = charges;
        self
    }

    pub fn programs(&mut self, ps: &[Program]) -> &mut Self {
        for &p in ps {
            self.program(p);
        }
        self
    }

    pub fn trace_cap(&mut self, cap: u8) -> &mut Self {
        self.m.trace_cap = cap;
        self
    }

    pub fn intel_known(&mut self, known: bool) -> &mut Self {
        self.m.intel_known = known;
        self
    }

    pub fn cycles(&mut self, cycles: u8) -> &mut Self {
        self.m.cycles_per_turn = cycles;
        self
    }

    pub fn build(&self) -> Mission {
        let mut m = self.m.clone();
        for &(a, b) in &self.edges {
            m.adj[a as usize] |= 1 << b;
            m.adj[b as usize] |= 1 << a;
        }
        // Breadth-first distances.
        for src in 0..m.n {
            m.dist[src][src] = 0;
            let mut frontier = vec![src];
            while let Some(u) = frontier.pop() {
                for v in 0..m.n {
                    if m.adj[u] & (1 << v) != 0 && m.dist[src][v] == u8::MAX {
                        m.dist[src][v] = m.dist[src][u] + 1;
                        frontier.insert(0, v);
                    }
                }
            }
        }
        let mut period = 1u32;
        for s in &m.sentinels {
            let len = u32::try_from(s.route.len()).unwrap_or(1);
            period = period / gcd(period, len) * len;
        }
        assert!(period <= 16, "period must fit 4 bits");
        m.period = u8::try_from(period).unwrap_or(1);
        for (i, &k) in m.kit.iter().enumerate() {
            let p = Program::ALL[i];
            // Spoof must be limited: the solver relies on it for its lower bound.
            assert!(
                p != Program::Spoof || k != UNLIMITED,
                "Spoof must have a finite number of charges"
            );
            assert!(k == 0 || k == UNLIMITED || k <= MAX_CHARGES);
        }
        m
    }
}

impl Mission {
    /// Trace gain after the difficulty multiplier (rounded up: a noise of 1 never vanishes).
    pub fn gain(&self, raw: u8) -> u8 {
        let v = (u16::from(raw) * u16::from(self.gain_q4)).div_ceil(4);
        u8::try_from(v).unwrap_or(u8::MAX)
    }

    /// The same mission with `p` removed from the kit (domination check, invariant I19).
    pub fn without(&self, p: Program) -> Mission {
        let mut m = self.clone();
        m.kit[p.index()] = 0;
        m
    }

    pub fn has(&self, p: Program) -> bool {
        self.kit[p.index()] != 0
    }

    pub fn is_adjacent(&self, a: u8, b: u8) -> bool {
        self.adj[a as usize] & (1 << b) != 0
    }

    pub fn initial(&self) -> State {
        let mut charges = [0u8; PROGRAM_COUNT];
        charges.copy_from_slice(&self.kit);
        State {
            pos: self.start,
            trace: 0,
            cycles: self.cycles_per_turn,
            phase: 0,
            status: Status::Running,
            cloak: false,
            ghost: false,
            loot_taken: 0,
            progress: [0; MAX_NODES],
            neutral: [0; MAX_SENTINELS],
            charges,
        }
    }

    pub fn sentinel_pos(&self, i: usize, phase: u8) -> u8 {
        let r = &self.sentinels[i].route;
        r[usize::from(phase) % r.len()]
    }

    /// A sentinel scans unless it is neutralised or its guardian node has been cleared.
    pub fn sentinel_active(&self, s: &State, i: usize) -> bool {
        if s.neutral[i] > 0 {
            return false;
        }
        match self.sentinels[i].anchor {
            Some(a) => s.progress[usize::from(a)] < self.ice_strength[usize::from(a)],
            None => true,
        }
    }

    pub fn is_cleared(&self, s: &State, node: u8) -> bool {
        let i = usize::from(node);
        s.progress[i] >= self.ice_strength[i]
    }

    pub fn intel_ok(&self, s: &State) -> bool {
        self.intel_known || s.loot_taken & self.intel_mask != 0
    }

    pub fn objectives_done(&self, s: &State) -> bool {
        s.loot_taken & self.objective_mask == self.objective_mask
    }

    /// Information-theoretic size of the state in bits (mixed radix), to compare with
    /// the 84 bits of the fixed packing.
    pub fn state_bits(&self) -> f64 {
        let mut bits = (self.n as f64).log2();
        bits += f64::from(self.trace_cap).log2();
        bits += f64::from(self.cycles_per_turn + 3).log2();
        bits += f64::from(self.period).log2();
        for i in 0..self.n {
            bits += f64::from(self.ice_strength[i] + 1).log2();
            if self.loot[i] != Loot::None {
                bits += 1.0;
            }
        }
        for &k in &self.kit {
            if k != 0 && k != UNLIMITED {
                bits += f64::from(k + 1).log2();
            }
        }
        bits += self.sentinels.len() as f64 * 3f64.log2();
        bits + 2.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Running,
    Won,
    Burned,
}

/// Unpacked state, used by the rules. The solver stores [`State::pack`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct State {
    pub pos: u8,
    pub trace: u8,
    /// Cycles left this turn.
    pub cycles: u8,
    /// Turn number modulo the patrol period.
    pub phase: u8,
    pub status: Status,
    pub cloak: bool,
    pub ghost: bool,
    pub loot_taken: u8,
    pub progress: [u8; MAX_NODES],
    pub neutral: [u8; MAX_SENTINELS],
    pub charges: [u8; PROGRAM_COUNT],
}

// Bit layout of the packed state (84 bits used, bit 127 set so that 0 means "empty").
const POS_SHIFT: u32 = 0;
const TRACE_SHIFT: u32 = 3;
const CYCLES_SHIFT: u32 = 10;
const PHASE_SHIFT: u32 = 14;
const PROGRESS_SHIFT: u32 = 18;
const CHARGES_SHIFT: u32 = 42;
const LOOT_SHIFT: u32 = 64;
const NEUTRAL_SHIFT: u32 = 72;
const CLOAK_SHIFT: u32 = 80;
const GHOST_SHIFT: u32 = 81;
const STATUS_SHIFT: u32 = 82;
const MARKER: u128 = 1 << 127;
pub const PACKED_BITS: u32 = 84;

impl State {
    pub fn pack(&self) -> u128 {
        let mut k = MARKER;
        k |= u128::from(self.pos) << POS_SHIFT;
        k |= u128::from(self.trace) << TRACE_SHIFT;
        k |= u128::from(self.cycles) << CYCLES_SHIFT;
        k |= u128::from(self.phase) << PHASE_SHIFT;
        for (i, &p) in self.progress.iter().enumerate() {
            k |= u128::from(p) << (PROGRESS_SHIFT + 3 * i as u32);
        }
        for (i, &c) in self.charges.iter().enumerate() {
            k |= u128::from(c.min(MAX_CHARGES)) << (CHARGES_SHIFT + 2 * i as u32);
        }
        k |= u128::from(self.loot_taken) << LOOT_SHIFT;
        for (i, &n) in self.neutral.iter().enumerate() {
            k |= u128::from(n) << (NEUTRAL_SHIFT + 2 * i as u32);
        }
        k |= u128::from(self.cloak) << CLOAK_SHIFT;
        k |= u128::from(self.ghost) << GHOST_SHIFT;
        let st = match self.status {
            Status::Running => 0u128,
            Status::Won => 1,
            Status::Burned => 2,
        };
        k | (st << STATUS_SHIFT)
    }

    /// A won run only keeps its Trace: every winning path to the same Trace merges.
    pub fn canonical(&self) -> State {
        if self.status != Status::Won {
            return *self;
        }
        State {
            pos: 0,
            trace: self.trace,
            cycles: 0,
            phase: 0,
            status: Status::Won,
            cloak: false,
            ghost: false,
            loot_taken: 0,
            progress: [0; MAX_NODES],
            neutral: [0; MAX_SENTINELS],
            charges: [0; PROGRAM_COUNT],
        }
    }

    pub fn unpack(m: &Mission, k: u128) -> State {
        let f = |shift: u32, bits: u32| ((k >> shift) & ((1u128 << bits) - 1)) as u8;
        let mut progress = [0u8; MAX_NODES];
        for (i, p) in progress.iter_mut().enumerate() {
            *p = f(PROGRESS_SHIFT + 3 * i as u32, 3);
        }
        let mut charges = [0u8; PROGRAM_COUNT];
        for (i, c) in charges.iter_mut().enumerate() {
            *c = if m.kit[i] == UNLIMITED {
                UNLIMITED
            } else {
                f(CHARGES_SHIFT + 2 * i as u32, 2)
            };
        }
        let mut neutral = [0u8; MAX_SENTINELS];
        for (i, n) in neutral.iter_mut().enumerate() {
            *n = f(NEUTRAL_SHIFT + 2 * i as u32, 2);
        }
        State {
            pos: f(POS_SHIFT, 3),
            trace: f(TRACE_SHIFT, 7),
            cycles: f(CYCLES_SHIFT, 4),
            phase: f(PHASE_SHIFT, 4),
            status: match f(STATUS_SHIFT, 2) {
                0 => Status::Running,
                1 => Status::Won,
                _ => Status::Burned,
            },
            cloak: f(CLOAK_SHIFT, 1) == 1,
            ghost: f(GHOST_SHIFT, 1) == 1,
            loot_taken: f(LOOT_SHIFT, 8),
            progress,
            neutral,
            charges,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    Move(u8),
    Breach { node: u8, program: Program },
    Cloak,
    Spoof,
    Ghost,
    Overclock,
    Loot,
    End,
    Jackout,
}

impl Action {
    /// One-byte code (for plans stored in the arena).
    pub fn encode(self) -> u8 {
        match self {
            Action::Move(n) => n,
            Action::Breach { node, program } => 8 + node * 7 + program.index() as u8,
            Action::Cloak => 64,
            Action::Spoof => 65,
            Action::Ghost => 66,
            Action::Overclock => 67,
            Action::Loot => 68,
            Action::End => 69,
            Action::Jackout => 70,
        }
    }

    pub fn decode(c: u8) -> Action {
        match c {
            0..=7 => Action::Move(c),
            8..=63 => Action::Breach {
                node: (c - 8) / 7,
                program: Program::ALL[usize::from((c - 8) % 7)],
            },
            64 => Action::Cloak,
            65 => Action::Spoof,
            66 => Action::Ghost,
            67 => Action::Overclock,
            68 => Action::Loot,
            69 => Action::End,
            _ => Action::Jackout,
        }
    }

    pub fn describe(self, m: &Mission) -> String {
        let name = |n: u8| m.node_names.get(usize::from(n)).copied().unwrap_or("?");
        match self {
            Action::Move(n) => format!("move {}", name(n)),
            Action::Breach { node, program } => format!("breach {} {}", name(node), program.name()),
            Action::Cloak => "cloak".into(),
            Action::Spoof => "spoof".into(),
            Action::Ghost => "ghost".into(),
            Action::Overclock => "overclock".into(),
            Action::Loot => "loot".into(),
            Action::End => "end".into(),
            Action::Jackout => "jackout".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionError {
    RunOver,
    NotEnoughCycles,
    NotAdjacent,
    IceBlocks,
    NoIce,
    NoCharges,
    NotInKit,
    NeedsIntel,
    NothingToLoot,
    ObjectivesMissing,
    AlreadyActive,
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// What happened, in order. The "Forecast" line of the real game is built from these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Moved {
        to: u8,
    },
    Breached {
        node: u8,
        program: Program,
        power: u8,
        from: u8,
        to: u8,
    },
    Cleared {
        node: u8,
    },
    Noise {
        added: u8,
    },
    Neutralised {
        sentinel: u8,
    },
    CloakOn,
    GhostOn,
    SpoofApplied {
        removed: u8,
    },
    ExtraCycles {
        count: u8,
    },
    Looted {
        node: u8,
    },
    PatrolMoved {
        sentinel: u8,
        from: u8,
        to: u8,
    },
    Scan {
        sentinel: u8,
        from: u8,
        added: u8,
    },
    ScanCancelled {
        sentinel: u8,
        from: u8,
    },
    Ambient {
        added: u8,
    },
    TurnEnded,
    Burned,
    Won {
        trace: u8,
    },
}

pub trait Sink {
    fn event(&mut self, e: Event);
}

impl Sink for () {
    #[inline]
    fn event(&mut self, _e: Event) {}
}

impl Sink for Vec<Event> {
    fn event(&mut self, e: Event) {
        self.push(e);
    }
}

fn add_trace<S: Sink>(m: &Mission, t: &mut State, amount: u8, sink: &mut S) {
    if amount == 0 || t.status != Status::Running {
        return;
    }
    t.trace = t.trace.saturating_add(amount);
    sink.event(Event::Noise { added: amount });
    if t.trace >= m.trace_cap {
        t.status = Status::Burned;
        sink.event(Event::Burned);
    }
}

/// Noise of an action: cancelled by Ghost, +2 per breach in the danger band, scaled.
fn action_noise(m: &Mission, t: &State, raw: u8, breach: bool) -> u8 {
    if t.ghost {
        return 0;
    }
    let extra = if breach && t.trace >= BAND_DANGER {
        DANGER_EXTRA_NOISE
    } else {
        0
    };
    m.gain(raw + extra)
}

fn spend(t: &mut State, cost: u8) -> Result<(), ActionError> {
    if t.cycles < cost {
        return Err(ActionError::NotEnoughCycles);
    }
    t.cycles -= cost;
    Ok(())
}

fn take_charge(m: &Mission, t: &mut State, p: Program) -> Result<(), ActionError> {
    match m.kit[p.index()] {
        0 => Err(ActionError::NotInKit),
        UNLIMITED => Ok(()),
        _ => {
            let c = &mut t.charges[p.index()];
            if *c == 0 {
                return Err(ActionError::NoCharges);
            }
            *c -= 1;
            Ok(())
        }
    }
}

/// The single rules function. Applies `a` to `s`, reports events to `sink`.
///
/// Every legal action is accepted here and only here: the solver's action generator
/// ([`legal_actions`]) is tested against it.
pub fn step_with<S: Sink>(
    m: &Mission,
    s: &State,
    a: Action,
    sink: &mut S,
) -> Result<State, ActionError> {
    if s.status != Status::Running {
        return Err(ActionError::RunOver);
    }
    let mut t = *s;
    match a {
        Action::Move(to) => {
            if !m.is_adjacent(t.pos, to) {
                return Err(ActionError::NotAdjacent);
            }
            if !m.is_cleared(&t, to) {
                return Err(ActionError::IceBlocks);
            }
            spend(&mut t, 1)?;
            t.pos = to;
            sink.event(Event::Moved { to });
        }
        Action::Breach { node, program } => {
            if !program.is_breach() {
                return Err(ActionError::NotInKit);
            }
            if !m.is_adjacent(t.pos, node) {
                return Err(ActionError::NotAdjacent);
            }
            let ni = usize::from(node);
            if m.ice_strength[ni] == 0 || m.is_cleared(&t, node) {
                return Err(ActionError::NoIce);
            }
            if program == Program::Social && !m.intel_ok(&t) {
                return Err(ActionError::NeedsIntel);
            }
            let spec = program.spec();
            spend(&mut t, spec.cycles)?;
            take_charge(m, &mut t, program)?;
            let power = program.power_against(m.ice_family[ni]);
            let from = t.progress[ni];
            let to = (from + power).min(m.ice_strength[ni]);
            t.progress[ni] = to;
            sink.event(Event::Breached {
                node,
                program,
                power,
                from,
                to,
            });
            if to == m.ice_strength[ni] {
                sink.event(Event::Cleared { node });
            }
            if program == Program::Virus {
                // CHOICE: a Virus breach also neutralises every sentinel standing on the
                // target node right now (static guardian process or passing patrol).
                for i in 0..m.sentinels.len() {
                    if m.sentinel_pos(i, t.phase) == node {
                        t.neutral[i] = VIRUS_TURNS;
                        sink.event(Event::Neutralised { sentinel: i as u8 });
                    }
                }
            }
            let noise = action_noise(m, &t, spec.noise, true);
            add_trace(m, &mut t, noise, sink);
        }
        Action::Cloak => {
            if t.cloak {
                return Err(ActionError::AlreadyActive);
            }
            spend(&mut t, Program::Cloak.spec().cycles)?;
            take_charge(m, &mut t, Program::Cloak)?;
            // CHOICE: "cancels the next scan" = every scan of the end of this turn.
            t.cloak = true;
            sink.event(Event::CloakOn);
        }
        Action::Ghost => {
            if t.ghost {
                return Err(ActionError::AlreadyActive);
            }
            spend(&mut t, Program::Ghost.spec().cycles)?;
            take_charge(m, &mut t, Program::Ghost)?;
            // CHOICE: Ghost cancels the noise of the actions that follow in this turn
            // (not the noise already emitted, and not the ambient Trace nor the scans).
            t.ghost = true;
            sink.event(Event::GhostOn);
        }
        Action::Spoof => {
            spend(&mut t, Program::Spoof.spec().cycles)?;
            take_charge(m, &mut t, Program::Spoof)?;
            let removed = t.trace.min(SPOOF_AMOUNT);
            t.trace -= removed;
            sink.event(Event::SpoofApplied { removed });
        }
        Action::Overclock => {
            take_charge(m, &mut t, Program::Overclock)?;
            t.cycles += 2;
            sink.event(Event::ExtraCycles { count: 2 });
            let noise = action_noise(m, &t, Program::Overclock.spec().noise, false);
            add_trace(m, &mut t, noise, sink);
        }
        Action::Loot => {
            let bit = 1u8 << t.pos;
            let has = m.loot[usize::from(t.pos)] != Loot::None;
            if !has || t.loot_taken & bit != 0 {
                return Err(ActionError::NothingToLoot);
            }
            spend(&mut t, 1)?;
            t.loot_taken |= bit;
            sink.event(Event::Looted { node: t.pos });
        }
        Action::Jackout => {
            if !m.objectives_done(&t) {
                return Err(ActionError::ObjectivesMissing);
            }
            t.status = Status::Won;
            sink.event(Event::Won { trace: t.trace });
        }
        Action::End => end_turn(m, &mut t, sink),
    }
    Ok(t)
}

fn end_turn<S: Sink>(m: &Mission, t: &mut State, sink: &mut S) {
    let old_phase = t.phase;
    t.phase = (t.phase + 1) % m.period;
    // 1. Patrols advance.
    for (i, def) in m.sentinels.iter().enumerate() {
        if def.route.len() > 1 {
            sink.event(Event::PatrolMoved {
                sentinel: i as u8,
                from: m.sentinel_pos(i, old_phase),
                to: m.sentinel_pos(i, t.phase),
            });
        }
    }
    // 2. Scans from the new positions. The distance-2 rule reads the Trace as it was
    //    before the scans, so the result does not depend on the sentinel order.
    let trace_before = t.trace;
    for i in 0..m.sentinels.len() {
        if !m.sentinel_active(t, i) {
            continue;
        }
        let from = m.sentinel_pos(i, t.phase);
        let d = m.dist[usize::from(from)][usize::from(t.pos)];
        let sees = d <= 1 || (trace_before >= BAND_ALERT && d <= 2);
        if !sees {
            continue;
        }
        if t.cloak {
            sink.event(Event::ScanCancelled {
                sentinel: i as u8,
                from,
            });
        } else {
            let added = m.gain(SCAN_NOISE);
            sink.event(Event::Scan {
                sentinel: i as u8,
                from,
                added,
            });
            add_trace(m, t, added, sink);
        }
    }
    if t.status != Status::Running {
        return;
    }
    // 3. Neutralisation timers run down after the scan.
    for n in &mut t.neutral {
        *n = n.saturating_sub(1);
    }
    // 4. Ambient Trace: waiting is never free.
    let ambient = m.gain(m.ambient);
    sink.event(Event::Ambient { added: ambient });
    add_trace(m, t, ambient, sink);
    if t.status != Status::Running {
        return;
    }
    t.cloak = false;
    t.ghost = false;
    t.cycles = m.cycles_per_turn;
    sink.event(Event::TurnEnded);
}

/// `step_with` without events.
pub fn step(m: &Mission, s: &State, a: Action) -> Result<State, ActionError> {
    step_with(m, s, a, &mut ())
}

/// Legal actions of a state, in a fixed order.
///
/// CHOICE (canonical actions, provably lossless, checked against the brute force):
/// * Cloak only matters at the end of the turn, so once active only `End` (and `Jackout`)
///   is offered: every plan with an action after a Cloak can put the Cloak last;
/// * Ghost cancels the noise that follows it, so it is only offered as the first action of
///   a turn (a Ghost played later cancels less and costs the same cycle);
/// * Spoof is not offered at Trace 0 (it would do nothing).
pub fn legal_actions(m: &Mission, s: &State, out: &mut Vec<Action>) {
    out.clear();
    if s.status != Status::Running {
        return;
    }
    let pos = s.pos;
    let can = |p: Program| {
        let k = m.kit[p.index()];
        k == UNLIMITED || (k != 0 && s.charges[p.index()] > 0)
    };
    for to in 0..m.n as u8 {
        if m.is_adjacent(pos, to) {
            if m.is_cleared(s, to) {
                if s.cycles >= 1 {
                    out.push(Action::Move(to));
                }
            } else {
                for p in Program::ALL {
                    if !p.is_breach() || !can(p) || s.cycles < p.spec().cycles {
                        continue;
                    }
                    if p == Program::Social && !m.intel_ok(s) {
                        continue;
                    }
                    out.push(Action::Breach {
                        node: to,
                        program: p,
                    });
                }
            }
        }
    }
    if s.cycles >= 1 {
        if can(Program::Cloak) && !s.cloak {
            out.push(Action::Cloak);
        }
        if can(Program::Ghost)
            && !s.ghost
            && (!m.canonical_actions || s.cycles == m.cycles_per_turn)
        {
            out.push(Action::Ghost);
        }
        if can(Program::Spoof) && s.trace > 0 {
            out.push(Action::Spoof);
        }
        let has = m.loot[usize::from(pos)] != Loot::None;
        if has && s.loot_taken & (1 << pos) == 0 {
            out.push(Action::Loot);
        }
    }
    if can(Program::Overclock) {
        out.push(Action::Overclock);
    }
    out.push(Action::End);
    if m.objectives_done(s) {
        out.push(Action::Jackout);
    }
    if m.canonical_actions && s.cloak {
        out.retain(|a| matches!(a, Action::End | Action::Jackout));
    }
}

/// What the player sees before ending the turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forecast {
    /// State after the plan so far, before the turn ends.
    pub after_plan: State,
    /// State after the end of turn (equal to `after_plan` if the plan already ended the run).
    pub after_turn: State,
    /// Events of the end of turn: patrol moves, scans (or cancellations), ambient Trace.
    pub events: Vec<Event>,
}

/// Preview: applies the actions planned so far, then the end of turn, with the same
/// [`step_with`] as the resolution. Nothing is mutated.
pub fn forecast(m: &Mission, s: &State, plan: &[Action]) -> Result<Forecast, ActionError> {
    let mut cur = *s;
    for &a in plan {
        cur = step(m, &cur, a)?;
    }
    let mut events = Vec::new();
    let after_turn = if cur.status == Status::Running {
        step_with(m, &cur, Action::End, &mut events)?
    } else {
        cur
    };
    Ok(Forecast {
        after_plan: cur,
        after_turn,
        events,
    })
}

/// Resolution: applies `plan` and returns the final state and every event.
pub fn resolve(
    m: &Mission,
    s: &State,
    plan: &[Action],
) -> Result<(State, Vec<Event>), ActionError> {
    let mut cur = *s;
    let mut events = Vec::new();
    for &a in plan {
        cur = step_with(m, &cur, a, &mut events)?;
    }
    Ok((cur, events))
}

impl Forecast {
    /// Plain-text "Forecast" line, built from the events only.
    pub fn line(&self, m: &Mission) -> String {
        let name = |n: u8| m.node_names.get(usize::from(n)).copied().unwrap_or("?");
        let mut parts: Vec<String> = Vec::new();
        for e in &self.events {
            match *e {
                Event::PatrolMoved { sentinel, to, .. } => {
                    parts.push(format!("patrol {sentinel} -> {}", name(to)));
                }
                Event::Scan { from, added, .. } => {
                    parts.push(format!("scan from {}: +{added}", name(from)));
                }
                Event::ScanCancelled { from, .. } => {
                    parts.push(format!("scan from {} cancelled", name(from)));
                }
                Event::Ambient { added } => parts.push(format!("ambient +{added}")),
                _ => {}
            }
        }
        parts.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::missions;

    #[test]
    fn pack_roundtrip_on_initial_states() {
        for m in missions::all() {
            let s = m.initial();
            assert_eq!(State::unpack(&m, s.pack()), s, "{}", m.name);
        }
    }

    #[test]
    fn action_codes_roundtrip() {
        let mut v = Vec::new();
        for m in missions::all() {
            let s = m.initial();
            legal_actions(&m, &s, &mut v);
            for &a in &v {
                assert_eq!(Action::decode(a.encode()), a);
            }
        }
        for node in 0..8 {
            for p in Program::ALL.iter().copied().filter(|p| p.is_breach()) {
                let a = Action::Breach { node, program: p };
                assert_eq!(Action::decode(a.encode()), a);
            }
        }
    }

    #[test]
    fn off_family_power_is_one() {
        assert_eq!(Program::BruteForce.power_against(Family::Network), 2);
        assert_eq!(Program::BruteForce.power_against(Family::Human), 1);
        assert_eq!(Program::Quantum.power_against(Family::Encryption), 4);
        assert_eq!(Program::Quantum.power_against(Family::Network), 1);
    }

    #[test]
    fn gain_rounds_up() {
        let mut b = MissionBuilder::new("t", 1, Difficulty::Story);
        b.node("a", None, Loot::Objective);
        let m = b.build();
        assert_eq!(m.gain(5), 3);
        assert_eq!(m.gain(1), 1);
        assert_eq!(m.gain(2), 1);
        assert_eq!(m.gain(0), 0);
    }
}
