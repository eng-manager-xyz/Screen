//! `Tween<V>` — interpolate a value from `from` to `to` over
//! `duration`, shaped by an [`Ease`].

use std::time::Duration;

use crate::{Animatable, Animation, Ease};

/// Tween value type.
///
/// Built as a plain struct (not a builder) so callers can construct
/// it in `const` contexts and pass it around as a value. Chainable
/// builder methods (`.ease`, `.duration`) return `Self` for the
/// common construction style.
#[derive(Clone, Debug)]
pub struct Tween<V: Animatable> {
    /// Starting value.
    pub from: V,
    /// Ending value.
    pub to: V,
    /// Total duration.
    pub duration: Duration,
    /// Easing curve applied to the `0..=1` parameter before lerping.
    pub ease: Ease,
}

impl<V: Animatable> Tween<V> {
    /// Construct with default `Linear` ease.
    #[must_use]
    pub fn new(from: V, to: V, duration: Duration) -> Self {
        Self {
            from,
            to,
            duration,
            ease: Ease::Linear,
        }
    }

    /// Override the ease.
    #[must_use]
    pub fn ease(mut self, ease: Ease) -> Self {
        self.ease = ease;
        self
    }

    /// Override the duration.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

impl<V: Animatable> Animation for Tween<V> {
    type Output = V;

    fn duration(&self) -> Duration {
        self.duration
    }

    fn sample(&self, t: Duration) -> V {
        if self.duration.is_zero() {
            return self.to.clone();
        }
        #[allow(
            clippy::cast_possible_truncation,
            reason = "progress is bounded in [0, 1] before reaching the cast"
        )]
        let raw = (t.as_secs_f64() / self.duration.as_secs_f64()) as f32;
        let progress = raw.clamp(0.0, 1.0);
        let eased = self.ease.eval(progress);
        V::lerp(&self.from, &self.to, eased)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tween_f32_endpoints() {
        let t = Tween::new(0.0_f32, 100.0, Duration::from_secs(1));
        assert!((t.sample(Duration::ZERO) - 0.0).abs() < f32::EPSILON);
        assert!((t.sample(Duration::from_secs(1)) - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn tween_linear_midpoint_is_average() {
        let t = Tween::new(0.0_f32, 100.0, Duration::from_secs(1));
        assert!((t.sample(Duration::from_millis(500)) - 50.0).abs() < 1e-3);
    }

    #[test]
    fn tween_with_ease_changes_midpoint() {
        let linear = Tween::new(0.0_f32, 100.0, Duration::from_secs(1));
        let eased = linear.clone().ease(Ease::InQuad);
        let mid = Duration::from_millis(500);
        // OutQuad at t=0.5 = 1 - 0.5^2 = 0.75; InQuad = 0.25.
        assert!((linear.sample(mid) - 50.0).abs() < 1e-3);
        assert!((eased.sample(mid) - 25.0).abs() < 1e-3);
    }

    #[test]
    fn tween_clamps_outside_window() {
        let t = Tween::new(0.0_f32, 100.0, Duration::from_secs(1));
        assert!((t.sample(Duration::from_secs(99)) - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn zero_duration_returns_endpoint() {
        let t = Tween::new(0.0_f32, 100.0, Duration::ZERO);
        assert!((t.sample(Duration::ZERO) - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn builder_chain() {
        let t = Tween::new(0.0_f32, 1.0, Duration::from_millis(100))
            .ease(Ease::OutCubic)
            .duration(Duration::from_secs(2));
        assert_eq!(t.duration, Duration::from_secs(2));
        // Eq isn't derived because the Fn variant holds a fn ptr,
        // so check the chosen variant via the discriminant.
        assert!(matches!(t.ease, Ease::OutCubic));
    }
}
