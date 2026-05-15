//! `Repeat` — wrap an animation to loop, yoyo, or run for a
//! caller-bounded total duration.

use std::time::Duration;

use crate::Animation;

/// How many times a [`Repeat`] cycles before stopping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepeatCount {
    /// Finite cycle count. `0` means "play once, no repeat".
    Finite(u32),
    /// Run forever — duration reports as [`Duration::MAX`].
    Infinite,
    /// Stop after this wall-clock duration, regardless of where in
    /// the cycle the animation is.
    ForDuration(Duration),
}

/// How a [`Repeat`] direction evolves over cycles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RepeatStrategy {
    /// Restart from `t = 0` at every cycle boundary.
    #[default]
    Loop,
    /// Alternate direction — forward, then backward, then forward.
    /// "Yoyo".
    MirroredRepeat,
}

/// Wraps an animation to repeat.
#[derive(Clone, Debug)]
pub struct Repeat<A: Animation> {
    /// Inner animation.
    pub inner: A,
    /// Cycle count.
    pub count: RepeatCount,
    /// Direction strategy.
    pub strategy: RepeatStrategy,
}

impl<A: Animation> Repeat<A> {
    /// Construct a repeater. Defaults to `Loop` strategy.
    #[must_use]
    pub const fn new(inner: A, count: RepeatCount) -> Self {
        Self {
            inner,
            count,
            strategy: RepeatStrategy::Loop,
        }
    }

    /// Override the direction strategy.
    #[must_use]
    pub const fn strategy(mut self, strategy: RepeatStrategy) -> Self {
        self.strategy = strategy;
        self
    }
}

impl<A: Animation> Animation for Repeat<A> {
    type Output = A::Output;

    fn duration(&self) -> Duration {
        let cycle = self.inner.duration();
        match self.count {
            RepeatCount::Finite(n) => cycle.saturating_mul(n.saturating_add(1).max(1)),
            RepeatCount::Infinite => Duration::MAX,
            RepeatCount::ForDuration(d) => d,
        }
    }

    fn sample(&self, t: Duration) -> A::Output {
        let cycle = self.inner.duration();
        if cycle.is_zero() {
            return self.inner.sample(Duration::ZERO);
        }
        let total = self.duration();
        let t_clamped = if matches!(self.count, RepeatCount::Infinite) {
            t
        } else if t >= total {
            // Past the end: deliver the terminal value once.
            return self.inner.sample(cycle);
        } else {
            t
        };

        // How many full cycles we've completed and how far into
        // the current cycle we are.
        let cycle_nanos = cycle.as_nanos();
        let t_nanos = t_clamped.as_nanos();
        #[allow(
            clippy::cast_possible_truncation,
            reason = "cycle index for animations stays within u64 in practice (animations don't run for 2^64 ns)"
        )]
        let cycle_index = (t_nanos / cycle_nanos) as u64;
        #[allow(
            clippy::cast_possible_truncation,
            reason = "local nanos < cycle_nanos which fits u64"
        )]
        let local_nanos = (t_nanos % cycle_nanos) as u64;
        let local = Duration::from_nanos(local_nanos);

        match self.strategy {
            RepeatStrategy::Loop => self.inner.sample(local),
            RepeatStrategy::MirroredRepeat => {
                if cycle_index.is_multiple_of(2) {
                    self.inner.sample(local)
                } else {
                    self.inner.sample(cycle.saturating_sub(local))
                }
            }
        }
    }
}

/// Sugar on the [`Animation`] trait to wrap with a `Repeat`.
pub trait AnimationRepeatExt: Animation + Sized {
    /// Wrap with [`Repeat`].
    fn repeat(self, count: RepeatCount) -> Repeat<Self> {
        Repeat::new(self, count)
    }
    /// Wrap with [`Repeat`] + custom [`RepeatStrategy`].
    fn repeat_with(self, count: RepeatCount, strategy: RepeatStrategy) -> Repeat<Self> {
        Repeat::new(self, count).strategy(strategy)
    }
}

impl<A: Animation + Sized> AnimationRepeatExt for A {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LinearRamp, Tween};

    #[test]
    fn loop_strategy_restarts_each_cycle() {
        let r = LinearRamp::new(0.0_f32, 1.0, Duration::from_millis(100))
            .repeat(RepeatCount::Finite(3));
        // 4 cycles × 100ms = 400ms total.
        assert_eq!(r.duration(), Duration::from_millis(400));
        // At t=150ms → 50ms into second cycle → 0.5.
        assert!((r.sample(Duration::from_millis(150)) - 0.5).abs() < 1e-3);
        // At t=250ms → 50ms into third cycle → 0.5.
        assert!((r.sample(Duration::from_millis(250)) - 0.5).abs() < 1e-3);
    }

    #[test]
    fn mirrored_repeat_reverses_on_odd_cycles() {
        let r = LinearRamp::new(0.0_f32, 1.0, Duration::from_millis(100))
            .repeat_with(RepeatCount::Finite(3), RepeatStrategy::MirroredRepeat);
        // Cycle 0 (forward): at t=50ms → 0.5
        assert!((r.sample(Duration::from_millis(50)) - 0.5).abs() < 1e-3);
        // Cycle 1 (reverse): at t=150ms → 50ms into reverse → 0.5
        // (going from 1.0 back to 0.0 at midpoint).
        assert!((r.sample(Duration::from_millis(150)) - 0.5).abs() < 1e-3);
        // Cycle 1 (reverse) at t=199ms → near end of reverse → near 0.0.
        assert!(r.sample(Duration::from_millis(199)) < 0.1);
    }

    #[test]
    fn infinite_duration_is_max() {
        let r =
            LinearRamp::new(0.0_f32, 1.0, Duration::from_millis(100)).repeat(RepeatCount::Infinite);
        assert_eq!(r.duration(), Duration::MAX);
    }

    #[test]
    fn for_duration_caps_total() {
        let r = LinearRamp::new(0.0_f32, 1.0, Duration::from_millis(100))
            .repeat(RepeatCount::ForDuration(Duration::from_millis(450)));
        assert_eq!(r.duration(), Duration::from_millis(450));
    }

    #[test]
    fn finite_clamps_past_end_to_terminal_value() {
        let r = Tween::new(0.0_f32, 1.0, Duration::from_millis(100)).repeat(RepeatCount::Finite(0)); // play once, no repeat
        assert_eq!(r.duration(), Duration::from_millis(100));
        assert!((r.sample(Duration::from_millis(999)) - 1.0).abs() < 1e-3);
    }
}
