//! The typewriter: what the game just said appears character by character.
//!
//! Like [`super::motion`] this file is pure. A [`Typing`] is moved forward by durations the
//! caller measures, and says how many columns of the new text are shown so far; nothing here
//! touches ratatui or the clock. It is a presentation only: the text is the same, any key
//! shows the rest of it at once and is then handled as usual (`MOT-8`), reduced motion turns it
//! off, and a step never takes longer than [`MAX_DURATION`] to type however long it is
//! (`MOT-3`).

use std::time::Duration;

/// Longest the typing of one step lasts.
pub(crate) const MAX_DURATION: Duration = Duration::from_millis(2000);

/// The typing of a step: `total` columns of text, shown at a pace.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Typing {
    total: usize,
    elapsed: Duration,
    duration: Duration,
}

impl Typing {
    /// Typing of `total` columns at `rate` columns a second, or `None` when there is nothing
    /// to type. A long text is typed faster rather than longer: it still ends within
    /// [`MAX_DURATION`].
    pub(crate) fn new(total: usize, rate: f64) -> Option<Self> {
        if total == 0 || rate.is_nan() || rate <= 0.0 {
            return None;
        }
        #[allow(
            clippy::cast_precision_loss,
            reason = "a few thousand columns at most; the pace needs no more precision"
        )]
        let wanted = Duration::from_secs_f64(total as f64 / rate);
        Some(Self {
            total,
            elapsed: Duration::ZERO,
            duration: wanted.min(MAX_DURATION),
        })
    }

    /// Moves forward. Never past the end.
    pub(crate) fn tick(&mut self, elapsed: Duration) {
        self.elapsed = self.elapsed.saturating_add(elapsed).min(self.duration);
    }

    pub(crate) fn is_over(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// Columns shown so far: none at the start, all of them at the end, never fewer than
    /// before.
    pub(crate) fn shown(&self) -> usize {
        if self.is_over() {
            return self.total;
        }
        let share = self.elapsed.as_secs_f64() / self.duration.as_secs_f64();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss,
            reason = "a share between 0 and 1 of a few thousand columns, rounded down"
        )]
        let shown = (self.total as f64 * share) as usize;
        shown.min(self.total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn nothing_to_type_or_no_pace_is_no_typing() {
        assert!(Typing::new(0, 400.0).is_none());
        assert!(Typing::new(100, 0.0).is_none());
        assert!(Typing::new(100, -3.0).is_none());
        assert!(Typing::new(100, f64::NAN).is_none());
    }

    #[test]
    fn text_comes_at_the_pace_asked() {
        let mut typing = Typing::new(120, 120.0).unwrap();
        assert_eq!(typing.shown(), 0);
        typing.tick(SECOND / 2);
        assert_eq!(typing.shown(), 60);
        assert!(!typing.is_over());
        typing.tick(SECOND / 2);
        assert_eq!(typing.shown(), 120);
        assert!(typing.is_over());
    }

    #[test]
    fn a_long_text_is_typed_faster_not_longer() {
        // 4,000 columns at 40 a second would take 100 s.
        let mut typing = Typing::new(4000, 40.0).unwrap();
        typing.tick(MAX_DURATION / 2);
        assert_eq!(typing.shown(), 2000);
        typing.tick(MAX_DURATION / 2);
        assert!(typing.is_over());
        assert_eq!(typing.shown(), 4000);
    }

    #[test]
    fn it_never_goes_back_and_never_past_the_end() {
        let mut typing = Typing::new(50, 400.0).unwrap();
        let mut last = 0;
        for _ in 0..100 {
            typing.tick(Duration::from_millis(7));
            assert!(typing.shown() >= last);
            assert!(typing.shown() <= 50);
            last = typing.shown();
        }
        assert_eq!(last, 50);
    }

    #[test]
    fn the_slowest_pace_still_ends_within_the_limit() {
        let mut typing = Typing::new(10_000, 1.0).unwrap();
        typing.tick(MAX_DURATION);
        assert!(typing.is_over());
    }
}
