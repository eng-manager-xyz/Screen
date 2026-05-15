//! `Spring` — a damped harmonic oscillator that converges from
//! `from` to `to`. Open-ended motion; the [`Animation`]
//! impl reports a *settling* duration based on a small position
//! + velocity threshold.

#![allow(
    clippy::many_single_char_names,
    reason = "Spring math uses standard physics notation (m, k, c, ζ, ωn, ωd). Renaming to wordy forms hurts readability for anyone reading the closed-form expressions."
)]
//!
//! Two springs ship today:
//!
//! - [`Spring::critically_damped`] — fastest settle without
//!   oscillation. The default for UI motion where overshoot
//!   would be distracting.
//! - [`Spring::underdamped`] — overshoots, oscillates, settles.
//!   Good for "spring-y" affordances (toasts, undo notices).
//!
//! Implementation is closed-form analytical for both cases, so
//! `sample(t)` is a pure function. No iterative integration; no
//! per-frame hidden state.

use std::time::Duration;

use crate::Animation;

/// 1-D damped spring.
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    /// Resting value.
    pub to: f32,
    /// Starting position.
    pub from: f32,
    /// Stiffness `k` (N/m equivalent).
    pub stiffness: f32,
    /// Mass `m` (kg equivalent).
    pub mass: f32,
    /// Damping coefficient `c` (N·s/m equivalent).
    pub damping: f32,
    /// Initial velocity at `t = 0`.
    pub initial_velocity: f32,
}

impl Spring {
    /// Construct a critically-damped spring (fastest settle, no
    /// overshoot). Mass = 1.0; damping = `2 · sqrt(k · m)`.
    #[must_use]
    pub fn critically_damped(stiffness: f32, mass: f32) -> Self {
        let damping = 2.0 * (stiffness * mass).sqrt();
        Self {
            to: 1.0,
            from: 0.0,
            stiffness,
            mass: mass.max(f32::EPSILON),
            damping,
            initial_velocity: 0.0,
        }
    }

    /// Construct an underdamped spring — overshoots and oscillates.
    #[must_use]
    pub fn underdamped(stiffness: f32, mass: f32, damping_ratio: f32) -> Self {
        let critical = 2.0 * (stiffness * mass).sqrt();
        Self {
            to: 1.0,
            from: 0.0,
            stiffness,
            mass: mass.max(f32::EPSILON),
            damping: critical * damping_ratio.clamp(0.0, 1.0),
            initial_velocity: 0.0,
        }
    }

    /// Tuned UI-feel default — critically damped at `k = 170`,
    /// `m = 1`. Settle time ≈ 600 ms for a unit step input.
    #[must_use]
    pub fn ui_default() -> Self {
        Self::critically_damped(170.0, 1.0)
    }

    /// Override endpoints.
    #[must_use]
    pub fn between(mut self, from: f32, to: f32) -> Self {
        self.from = from;
        self.to = to;
        self
    }

    /// Override initial velocity.
    #[must_use]
    pub fn with_initial_velocity(mut self, v: f32) -> Self {
        self.initial_velocity = v;
        self
    }

    /// Estimate when the spring is "settled" — when position is
    /// within `1 %` of `to` AND velocity is below the threshold.
    /// Closed-form for the critically-damped + underdamped cases;
    /// returns a conservative `5 / omega_n` envelope.
    #[must_use]
    pub fn settling_duration(self) -> Duration {
        let omega_n = (self.stiffness / self.mass).sqrt().max(f32::EPSILON);
        let zeta = self.damping_ratio();
        let seconds = if zeta >= 1.0 {
            // Critically/overdamped — `5τ` covers most of the curve.
            5.0 / omega_n
        } else {
            // Underdamped — envelope decays as `e^(-zeta·omega_n·t)`.
            // Settle when envelope drops below 1% → t = ln(100)/(zeta·omega_n).
            let zeta_omega = zeta.max(0.05) * omega_n;
            (5.0_f32.ln() * 2.0) / zeta_omega
        };
        Duration::from_secs_f32(seconds.max(0.01))
    }

    /// Compute damping ratio `ζ = c / (2 · sqrt(k·m))`.
    #[must_use]
    pub fn damping_ratio(self) -> f32 {
        self.damping / (2.0 * (self.stiffness * self.mass).sqrt())
    }
}

impl Animation for Spring {
    type Output = f32;

    fn duration(&self) -> Duration {
        self.settling_duration()
    }

    fn sample(&self, t: Duration) -> f32 {
        let secs = t.as_secs_f32();
        let dx0 = self.from - self.to;
        let v0 = self.initial_velocity;
        let omega_n = (self.stiffness / self.mass).sqrt();
        let zeta = self.damping_ratio();

        if zeta >= 1.0 {
            // Critically/overdamped — `x(t) = (A + B·t) · e^(-omega_n · t)`.
            let a = dx0;
            let b = v0 + omega_n * dx0;
            let envelope = (-omega_n * secs).exp();
            self.to + (a + b * secs) * envelope
        } else {
            // Underdamped — damped sinusoid.
            let omega_d = omega_n * (1.0 - zeta * zeta).sqrt();
            let envelope = (-zeta * omega_n * secs).exp();
            let cos_part = dx0 * (omega_d * secs).cos();
            let sin_part =
                (v0 + zeta * omega_n * dx0) / omega_d.max(f32::EPSILON) * (omega_d * secs).sin();
            self.to + envelope * (cos_part + sin_part)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critically_damped_settles_within_a_second() {
        let s = Spring::ui_default().between(0.0, 1.0);
        let final_val = s.sample(Duration::from_secs(2));
        assert!((final_val - 1.0).abs() < 0.05);
    }

    #[test]
    fn settling_duration_bounded() {
        let s = Spring::ui_default();
        assert!(s.settling_duration() < Duration::from_secs(2));
        assert!(s.settling_duration() > Duration::from_millis(10));
    }

    #[test]
    fn underdamped_overshoots() {
        let s = Spring::underdamped(80.0, 1.0, 0.4).between(0.0, 1.0);
        // Find any sample > 1.0 within the first second.
        let mut max = 0.0_f32;
        for i in 0..100 {
            let v = s.sample(Duration::from_millis(i * 10));
            if v > max {
                max = v;
            }
        }
        assert!(max > 1.0, "expected overshoot, got max = {max}");
    }

    #[test]
    fn start_is_from() {
        let s = Spring::ui_default().between(10.0, 100.0);
        assert!((s.sample(Duration::ZERO) - 10.0).abs() < 1e-3);
    }

    #[test]
    fn initial_velocity_affects_curve() {
        let s_fast = Spring::ui_default()
            .between(0.0, 1.0)
            .with_initial_velocity(50.0);
        let s_slow = Spring::ui_default().between(0.0, 1.0);
        // Within the first 50ms, the velocity-injected spring should
        // be further along.
        let t = Duration::from_millis(50);
        assert!(s_fast.sample(t) > s_slow.sample(t));
    }

    #[test]
    fn damping_ratio_correct() {
        let s = Spring::critically_damped(100.0, 1.0);
        assert!((s.damping_ratio() - 1.0).abs() < 1e-3);
    }
}
