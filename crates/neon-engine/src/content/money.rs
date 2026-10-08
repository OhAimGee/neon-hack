//! Saturating amounts: credits and reputation.
//!
//! The audit's invariant I11 asks that unbounded arithmetic on money and reputation does not
//! compile. [`Credits`] and [`Reputation`] wrap the integer, expose only saturating
//! operations, and refuse to be read from a file or a save when they exceed their cap, so a
//! damaged or edited value cannot reach the engine.

use std::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Largest amount of credits: 10^9 (audit I11, `game-systems-audit.md` section 4).
pub const CREDITS_CAP: u32 = 1_000_000_000;

/// Largest absolute value of the reputation: 10^6 (audit I11).
pub const REPUTATION_CAP: i32 = 1_000_000;

/// An amount of credits: never negative, never above [`CREDITS_CAP`].
///
/// Adding saturates at the cap and subtracting stops at zero. Reading a larger value from a
/// file is an error.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Credits(u32);

impl Credits {
    /// No credits.
    pub const ZERO: Self = Self(0);

    /// The cap, [`CREDITS_CAP`].
    pub const CAP: Self = Self(CREDITS_CAP);

    /// An amount, brought down to the cap when it is above.
    #[must_use]
    pub fn new(amount: u32) -> Self {
        Self(amount.min(CREDITS_CAP))
    }

    /// The amount as a number.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }

    /// The sum, stopping at the cap.
    #[must_use]
    pub fn saturating_add(self, other: Self) -> Self {
        Self::new(self.0.saturating_add(other.0))
    }

    /// The difference, stopping at zero.
    #[must_use]
    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// `percent` per cent of the amount, rounded down and stopping at the cap.
    #[must_use]
    pub fn percent(self, percent: u32) -> Self {
        let scaled = u64::from(self.0).saturating_mul(u64::from(percent)) / 100;
        Self::new(u32::try_from(scaled).unwrap_or(u32::MAX))
    }
}

impl fmt::Display for Credits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for Credits {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }
}

impl<'de> Deserialize<'de> for Credits {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let amount = u32::deserialize(deserializer)?;
        if amount > CREDITS_CAP {
            return Err(D::Error::custom(format!(
                "{amount} credits are above the cap of {CREDITS_CAP}"
            )));
        }
        Ok(Self(amount))
    }
}

/// A reputation: positive or negative, between `-`[`REPUTATION_CAP`] and [`REPUTATION_CAP`].
///
/// Adding saturates at both ends. Reading a value outside the range from a file is an error.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reputation(i32);

impl Reputation {
    /// Neutral reputation.
    pub const ZERO: Self = Self(0);

    /// A reputation, brought back into the range when it is outside.
    #[must_use]
    pub fn new(value: i32) -> Self {
        Self(value.clamp(-REPUTATION_CAP, REPUTATION_CAP))
    }

    /// The reputation as a number.
    #[must_use]
    pub fn get(self) -> i32 {
        self.0
    }

    /// The sum, stopping at both ends of the range.
    #[must_use]
    pub fn saturating_add(self, other: Self) -> Self {
        Self::new(self.0.saturating_add(other.0))
    }

    /// The opposite value (a penalty).
    #[must_use]
    pub fn negated(self) -> Self {
        Self(-self.0)
    }
}

impl fmt::Display for Reputation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for Reputation {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i32(self.0)
    }
}

impl<'de> Deserialize<'de> for Reputation {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = i32::deserialize(deserializer)?;
        if !(-REPUTATION_CAP..=REPUTATION_CAP).contains(&value) {
            return Err(D::Error::custom(format!(
                "reputation {value} is outside -{REPUTATION_CAP}..={REPUTATION_CAP}"
            )));
        }
        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests;
