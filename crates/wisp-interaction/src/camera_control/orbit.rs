//! `OrbitController` — Three.js `OrbitControls.js` port.
//!
//! Rotates the camera around a fixed target point using spherical
//! coordinates. The user drags the LMB to orbit, holds shift+LMB or
//! presses MMB to pan, and rolls the wheel (or RMB-drags) to dolly.
//!
//! Reference: <https://github.com/mrdoob/three.js/blob/r170/examples/jsm/controls/OrbitControls.js>
//!
//! Differences from the JS original we intentionally skip in v1:
//! - keyboard arrow-key panning (file as follow-up if a consumer asks)
//! - target offset auto-clamp (we expose hard min/max for distance,
//!   polar, azimuth; the JS dynamic re-anchoring math is overkill)
//! - dolly-to-pointer / dolly-to-cursor (file as follow-up — the
//!   `PanZoomController` in WI.5 covers the 2D version)
//!
//! Math summary:
//! - Spherical = `(theta, phi, radius)` where
//!   - `theta` is azimuth, measured from +z around +y (right-handed)
//!   - `phi` is polar angle, measured from +y down to -y (0..=π)
//!   - `radius` is the distance from `target` to `position`
//! - Cartesian conversion (right-handed, y-up):
//!   - `x = radius * sin(phi) * sin(theta)`
//!   - `y = radius * cos(phi)`
//!   - `z = radius * sin(phi) * cos(theta)`
//! - Pan is computed in camera-local space: a horizontal-pixel delta
//!   maps to camera-`right`, vertical to camera-`up`, scaled by the
//!   "world units per pixel" at the target depth (via fov + distance).

use std::f32::consts::PI;

use glam::{Vec2, Vec3};

/// Minimal camera interface the controller mutates.
///
/// Hosts impl this for whatever camera struct they own
/// (`wisp_3d::Camera3D`, a test stub, etc.). Keeping the trait local
/// to `wisp-interaction` lets the controller stay free of a
/// `wisp-3d` dep — see the [crate]-level docs.
pub trait Camera3D {
    /// World-space camera position (the eye point).
    fn position(&self) -> Vec3;
    /// World-space target the camera looks at.
    fn target(&self) -> Vec3;
    /// World-space up direction (typically `Vec3::Y`).
    fn up(&self) -> Vec3;

    /// Set the camera's eye position.
    fn set_position(&mut self, p: Vec3);
    /// Set the look-at target.
    fn set_target(&mut self, t: Vec3);
    /// Vertical field of view in radians. Used by the pan math to
    /// convert pixel deltas to world units. Default `60deg.to_radians()`
    /// is a reasonable fallback for hosts that don't know.
    fn fov_y(&self) -> f32 {
        60.0_f32.to_radians()
    }
}

/// The orbit controller's discrete input state. Mostly internal but
/// public so callers can drive state transitions from custom adapters
/// (e.g. a keyboard arrow-key shim).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrbitState {
    /// No drag in progress.
    #[default]
    None,
    /// Rotating around the target (LMB drag).
    Rotate,
    /// Dollying in/out (RMB drag).
    Dolly,
    /// Panning the target (MMB drag or shift+LMB).
    Pan,
}

/// Three.js `OrbitControls.js` port.
///
/// Construction is via [`OrbitController::new`] (sensible defaults);
/// tweak the `pub` fields directly to customise. Drive it from your
/// adapter:
///
/// 1. On LMB press → [`OrbitController::pointer_down_rotate`]
///    (or `_pan` / `_dolly` for the other buttons).
/// 2. On pointer move while a button is held →
///    [`OrbitController::pointer_drag`] with the viewport position.
/// 3. On wheel → [`OrbitController::wheel`].
/// 4. On pointer release → [`OrbitController::pointer_up`].
/// 5. Every frame → [`OrbitController::update`] with the elapsed
///    seconds and the host's `&mut impl Camera3D`. Returns `true` iff
///    the camera actually moved — use to skip render submission when
///    nothing changed.
#[derive(Debug, Clone)]
pub struct OrbitController {
    /// Current discrete state (no input / rotate / dolly / pan).
    state: OrbitState,

    /// Enable inertia smoothing on rotate / pan / dolly accumulators.
    pub enable_damping: bool,
    /// Per-frame multiplier applied to remaining accumulators when
    /// damping is on. Three.js default is `0.05`; we use the same.
    pub damping_factor: f32,
    /// Multiplier on rotate delta (1.0 = match Three.js default).
    pub rotate_speed: f32,
    /// Multiplier on dolly delta.
    pub zoom_speed: f32,
    /// Multiplier on pan delta.
    pub pan_speed: f32,
    /// Minimum allowed `radius` (distance from target).
    pub min_distance: f32,
    /// Maximum allowed `radius`.
    pub max_distance: f32,
    /// Lower bound on `phi` (polar angle, radians from +Y).
    /// `0.0` lets the camera look straight down at the target.
    pub min_polar_angle: f32,
    /// Upper bound on `phi`. `PI` lets the camera look straight up.
    pub max_polar_angle: f32,
    /// Lower bound on `theta` (azimuth). `-inf` disables.
    pub min_azimuth_angle: f32,
    /// Upper bound on `theta`. `+inf` disables.
    pub max_azimuth_angle: f32,
    /// When true, [`OrbitController::update`] advances `theta` by
    /// `auto_rotate_speed * dt` every frame.
    pub auto_rotate: bool,
    /// Auto-rotate angular velocity (radians / second). Three.js
    /// default is `2.0 * PI / 60.0 * 2.0` (about 12 deg/s at 60fps).
    pub auto_rotate_speed: f32,

    // Spherical accumulators, applied on `update`.
    delta_theta: f32,
    delta_phi: f32,
    /// Multiplicative scale on radius. Accumulates dolly-in/out;
    /// applied as `radius *= scale` per update.
    scale: f32,
    /// Pan accumulator in WORLD coordinates. Applied to both target
    /// and position so the relative offset is preserved.
    pan_offset: Vec3,

    // Last pointer position (per state).
    rotate_last: Option<Vec2>,
    pan_last: Option<Vec2>,
    dolly_last: Option<Vec2>,
}

impl Default for OrbitController {
    fn default() -> Self {
        Self::new()
    }
}

impl OrbitController {
    /// Construct with Three.js defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: OrbitState::None,
            enable_damping: false,
            damping_factor: 0.05,
            rotate_speed: 1.0,
            zoom_speed: 1.0,
            pan_speed: 1.0,
            min_distance: 0.0,
            max_distance: f32::INFINITY,
            min_polar_angle: 0.0,
            max_polar_angle: PI,
            min_azimuth_angle: f32::NEG_INFINITY,
            max_azimuth_angle: f32::INFINITY,
            auto_rotate: false,
            auto_rotate_speed: (2.0 * PI / 60.0) * 2.0,
            delta_theta: 0.0,
            delta_phi: 0.0,
            scale: 1.0,
            pan_offset: Vec3::ZERO,
            rotate_last: None,
            pan_last: None,
            dolly_last: None,
        }
    }

    /// Read-only access to the current state, for tests + diagnostics.
    #[must_use]
    pub fn state(&self) -> OrbitState {
        self.state
    }

    /// Enter rotate mode (typically on LMB press).
    pub fn pointer_down_rotate(&mut self, viewport: Vec2) {
        self.state = OrbitState::Rotate;
        self.rotate_last = Some(viewport);
    }

    /// Enter pan mode (typically on MMB press or shift+LMB).
    pub fn pointer_down_pan(&mut self, viewport: Vec2) {
        self.state = OrbitState::Pan;
        self.pan_last = Some(viewport);
    }

    /// Enter dolly mode (typically on RMB press).
    pub fn pointer_down_dolly(&mut self, viewport: Vec2) {
        self.state = OrbitState::Dolly;
        self.dolly_last = Some(viewport);
    }

    /// Exit drag mode. Accumulators continue to damp out across
    /// future `update` calls if `enable_damping` is set.
    pub fn pointer_up(&mut self) {
        self.state = OrbitState::None;
        self.rotate_last = None;
        self.pan_last = None;
        self.dolly_last = None;
    }

    /// Pointer moved while a button is held. `viewport_size` is the
    /// host surface size in the same units as `viewport_pos` (CSS
    /// pixels), used to scale rotate / pan deltas. `distance` is the
    /// current camera-to-target distance, used to scale pan into
    /// world units.
    pub fn pointer_drag(
        &mut self,
        viewport_pos: Vec2,
        viewport_size: Vec2,
        distance: f32,
        camera_right: Vec3,
        camera_up: Vec3,
        fov_y: f32,
    ) {
        match self.state {
            OrbitState::Rotate => {
                if let Some(prev) = self.rotate_last {
                    let delta = viewport_pos - prev;
                    // Three.js: rotateLeft(2π * dx / clientHeight * rotateSpeed),
                    //          rotateUp(2π * dy / clientHeight * rotateSpeed).
                    // Using clientHeight for BOTH so the speed feels
                    // matched to a square viewport.
                    let scale = 2.0 * PI * self.rotate_speed / viewport_size.y.max(1.0);
                    self.delta_theta -= delta.x * scale;
                    self.delta_phi -= delta.y * scale;
                    self.rotate_last = Some(viewport_pos);
                }
            }
            OrbitState::Pan => {
                if let Some(prev) = self.pan_last {
                    let delta = viewport_pos - prev;
                    // Three.js panUp / panLeft: convert pixel delta to
                    // world units using fov_y + distance.
                    let world_units_per_pixel =
                        2.0 * distance * (fov_y * 0.5).tan() / viewport_size.y.max(1.0);
                    let pan_x = -delta.x * world_units_per_pixel * self.pan_speed;
                    let pan_y = delta.y * world_units_per_pixel * self.pan_speed;
                    self.pan_offset += camera_right * pan_x + camera_up * pan_y;
                    self.pan_last = Some(viewport_pos);
                }
            }
            OrbitState::Dolly => {
                if let Some(prev) = self.dolly_last {
                    let delta = viewport_pos - prev;
                    // Three.js: dolly_in when dragging up.
                    if delta.y > 0.0 {
                        self.dolly_in(0.95_f32.powf(delta.y.abs() * self.zoom_speed));
                    } else if delta.y < 0.0 {
                        self.dolly_out(0.95_f32.powf(delta.y.abs() * self.zoom_speed));
                    }
                    self.dolly_last = Some(viewport_pos);
                }
            }
            OrbitState::None => {}
        }
    }

    /// Wheel input. Positive `y_delta` = scroll down = dolly out
    /// (zoom away). Matches Three.js's `onMouseWheel` convention.
    pub fn wheel(&mut self, y_delta: f32) {
        if y_delta < 0.0 {
            self.dolly_in(0.95_f32.powf(self.zoom_speed));
        } else if y_delta > 0.0 {
            self.dolly_out(0.95_f32.powf(self.zoom_speed));
        }
    }

    /// Bring the camera closer to the target by the given scale
    /// factor (`< 1` = closer, `> 1` = farther).
    pub fn dolly_in(&mut self, factor: f32) {
        self.scale *= factor;
    }

    /// Push the camera away from the target.
    pub fn dolly_out(&mut self, factor: f32) {
        self.scale /= factor;
    }

    /// Apply accumulated deltas to the camera. Returns `true` if
    /// the camera actually changed (useful for skipping render
    /// submission when nothing happened).
    ///
    /// `dt` is the elapsed seconds since the last `update`. Only
    /// matters when `auto_rotate` is on; otherwise can pass `0.0`.
    pub fn update(&mut self, camera: &mut impl Camera3D, dt: f32) -> bool {
        let position = camera.position();
        let target = camera.target();
        let up = camera.up();

        // Step 1: spherical accumulators (auto-rotate adds to theta).
        if self.auto_rotate && self.state == OrbitState::None {
            self.delta_theta += self.auto_rotate_speed * dt;
        }

        // Step 2: convert (position - target) to spherical, apply
        // the accumulators, clamp, convert back.
        let offset = position - target;
        let radius = offset.length();
        let (mut theta, mut phi) = cartesian_to_spherical(offset);

        theta += self.delta_theta;
        phi += self.delta_phi;
        let mut new_radius = radius * self.scale;

        theta = theta.clamp(self.min_azimuth_angle, self.max_azimuth_angle);
        phi = phi.clamp(self.min_polar_angle, self.max_polar_angle);
        // Prevent gimbal: keep at least an epsilon away from the poles.
        // The epsilon is large enough that `cos(PI - eps)` and `cos(eps)`
        // are both representable as values strictly inside (-1, +1) in
        // f32 — so callers can detect the clamp behaviourally instead of
        // having to reason about subnormals.
        phi = phi.clamp(1.0e-3, PI - 1.0e-3);
        new_radius = new_radius.clamp(self.min_distance, self.max_distance);

        let new_offset = spherical_to_cartesian(theta, phi, new_radius);
        let new_target = target + self.pan_offset;
        let new_position = new_target + new_offset;

        let position_changed = (new_position - position).length_squared() > 1e-12;
        let target_changed = (new_target - target).length_squared() > 1e-12;

        if position_changed {
            camera.set_position(new_position);
        }
        if target_changed {
            camera.set_target(new_target);
        }
        // up is left alone — orbit doesn't roll.
        let _ = up;

        // Step 3: damp or reset.
        if self.enable_damping {
            let k = 1.0 - self.damping_factor;
            self.delta_theta *= k;
            self.delta_phi *= k;
            self.pan_offset *= k;
            // Scale damps toward 1.0 (no dolly), not 0.
            self.scale = 1.0 + (self.scale - 1.0) * k;
        } else {
            self.delta_theta = 0.0;
            self.delta_phi = 0.0;
            self.pan_offset = Vec3::ZERO;
            self.scale = 1.0;
        }

        position_changed || target_changed
    }
}

/// Convert a world-space `offset = position - target` into spherical
/// `(theta, phi)` with the controller's conventions.
fn cartesian_to_spherical(offset: Vec3) -> (f32, f32) {
    let r = offset.length().max(1.0e-9);
    let theta = offset.x.atan2(offset.z);
    let phi = (offset.y / r).clamp(-1.0, 1.0).acos();
    (theta, phi)
}

/// Inverse of [`cartesian_to_spherical`].
fn spherical_to_cartesian(theta: f32, phi: f32, radius: f32) -> Vec3 {
    let sin_phi = phi.sin();
    Vec3::new(
        radius * sin_phi * theta.sin(),
        radius * phi.cos(),
        radius * sin_phi * theta.cos(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tiny test stub — direct field access, no fov override.
    #[derive(Debug, Clone)]
    struct StubCam {
        position: Vec3,
        target: Vec3,
        up: Vec3,
    }

    impl StubCam {
        fn at(distance: f32) -> Self {
            Self {
                position: Vec3::new(0.0, 0.0, distance),
                target: Vec3::ZERO,
                up: Vec3::Y,
            }
        }
    }

    impl Camera3D for StubCam {
        fn position(&self) -> Vec3 {
            self.position
        }
        fn target(&self) -> Vec3 {
            self.target
        }
        fn up(&self) -> Vec3 {
            self.up
        }
        fn set_position(&mut self, p: Vec3) {
            self.position = p;
        }
        fn set_target(&mut self, t: Vec3) {
            self.target = t;
        }
    }

    #[test]
    fn rotate_drag_horizontal_changes_azimuth_and_distance_preserved() {
        let mut ctrl = OrbitController::new();
        let mut cam = StubCam::at(5.0);
        ctrl.pointer_down_rotate(Vec2::new(100.0, 100.0));
        ctrl.pointer_drag(
            Vec2::new(200.0, 100.0),
            Vec2::new(800.0, 600.0),
            5.0,
            Vec3::X,
            Vec3::Y,
            60_f32.to_radians(),
        );
        let changed = ctrl.update(&mut cam, 0.0);
        assert!(changed, "horizontal drag must move the camera");
        let dist = (cam.position - cam.target).length();
        assert!((dist - 5.0).abs() < 1e-4, "distance preserved: {dist}");
        // Azimuth changed → camera no longer on the +z axis.
        assert!(cam.position.x.abs() > 0.01, "x component changed");
    }

    #[test]
    fn dolly_in_decreases_distance_and_clamps_at_min() {
        let mut ctrl = OrbitController::new();
        ctrl.min_distance = 2.0;
        let mut cam = StubCam::at(5.0);
        // Aggressive dolly-in.
        for _ in 0..10 {
            ctrl.dolly_in(0.5);
            ctrl.update(&mut cam, 0.0);
        }
        let dist = (cam.position - cam.target).length();
        assert!(
            (dist - 2.0).abs() < 1e-4,
            "clamped at min_distance, got {dist}"
        );
    }

    #[test]
    fn dolly_out_increases_distance_and_clamps_at_max() {
        let mut ctrl = OrbitController::new();
        ctrl.max_distance = 20.0;
        let mut cam = StubCam::at(5.0);
        for _ in 0..10 {
            ctrl.dolly_out(0.5);
            ctrl.update(&mut cam, 0.0);
        }
        let dist = (cam.position - cam.target).length();
        assert!(
            (dist - 20.0).abs() < 1e-4,
            "clamped at max_distance, got {dist}"
        );
    }

    #[test]
    fn pan_translates_both_target_and_position() {
        let mut ctrl = OrbitController::new();
        let mut cam = StubCam::at(5.0);
        let initial_offset = cam.position - cam.target;

        ctrl.pointer_down_pan(Vec2::new(100.0, 100.0));
        ctrl.pointer_drag(
            Vec2::new(150.0, 100.0),
            Vec2::new(800.0, 600.0),
            5.0,
            Vec3::X,
            Vec3::Y,
            60_f32.to_radians(),
        );
        ctrl.update(&mut cam, 0.0);

        // Target moved (non-zero).
        assert!(cam.target.length() > 1e-6, "target moved");
        // position - target invariant (just translated).
        let new_offset = cam.position - cam.target;
        assert!(
            (new_offset - initial_offset).length() < 1e-4,
            "relative offset preserved: {initial_offset:?} vs {new_offset:?}",
        );
    }

    #[test]
    fn update_with_no_input_returns_false() {
        let mut ctrl = OrbitController::new();
        let mut cam = StubCam::at(5.0);
        assert!(!ctrl.update(&mut cam, 0.0));
    }

    #[test]
    fn damping_keeps_motion_alive_after_pointer_up() {
        let mut ctrl = OrbitController::new();
        ctrl.enable_damping = true;
        let mut cam = StubCam::at(5.0);

        ctrl.pointer_down_rotate(Vec2::new(100.0, 100.0));
        ctrl.pointer_drag(
            Vec2::new(200.0, 100.0),
            Vec2::new(800.0, 600.0),
            5.0,
            Vec3::X,
            Vec3::Y,
            60_f32.to_radians(),
        );
        ctrl.pointer_up();

        let p0 = cam.position;
        ctrl.update(&mut cam, 0.016);
        let p1 = cam.position;
        ctrl.update(&mut cam, 0.016);
        let p2 = cam.position;

        // Position changes in both subsequent frames thanks to damping.
        assert!((p1 - p0).length() > 1e-6, "damped frame 1 moved");
        assert!((p2 - p1).length() > 1e-6, "damped frame 2 moved");
    }

    #[test]
    fn auto_rotate_advances_azimuth_per_tick() {
        let mut ctrl = OrbitController::new();
        ctrl.auto_rotate = true;
        let mut cam = StubCam::at(5.0);

        let p0 = cam.position;
        ctrl.update(&mut cam, 0.5);
        let p1 = cam.position;
        assert!((p1 - p0).length() > 1e-4, "auto-rotate moved the camera");
    }

    #[test]
    fn auto_rotate_blocked_during_active_rotate_drag() {
        let mut ctrl = OrbitController::new();
        ctrl.auto_rotate = true;
        ctrl.auto_rotate_speed = 100.0; // Large value to make any leak obvious.
        let mut cam = StubCam::at(5.0);
        ctrl.pointer_down_rotate(Vec2::new(100.0, 100.0));

        // No drag — accumulators stay zero, so position must not change.
        let p0 = cam.position;
        ctrl.update(&mut cam, 1.0);
        let p1 = cam.position;
        assert!(
            (p1 - p0).length() < 1e-6,
            "auto-rotate must NOT fire while user is dragging"
        );
    }

    #[test]
    fn polar_clamps_avoid_gimbal_lock() {
        let mut ctrl = OrbitController::new();
        let mut cam = StubCam::at(5.0);
        // Per Three.js convention, drag-UP tilts the camera DOWN
        // (phi increases past PI/2 toward the south pole). A huge
        // downward viewport delta drives phi toward the south pole;
        // a huge upward delta drives it toward the north pole. We
        // pick south pole here.
        ctrl.pointer_down_rotate(Vec2::new(100.0, 100.0));
        ctrl.pointer_drag(
            Vec2::new(100.0, -10_000.0),
            Vec2::new(800.0, 600.0),
            5.0,
            Vec3::X,
            Vec3::Y,
            60_f32.to_radians(),
        );
        ctrl.update(&mut cam, 0.0);
        // Camera approached the south pole but didn't flip past it —
        // y is near -radius but strictly greater (epsilon clamp).
        assert!(
            cam.position.y > -5.0,
            "didn't flip past south pole: {}",
            cam.position.y
        );
        assert!(
            cam.position.y < -4.9,
            "got near the south pole: {}",
            cam.position.y
        );
    }
}
