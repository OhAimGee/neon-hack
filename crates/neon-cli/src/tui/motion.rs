//! Staging: the animation that plays when the game tells of an intrusion or of cover bought.
//!
//! The engine has no clock and resolves `hack` at once, so what plays here is a staging of
//! a result that is already known: nothing in it can change the game, and what it shows is
//! said again, in words, by the messages that follow. It is a presentation setting
//! (`--reduce-motion` removes it, a screen reader implies that), it ends by itself within
//! [`MAX_DURATION`], and any key ends it (and is then handled as usual, so that nothing typed
//! is lost).
//!
//! This file is pure: a [`Staging`] is moved forward by durations the caller measures, so
//! the tests step through a whole animation without waiting. Nothing here touches ratatui.

use std::time::Duration;

use neon_engine::event::Effect;

/// Longest a staging lasts (`MOT-3`).
pub(crate) const MAX_DURATION: Duration = Duration::from_millis(2000);
/// Time between two drawings while something moves.
pub(crate) const FRAME: Duration = Duration::from_millis(50);
/// How long the noise keeps one face. Slower than the frames on purpose: the figures change
/// about eight times a second, calmly, never flickering (`MOT-1`).
const NOISE_STEP_MS: u128 = 125;
/// The last stretch of a staging, held on its result.
const HOLD: Duration = Duration::from_millis(300);

/// Where the bar stands, as `(share of the time, share of the bar)`. The intrusion hurries
/// through the scan, lingers on the defences and finishes in a rush.
const HACK_CURVE: [(f64, f64); 6] = [
    (0.0, 0.0),
    (0.12, 0.20),
    (0.52, 0.55),
    (0.70, 0.80),
    (0.84, 0.95),
    (1.0, 1.0),
];
const LAYLOW_CURVE: [(f64, f64); 4] = [(0.0, 0.0), (0.3, 0.35), (0.7, 0.75), (1.0, 1.0)];

/// What a staging needs to be drawn: the effect itself, and the notoriety it moves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Staging {
    effect: Effect,
    /// Notoriety before and after, when the same step told of the change.
    notoriety: Option<(i32, i32)>,
    elapsed: Duration,
}

/// Which step of the story the bar is in, from 1.
pub(crate) type Phase = usize;

impl Staging {
    pub(crate) fn new(effect: Effect, notoriety: Option<(i32, i32)>) -> Self {
        Self {
            effect,
            notoriety,
            elapsed: Duration::ZERO,
        }
    }

    pub(crate) fn effect(&self) -> &Effect {
        &self.effect
    }

    /// How long the whole staging lasts.
    pub(crate) fn duration(&self) -> Duration {
        match self.effect {
            Effect::Hack { .. } => Duration::from_millis(1800),
            Effect::Laylow { .. } => Duration::from_millis(1400),
        }
        .min(MAX_DURATION)
    }

    /// Moves forward. Never past the end.
    pub(crate) fn tick(&mut self, elapsed: Duration) {
        self.elapsed = self.elapsed.saturating_add(elapsed).min(self.duration());
    }

    pub(crate) fn is_over(&self) -> bool {
        self.elapsed >= self.duration()
    }

    fn curve(&self) -> &'static [(f64, f64)] {
        match self.effect {
            Effect::Hack { .. } => &HACK_CURVE,
            Effect::Laylow { .. } => &LAYLOW_CURVE,
        }
    }

    /// The bar, from 0 to 1: never back, 1 only at the end of the curve.
    pub(crate) fn progress(&self) -> f64 {
        let moving = self.duration().saturating_sub(HOLD).as_secs_f64();
        let share = if moving > 0.0 {
            (self.elapsed.as_secs_f64() / moving).min(1.0)
        } else {
            1.0
        };
        let curve = self.curve();
        for pair in curve.windows(2) {
            let &[(t0, p0), (t1, p1)] = pair else {
                continue;
            };
            if share <= t1 {
                return p0 + (p1 - p0) * ((share - t0) / (t1 - t0));
            }
        }
        1.0
    }

    /// The bar as a whole percentage. 100 is reached only when the staging holds its result.
    pub(crate) fn percent(&self) -> u8 {
        // Rounded down, in 0 to 100 by construction of the curve.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a share of a hundred is a small whole number"
        )]
        let percent = (self.progress() * 100.0).floor() as u8;
        percent.min(100)
    }

    /// The step of the story, from 1: the segment of the curve the bar is in.
    pub(crate) fn phase(&self) -> Phase {
        let progress = self.progress();
        let curve = self.curve();
        curve
            .windows(2)
            .position(|pair| pair.get(1).is_some_and(|next| progress < next.1))
            .map_or(curve.len() - 1, |index| index + 1)
    }

    /// How many steps there are.
    #[cfg(test)]
    pub(crate) fn phases(&self) -> Phase {
        self.curve().len() - 1
    }

    /// True once the bar is full: the staging holds its result.
    pub(crate) fn is_done(&self) -> bool {
        self.progress() >= 1.0
    }

    /// The notoriety on its way from before to after, when the step told of it.
    pub(crate) fn notoriety(&self) -> Option<i32> {
        let (from, to) = self.notoriety?;
        let value = f64::from(from) + (f64::from(to) - f64::from(from)) * self.progress();
        // Rounded, between two `i32`s.
        #[allow(
            clippy::cast_possible_truncation,
            reason = "an interpolation between two i32 is an i32"
        )]
        Some(value.round() as i32)
    }

    /// The face of the noise: it changes every [`NOISE_STEP_MS`].
    fn noise_face(&self) -> u64 {
        u64::try_from(self.elapsed.as_millis() / NOISE_STEP_MS).unwrap_or(u64::MAX)
    }

    /// What identifies this staging, so that two intrusions do not show the same noise.
    fn seed(&self) -> u64 {
        match &self.effect {
            Effect::Hack { site, .. } => site.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
            }),
            Effect::Laylow { cooling } => 0x9e37_79b9 ^ u64::from(*cooling),
        }
    }

    /// One row of a memory dump, `width` cells wide: pairs of hexadecimal digits. The part
    /// the bar has already passed has settled into a fixed face (`locked`); the rest is
    /// still running (`live`). Always ASCII, so it is the same with `--ascii`.
    pub(crate) fn noise(&self, row: usize, width: usize) -> Noise {
        let settled = filled(self.progress(), width);
        let face = self.noise_face();
        let mut locked = String::new();
        let mut live = String::new();
        for column in 0..width {
            // A space every third cell makes pairs of digits, like a dump.
            let character = if column % 3 == 2 {
                ' '
            } else if column < settled {
                hex(mix(self.seed(), row as u64, column as u64, 0))
            } else {
                hex(mix(self.seed(), row as u64, column as u64, face + 1))
            };
            if column < settled {
                locked.push(character);
            } else {
                live.push(character);
            }
        }
        Noise { locked, live }
    }
}

/// How many of `cells` cells `progress` (0 to 1) fills, rounded down: a bar is never fuller
/// than the figure next to it.
pub(crate) fn filled(progress: f64, cells: usize) -> usize {
    let cells_f = f64::from(u32::try_from(cells).unwrap_or(u32::MAX));
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a share of a count of cells is a count of cells"
    )]
    let count = (progress.clamp(0.0, 1.0) * cells_f).floor() as usize;
    count.min(cells)
}

/// Where cell number `cell` stands along a row of `cells` cells, from 0 (the first) to 1
/// (the last).
pub(crate) fn along(cell: usize, cells: usize) -> f64 {
    if cells <= 1 {
        return 0.0;
    }
    let to_f64 = |count: usize| f64::from(u32::try_from(count).unwrap_or(u32::MAX));
    to_f64(cell) / to_f64(cells - 1)
}

/// A row of noise, in two parts that each get their own colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Noise {
    pub(crate) locked: String,
    pub(crate) live: String,
}

fn hex(value: u64) -> char {
    char::from_digit(u32::try_from(value % 16).unwrap_or(0), 16)
        .map_or('0', |digit| digit.to_ascii_uppercase())
}

/// A small, well-spread mix of four numbers (splitmix64), so that the noise is the same
/// every time it is asked for the same moment.
fn mix(seed: u64, row: u64, column: u64, face: u64) -> u64 {
    let mut value = seed
        .wrapping_add(row.wrapping_mul(0x9e37_79b9_7f4a_7c15))
        .wrapping_add(column.wrapping_mul(0xbf58_476d_1ce4_e5b9))
        .wrapping_add(face.wrapping_mul(0x94d0_49bb_1331_11eb));
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests;
