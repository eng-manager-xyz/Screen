//! `KineticPan` — flick-and-glide pan for the Gantt body.
//!
//! Wraps two `wisp_animation::Decay` instances (one per axis) so a
//! drag-release leaves the body coasting toward rest under
//! exponential decay. Velocity is captured at pan-end by the host
//! from the drag history; the resulting `Vec2` plus a time
//! constant `τ` shapes the glide.
//!
//! ## Why a Gantt-side wrapper
//!
//! `Decay` is one-dimensional and time-domain-only. A 2D pan needs:
//!
//! - Two decays (one per axis), shared start time, shared cancel
//!   signal.
//! - Clamp awareness: when the body offset clamps against the
//!   content boundary on one axis, that axis's decay must stop
//!   immediately (no overshoot).
//! - Press-to-cancel: a new `pan_begin` mid-glide must drop the
//!   inertia.
//! - Reduced-motion gate: hosts can disable the glide entirely.
//!
//! ## Lifecycle
//!
//! 1. Host tracks drag-pointer samples + their dt during
//!    `pan_drag`.
//! 2. On `pan_end`, host computes velocity = `(last_pos - earlier_pos)
//!    / sum_dt` (pixels/sec).
//! 3. Host constructs `KineticPan::from_velocity(current_offset,
//!    velocity, tau)` IF velocity exceeds a small threshold (e.g.
//!    `0.05` px/ms) AND reduced-motion is off.
//! 4. Host's render loop calls `kinetic.tick(dt, controller,
//!    viewport)` each frame; the method writes new
//!    `viewport.body_offset` and returns `true` while still
//!    animating.
//! 5. Host drops the `KineticPan` (or calls `cancel`) on the next
//!    `pan_begin`.

use std::time::Duration;

use glam::Vec2;
use wisp_animation::{Animation, Decay};

use crate::gantt::pan::{GanttPanController, GanttViewport};

/// Recommended decay time constant for Gantt kinetic pan,
/// in seconds. Matched empirically against the H2 planning DOM's
/// `gantt.js` momentum: 350 ms feels "weighty but responsive."
pub const DEFAULT_TAU: f32 = 0.35;

/// Minimum |velocity| (pixels/sec) that triggers a kinetic glide.
/// Below this threshold a release is treated as a static pan end
/// (no inertia). 60 px/s ≈ 1 cell per second at 60 px cells.
pub const MIN_VELOCITY_PX_PER_S: f32 = 60.0;

/// Two-axis kinetic-pan animator wrapping
/// [`wisp_animation::Decay`].
///
/// Construct via [`KineticPan::from_velocity`] when the host wants
/// to start a glide on pan release. Tick from the render loop;
/// cancel on the next press.
#[derive(Debug, Clone)]
pub struct KineticPan {
    decay_x: Decay,
    decay_y: Decay,
    /// Elapsed seconds since start, accumulated by `tick`.
    elapsed: f32,
    /// Whether each axis is still animating (clears when the axis
    /// clamps against content bounds).
    active_x: bool,
    active_y: bool,
}

impl KineticPan {
    /// Build from an initial offset + velocity + time constant.
    ///
    /// Returns `None` when |velocity| is below
    /// [`MIN_VELOCITY_PX_PER_S`] on BOTH axes — there's nothing to
    /// animate. Callers should fall back to leaving the offset at
    /// `start_offset`.
    ///
    /// `tau` is the per-axis decay time constant (use
    /// [`DEFAULT_TAU`] for the default feel).
    #[must_use]
    pub fn from_velocity(start_offset: Vec2, velocity: Vec2, tau: f32) -> Option<Self> {
        if velocity.length() < MIN_VELOCITY_PX_PER_S {
            return None;
        }
        Some(Self {
            decay_x: Decay::new(start_offset.x, velocity.x, tau),
            decay_y: Decay::new(start_offset.y, velocity.y, tau),
            elapsed: 0.0,
            active_x: true,
            active_y: true,
        })
    }

    /// Cancel the glide immediately. Both axes go inactive.
    pub fn cancel(&mut self) {
        self.active_x = false;
        self.active_y = false;
    }

    /// `true` if either axis is still animating.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active_x || self.active_y
    }

    /// Advance the glide by `dt` seconds and write the new offset
    /// into `viewport.body_offset`. Clamps through
    /// [`GanttPanController::clamp`]; when an axis clamps against
    /// the content boundary, that axis stops animating immediately
    /// (no overshoot, no bounce).
    ///
    /// Returns `true` while the glide is still going.
    pub fn tick(
        &mut self,
        dt: f32,
        controller: &GanttPanController,
        viewport: &mut GanttViewport,
    ) -> bool {
        if !self.is_active() {
            return false;
        }
        self.elapsed += dt.max(0.0);
        let t = Duration::from_secs_f32(self.elapsed);

        let prev_offset = viewport.body_offset;
        let mut next = prev_offset;
        if self.active_x {
            next.x = self.decay_x.sample(t);
        }
        if self.active_y {
            next.y = self.decay_y.sample(t);
        }
        viewport.body_offset = next;
        controller.clamp(viewport);

        // If clamp moved an axis back, that axis hit a boundary —
        // stop animating it.
        if self.active_x && (viewport.body_offset.x - next.x).abs() > 1e-3 {
            self.active_x = false;
        }
        if self.active_y && (viewport.body_offset.y - next.y).abs() > 1e-3 {
            self.active_y = false;
        }

        // Stop axes that have effectively reached their asymptote
        // (5τ = 99.3% per Decay::duration). We approximate by
        // comparing the predicted target to the current value.
        let predicted_x = self.decay_x.predict_target();
        if self.active_x && (predicted_x - viewport.body_offset.x).abs() < 0.5 {
            self.active_x = false;
        }
        let predicted_y = self.decay_y.predict_target();
        if self.active_y && (predicted_y - viewport.body_offset.y).abs() < 0.5 {
            self.active_y = false;
        }

        self.is_active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl() -> GanttPanController {
        GanttPanController::new(
            60.0,
            180.0,
            Vec2::new(2000.0, 500.0),
            Vec2::new(800.0, 400.0),
        )
    }

    #[test]
    fn from_velocity_returns_none_below_min_threshold() {
        let k = KineticPan::from_velocity(Vec2::ZERO, Vec2::new(20.0, 30.0), DEFAULT_TAU);
        assert!(k.is_none(), "30 px/s vector below min threshold");
    }

    #[test]
    fn from_velocity_returns_some_above_min_threshold() {
        let k = KineticPan::from_velocity(Vec2::ZERO, Vec2::new(200.0, 100.0), DEFAULT_TAU);
        assert!(k.is_some());
    }

    #[test]
    fn diagonal_velocity_preserves_both_components() {
        let mut k = KineticPan::from_velocity(
            Vec2::new(-50.0, -20.0),
            Vec2::new(-300.0, -200.0),
            DEFAULT_TAU,
        )
        .unwrap();
        let controller = ctrl();
        let mut v = GanttViewport {
            body_offset: Vec2::new(-50.0, -20.0),
        };
        // First tick — both axes should move.
        k.tick(0.016, &controller, &mut v);
        assert!(v.body_offset.x < -50.0 - 1e-3, "x moved");
        assert!(v.body_offset.y < -20.0 - 1e-3, "y moved");
    }

    #[test]
    fn tick_returns_false_after_axis_clamps_and_other_axis_done() {
        // Throw with both axes destined to clamp.
        let mut k = KineticPan::from_velocity(
            Vec2::new(-500.0, -200.0),
            Vec2::new(-9_000.0, -9_000.0),
            DEFAULT_TAU,
        )
        .unwrap();
        let controller = ctrl();
        let mut v = GanttViewport {
            body_offset: Vec2::new(-500.0, -200.0),
        };
        // Step until inactive (bound is finite — should converge).
        for _ in 0..200 {
            if !k.tick(0.016, &controller, &mut v) {
                break;
            }
        }
        assert!(!k.is_active(), "should converge / clamp out");
    }

    #[test]
    fn cancel_stops_glide_immediately() {
        let mut k =
            KineticPan::from_velocity(Vec2::ZERO, Vec2::new(500.0, 500.0), DEFAULT_TAU).unwrap();
        k.cancel();
        assert!(!k.is_active());
        let controller = ctrl();
        let mut v = GanttViewport::new();
        // Subsequent tick is a no-op.
        let still = k.tick(0.016, &controller, &mut v);
        assert!(!still);
        assert_eq!(v.body_offset, Vec2::ZERO);
    }

    #[test]
    fn deterministic_sample_with_fixed_dt() {
        // Two glides with identical params + identical tick sequence
        // produce identical end offsets.
        let make = || {
            KineticPan::from_velocity(Vec2::ZERO, Vec2::new(-200.0, -200.0), DEFAULT_TAU).unwrap()
        };
        let controller = ctrl();
        let mut a = make();
        let mut b = make();
        let mut va = GanttViewport::new();
        let mut vb = GanttViewport::new();
        for _ in 0..10 {
            a.tick(0.016, &controller, &mut va);
            b.tick(0.016, &controller, &mut vb);
        }
        assert!((va.body_offset - vb.body_offset).length() < 1e-5);
    }
}
