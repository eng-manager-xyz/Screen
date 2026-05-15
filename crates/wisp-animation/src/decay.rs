//! `Decay` — exponential-decay glide. Position approaches an
//! asymptote `from + velocity·τ·(1 - e^{-t/τ})`. Used for
//! "fling-and-settle" interactions (drag-throw, kinetic snap).

use std::time::Duration;

use crate::Animation;

/// 1-D exponential decay.
#[derive(Clone, Copy, Debug)]
pub struct Decay {
    /// Starting position.
    pub from: f32,
    /// Initial velocity (units / sec).
    pub initial_velocity: f32,
    /// Time constant `τ` (seconds) — higher = slower settling.
    pub time_constant: f32,
}

impl Decay {
    /// Construct.
    #[must_use]
    pub const fn new(from: f32, initial_velocity: f32, time_constant: f32) -> Self {
        Self {
            from,
            initial_velocity,
            time_constant,
        }
    }

    /// Predict the asymptotic position as `t → ∞`.
    /// `position = from + velocity · τ`.
    #[must_use]
    pub fn predict_target(&self) -> f32 {
        self.from + self.initial_velocity * self.time_constant
    }
}

impl Animation for Decay {
    type Output = f32;

    fn duration(&self) -> Duration {
        // 5τ covers >99.3 % of the asymptote.
        Duration::from_secs_f32((self.time_constant * 5.0).max(0.01))
    }

    fn sample(&self, t: Duration) -> f32 {
        let secs = t.as_secs_f32();
        let tau = self.time_constant.max(f32::EPSILON);
        let envelope = 1.0 - (-secs / tau).exp();
        self.from + self.initial_velocity * tau * envelope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predict_matches_5tau_sample_within_one_percent() {
        let d = Decay::new(0.0, 10.0, 0.4);
        let asymptote = d.predict_target();
        let late = d.sample(Duration::from_secs_f32(5.0 * 0.4));
        assert!(
            (late - asymptote).abs() / asymptote.abs() < 0.01,
            "expected within 1%, got {late} vs {asymptote}"
        );
    }

    #[test]
    fn zero_initial_velocity_stays_put() {
        let d = Decay::new(7.0, 0.0, 0.4);
        assert!((d.sample(Duration::from_secs(1)) - 7.0).abs() < 1e-3);
    }

    #[test]
    fn starts_at_from() {
        let d = Decay::new(5.0, 10.0, 0.3);
        assert!((d.sample(Duration::ZERO) - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn duration_is_five_tau() {
        let d = Decay::new(0.0, 1.0, 0.2);
        assert!(
            (d.duration().as_secs_f32() - 1.0).abs() < 1e-3,
            "5 * 0.2 = 1.0s"
        );
    }
}
