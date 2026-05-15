//! `TypeWriter` — character-by-character text reveal.
//!
//! Produces a `usize` count of visible characters as a function of
//! time. Caller pairs this with `wisp::Text` (or any text renderer)
//! by truncating the string before render. A future enabler ticket
//! on `wisp::Text` will expose a `visible_glyph_range` setter so
//! `TypeWriter` can drive it directly via `Target<usize>`.

use std::time::Duration;

use crate::Animation;

/// Reveal characters of a fixed-length string over `duration`.
#[derive(Clone, Copy, Debug)]
pub struct TypeWriter {
    /// Total character count to reveal.
    pub total_chars: usize,
    /// Reveal duration.
    pub duration: Duration,
}

impl TypeWriter {
    /// Construct from a target character count and total duration.
    #[must_use]
    pub const fn new(total_chars: usize, duration: Duration) -> Self {
        Self {
            total_chars,
            duration,
        }
    }

    /// Construct from a desired character rate (chars / sec).
    #[must_use]
    pub fn at_rate(total_chars: usize, chars_per_sec: f32) -> Self {
        let rate = chars_per_sec.max(f32::EPSILON);
        #[allow(
            clippy::cast_precision_loss,
            reason = "chars typically < 10_000; f32 fits"
        )]
        let dur = Duration::from_secs_f32(total_chars as f32 / rate);
        Self {
            total_chars,
            duration: dur,
        }
    }
}

impl Animation for TypeWriter {
    type Output = usize;

    fn duration(&self) -> Duration {
        self.duration
    }

    fn sample(&self, t: Duration) -> usize {
        if self.total_chars == 0 || self.duration.is_zero() {
            return self.total_chars;
        }
        #[allow(
            clippy::cast_possible_truncation,
            reason = "progress bounded [0, 1]"
        )]
        let raw = (t.as_secs_f64() / self.duration.as_secs_f64()) as f32;
        let p = raw.clamp(0.0, 1.0);
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "chars typically < 10_000; bounded"
        )]
        let visible = (p * self.total_chars as f32).round() as usize;
        visible.min(self.total_chars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_at_start() {
        let tw = TypeWriter::new(20, Duration::from_secs(1));
        assert_eq!(tw.sample(Duration::ZERO), 0);
    }

    #[test]
    fn all_at_end() {
        let tw = TypeWriter::new(20, Duration::from_secs(1));
        assert_eq!(tw.sample(Duration::from_secs(1)), 20);
    }

    #[test]
    fn halfway_is_half() {
        let tw = TypeWriter::new(20, Duration::from_secs(1));
        assert_eq!(tw.sample(Duration::from_millis(500)), 10);
    }

    #[test]
    fn at_rate_derives_duration() {
        let tw = TypeWriter::at_rate(20, 20.0); // 20 chars/sec
        assert_eq!(tw.duration, Duration::from_secs(1));
    }

    #[test]
    fn clamps_past_end() {
        let tw = TypeWriter::new(10, Duration::from_secs(1));
        assert_eq!(tw.sample(Duration::from_secs(99)), 10);
    }
}
