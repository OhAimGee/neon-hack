//! Reproducible random numbers: a PCG32 generator (XSH RR 64/32).
//!
//! The same seed gives the same sequence on every platform, which is what makes games
//! replayable and tests deterministic.

use serde::{Deserialize, Serialize};

use crate::save::hex_u64;

const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

/// PCG32 random number generator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pcg32 {
    #[serde(with = "hex_u64")]
    state: u64,
    #[serde(with = "hex_u64")]
    inc: u64,
}

impl Pcg32 {
    /// Creates a generator from a seed and a stream selector, like the reference
    /// `pcg32_srandom_r`.
    #[must_use]
    pub fn new(seed: u64, stream: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: (stream << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    /// False for a generator that [`Pcg32::new`] could never have made: the increment of a
    /// PCG generator is always odd. Used to validate a loaded state.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.inc & 1 == 1
    }

    /// Returns the next 32 random bits.
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULTIPLIER).wrapping_add(self.inc);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "PCG32 deliberately keeps only the low 32 bits of the xorshifted state"
        )]
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rotation = (old >> 59) as u32;
        xorshifted.rotate_right(rotation)
    }

    /// Returns a uniformly distributed value in `0..bound`, without modulo bias.
    ///
    /// Returns `None` when `bound` is zero, since the range is then empty.
    pub fn next_below(&mut self, bound: u32) -> Option<u32> {
        if bound == 0 {
            return None;
        }
        // Reject the few low values that would make the result favour small numbers.
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let candidate = self.next_u32();
            if candidate >= threshold {
                return Some(candidate % bound);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn matches_the_reference_sequence() {
        // Reference output of pcg32_srandom_r(42, 54) from the PCG authors' demo program.
        let mut rng = Pcg32::new(42, 54);
        let drawn: Vec<u32> = (0..6).map(|_| rng.next_u32()).collect();
        assert_eq!(
            drawn,
            [
                0xa15c_02b7,
                0x7b47_f409,
                0xba1d_3330,
                0x83d2_f293,
                0xbfa4_784b,
                0xcbed_606e
            ]
        );
    }

    #[test]
    fn empty_range_has_no_value() {
        let mut rng = Pcg32::new(1, 1);
        assert_eq!(rng.next_below(0), None);
    }

    proptest! {
        #[test]
        fn same_seed_same_sequence(seed: u64, stream: u64) {
            let mut a = Pcg32::new(seed, stream);
            let mut b = Pcg32::new(seed, stream);
            for _ in 0..32 {
                prop_assert_eq!(a.next_u32(), b.next_u32());
            }
        }

        #[test]
        fn next_below_stays_in_range(seed: u64, bound in 1u32..=u32::MAX) {
            let mut rng = Pcg32::new(seed, 54);
            for _ in 0..16 {
                let value = rng.next_below(bound);
                prop_assert!(matches!(value, Some(v) if v < bound));
            }
        }
    }
}
