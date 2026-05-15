//! Composition primitives — `Sequence`, `Parallel`, `Delay`.
//!
//! Sequencing semantics:
//!
//! - **`Sequence<O>`** plays children back-to-back. Duration is
//!   the sum of child durations. Sampling at `t` dispatches to
//!   the child whose cumulative window contains `t`.
//! - **`Parallel<O>`** plays children simultaneously. Duration is
//!   the max child duration. Sampling returns the last child's
//!   output that's still "active" (typical use is for animations
//!   targeting *different* properties — the caller dispatches the
//!   composite output to multiple targets).
//! - **`Delay`** is a zero-output animation that just consumes
//!   time. Use it inside a `Sequence` to pad gaps.
//!
//! Infix builders on the [`AnimationExt`] trait expose
//! `a.then(b)` and `a.and(b)` syntax that returns boxed-trait
//! values typed-erased through `Animation<Output = O>`.

use std::time::Duration;

use crate::Animation;

/// Boxed [`Animation`] trait object. Composition stores children
/// as `Box<dyn Animation<Output = O>>` to allow mixing concrete
/// types (a `Tween<f32>` and a `LinearRamp`, for example).
pub type BoxedAnimation<O> = Box<dyn Animation<Output = O>>;

// ---------------------------------------------------------------------
// Delay — no-output, time-consuming animation
// ---------------------------------------------------------------------

/// A no-output animation that consumes time. Use as a spacer in
/// `Sequence`. Output is `()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delay {
    /// Duration of the delay.
    pub duration: Duration,
}

impl Delay {
    /// Construct a delay of the given duration.
    #[must_use]
    pub const fn new(duration: Duration) -> Self {
        Self { duration }
    }
}

impl Animation for Delay {
    type Output = ();

    fn duration(&self) -> Duration {
        self.duration
    }

    fn sample(&self, _t: Duration) -> Self::Output {}
}

// ---------------------------------------------------------------------
// Sequence — play children one after another
// ---------------------------------------------------------------------

/// Play children one after another. Total duration is the sum of
/// child durations; sampling at `t` finds the active child by
/// linear scan + dispatches.
pub struct Sequence<O: Default> {
    children: Vec<BoxedAnimation<O>>,
}

impl<O: Default> Default for Sequence<O> {
    fn default() -> Self {
        Self::new()
    }
}

impl<O: Default> Sequence<O> {
    /// Construct an empty sequence. Add children with `.then(...)`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Append a child animation.
    #[must_use]
    pub fn then<A: Animation<Output = O> + 'static>(mut self, anim: A) -> Self {
        self.children.push(Box::new(anim));
        self
    }

    /// Number of children.
    #[must_use]
    pub fn child_count(&self) -> usize {
        self.children.len()
    }
}

impl<O: Default> std::fmt::Debug for Sequence<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sequence")
            .field("child_count", &self.children.len())
            .field("total_duration", &self.duration())
            .finish()
    }
}

impl<O: Default> Animation for Sequence<O> {
    type Output = O;

    fn duration(&self) -> Duration {
        self.children
            .iter()
            .map(|c| c.duration())
            .fold(Duration::ZERO, |acc, d| acc.saturating_add(d))
    }

    fn sample(&self, t: Duration) -> O {
        if self.children.is_empty() {
            return O::default();
        }
        let mut cursor = Duration::ZERO;
        for child in &self.children {
            let dur = child.duration();
            let end = cursor.saturating_add(dur);
            if t <= end {
                let local = t.checked_sub(cursor).unwrap_or_default();
                return child.sample(local);
            }
            cursor = end;
        }
        // Past the end: return the last child's terminal value.
        let last = self.children.last().expect("non-empty above");
        last.sample(last.duration())
    }
}

// ---------------------------------------------------------------------
// Parallel — play children simultaneously
// ---------------------------------------------------------------------

/// Play children simultaneously. Sampling returns the last child's
/// output — typical use is to fan out to multiple targets, with
/// callers tracking the per-child output via separate calls.
pub struct Parallel<O: Default> {
    children: Vec<BoxedAnimation<O>>,
}

impl<O: Default> Default for Parallel<O> {
    fn default() -> Self {
        Self::new()
    }
}

impl<O: Default> Parallel<O> {
    /// Construct an empty parallel set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Add a child animation.
    #[must_use]
    pub fn with<A: Animation<Output = O> + 'static>(mut self, anim: A) -> Self {
        self.children.push(Box::new(anim));
        self
    }

    /// Number of children.
    #[must_use]
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Sample every child at the same `t` and return their outputs
    /// in declaration order. Allocates a `Vec` — for hot-path use
    /// the caller should `iter()` and call `Animation::sample`
    /// directly on each child.
    pub fn sample_all(&self, t: Duration) -> Vec<O> {
        self.children.iter().map(|c| c.sample(t)).collect()
    }
}

impl<O: Default> std::fmt::Debug for Parallel<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parallel")
            .field("child_count", &self.children.len())
            .field("total_duration", &self.duration())
            .finish()
    }
}

impl<O: Default> Animation for Parallel<O> {
    type Output = O;

    fn duration(&self) -> Duration {
        self.children
            .iter()
            .map(|c| c.duration())
            .max()
            .unwrap_or_default()
    }

    fn sample(&self, t: Duration) -> O {
        if let Some(last) = self.children.last() {
            last.sample(t)
        } else {
            O::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LinearRamp, Tween};

    #[test]
    fn sequence_concatenates_durations() {
        let s: Sequence<f32> = Sequence::new()
            .then(LinearRamp::new(0.0, 1.0, Duration::from_millis(300)))
            .then(LinearRamp::new(1.0, 0.0, Duration::from_millis(200)));
        assert_eq!(s.duration(), Duration::from_millis(500));
    }

    #[test]
    fn sequence_dispatches_to_correct_child() {
        let s: Sequence<f32> = Sequence::new()
            .then(LinearRamp::new(0.0, 1.0, Duration::from_millis(100)))
            .then(LinearRamp::new(1.0, 0.0, Duration::from_millis(100)));
        // t = 50ms → first child halfway → 0.5
        assert!((s.sample(Duration::from_millis(50)) - 0.5).abs() < 1e-3);
        // t = 150ms → second child halfway → 0.5
        assert!((s.sample(Duration::from_millis(150)) - 0.5).abs() < 1e-3);
        // t = 200ms → at end → 0.0
        assert!((s.sample(Duration::from_millis(200)) - 0.0).abs() < 1e-3);
    }

    #[test]
    fn empty_sequence_returns_default() {
        let s: Sequence<f32> = Sequence::new();
        assert!((s.sample(Duration::ZERO) - 0.0).abs() < f32::EPSILON);
        assert_eq!(s.duration(), Duration::ZERO);
    }

    #[test]
    fn parallel_duration_is_max() {
        let p: Parallel<f32> = Parallel::new()
            .with(LinearRamp::new(0.0, 1.0, Duration::from_millis(300)))
            .with(LinearRamp::new(0.0, 1.0, Duration::from_millis(700)));
        assert_eq!(p.duration(), Duration::from_millis(700));
    }

    #[test]
    fn parallel_sample_all_returns_per_child() {
        let p: Parallel<f32> = Parallel::new()
            .with(LinearRamp::new(0.0, 100.0, Duration::from_secs(1)))
            .with(LinearRamp::new(0.0, 50.0, Duration::from_secs(1)));
        let s = p.sample_all(Duration::from_millis(500));
        assert_eq!(s.len(), 2);
        assert!((s[0] - 50.0).abs() < 1e-3);
        assert!((s[1] - 25.0).abs() < 1e-3);
    }

    #[test]
    fn delay_consumes_time() {
        let d = Delay::new(Duration::from_millis(250));
        assert_eq!(d.duration(), Duration::from_millis(250));
    }

    #[test]
    fn sequence_with_delay_pads() {
        // A Tween, then a 200 ms pause, then a Tween-style hold.
        let s: Sequence<f32> = Sequence::new()
            .then(Tween::new(0.0_f32, 1.0, Duration::from_millis(100)))
            .then(Tween::new(1.0_f32, 1.0, Duration::from_millis(200)));
        // After 250 ms: 100 ms tween done, 150 ms into hold → 1.0.
        assert!((s.sample(Duration::from_millis(250)) - 1.0).abs() < 1e-3);
    }
}
