//! The programs of the deck, the families of ICE they fight, and the supply of charges.
//!
//! The values are those of `docs/design/game-systems-audit.md` § 3.3; they are the starting
//! point of the balance (`neon-sim`, lot R6), not its end.

use serde::{Deserialize, Deserializer, de};

/// How many programs there are.
pub const PROGRAM_COUNT: usize = 11;

/// A family of ICE, which decides what is strong against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// Firewalls and routers.
    Network,
    /// Encrypted vaults.
    Encryption,
    /// People: guards, operators.
    Human,
    /// Guardian AIs.
    AiGuardian,
}

impl Family {
    /// Every family, in a fixed order.
    pub const ALL: [Self; 4] = [
        Self::Network,
        Self::Encryption,
        Self::Human,
        Self::AiGuardian,
    ];

    /// Stable identifier, the glossary term of the family.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::Encryption => "encryption",
            Self::Human => "human",
            Self::AiGuardian => "ai_guardian",
        }
    }
}

/// A program. The first seven breach ICE; the last four are utilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Program {
    /// Cheap and loud against networks.
    BruteForce,
    /// Strong and quiet against networks, a few charges.
    Exploit,
    /// A weak, quiet breach (its effect on the next run belongs to the campaign).
    Backdoor,
    /// Steady against encryption.
    Decrypt,
    /// The strongest against encryption, a few charges.
    Quantum,
    /// Against people; needs intel on the site.
    Social,
    /// Against guardian AIs; also neutralises the sentinels standing on the node.
    Virus,
    /// Erases Trace gained earlier in the turn.
    Spoof,
    /// Cancels the scans of the end of the turn.
    Cloak,
    /// Silences the actions that follow in the turn.
    Ghost,
    /// Gives cycles for a little noise.
    Overclock,
}

impl<'de> Deserialize<'de> for Program {
    /// A program is written by its [`Program::id`] in the data.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        Self::from_id(&id).ok_or_else(|| de::Error::custom(format!("unknown program `{id}`")))
    }
}

/// How many uses of a program a run holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Supply {
    /// As many as wanted.
    Unlimited,
    /// This many, from 1 to [`Supply::MAX_LIMITED`].
    Limited(u8),
}

impl Supply {
    /// Most charges a limited supply may hold.
    pub const MAX_LIMITED: u8 = 3;
}

/// What a program is, before any deck or mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramSpec {
    /// The family it is strong against; `None` for a utility.
    pub family: Option<Family>,
    /// Progress it adds against ICE of its family (1 against any other).
    pub power: u8,
    /// Cycles it costs.
    pub cycles: u8,
    /// Raw Trace it makes.
    pub noise: u8,
    /// Charges a deck gives it by default.
    pub supply: Supply,
}

impl Program {
    /// Every program, in a fixed order.
    pub const ALL: [Self; PROGRAM_COUNT] = [
        Self::BruteForce,
        Self::Exploit,
        Self::Backdoor,
        Self::Decrypt,
        Self::Quantum,
        Self::Social,
        Self::Virus,
        Self::Spoof,
        Self::Cloak,
        Self::Ghost,
        Self::Overclock,
    ];

    /// Position in [`Program::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Whether it works on the ICE of a node (and not on the run itself).
    #[must_use]
    pub const fn is_breach(self) -> bool {
        self.index() <= Self::Virus.index()
    }

    /// Stable identifier: what the player types and what the texts are keyed by.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::BruteForce => "brute",
            Self::Exploit => "exploit",
            Self::Backdoor => "backdoor",
            Self::Decrypt => "decrypt",
            Self::Quantum => "quantum",
            Self::Social => "social",
            Self::Virus => "virus",
            Self::Spoof => "spoof",
            Self::Cloak => "cloak",
            Self::Ghost => "ghost",
            Self::Overclock => "overclock",
        }
    }

    /// The program with this identifier.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|program| program.id() == id)
    }

    /// Its starting values.
    #[must_use]
    pub const fn spec(self) -> ProgramSpec {
        const fn make(
            family: Option<Family>,
            power: u8,
            cycles: u8,
            noise: u8,
            supply: Supply,
        ) -> ProgramSpec {
            ProgramSpec {
                family,
                power,
                cycles,
                noise,
                supply,
            }
        }
        match self {
            Self::BruteForce => make(Some(Family::Network), 2, 1, 5, Supply::Unlimited),
            Self::Exploit => make(Some(Family::Network), 3, 2, 2, Supply::Limited(2)),
            Self::Backdoor => make(Some(Family::Network), 1, 1, 1, Supply::Unlimited),
            Self::Decrypt => make(Some(Family::Encryption), 2, 1, 3, Supply::Unlimited),
            Self::Quantum => make(Some(Family::Encryption), 4, 2, 2, Supply::Limited(2)),
            Self::Social => make(Some(Family::Human), 3, 1, 1, Supply::Limited(2)),
            Self::Virus => make(Some(Family::AiGuardian), 2, 2, 4, Supply::Limited(1)),
            Self::Spoof | Self::Cloak => make(None, 0, 1, 0, Supply::Limited(2)),
            Self::Ghost => make(None, 0, 1, 0, Supply::Limited(1)),
            Self::Overclock => make(None, 0, 0, 3, Supply::Limited(1)),
        }
    }

    /// Progress this program makes against ICE of `family`: off its own family, 1.
    #[must_use]
    pub fn power_against(self, family: Family) -> u8 {
        let spec = self.spec();
        match spec.family {
            Some(own) if own == family => spec.power,
            _ => 1,
        }
    }
}
