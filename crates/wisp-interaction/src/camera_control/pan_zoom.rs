//! `PanZoomController` — Figma-style 2D pan + zoom-around-pointer.
//!
//! Wisp's 2D scene-graph has no "camera"; the equivalent is a single
//! root container with a uniform `transform`. The controller mutates
//! a [`Viewport2D`] (a tiny struct that the host wires to whatever
//! root-container transform it owns) so the same code works for
//! - the recorder's editor canvas (pan a Figma-style infinite plane),
//! - the chart inspector (pan/zoom around a chart),
//! - any other 2D scene that has a single root transform.
//!
//! Math:
//! - World-to-screen: `screen = world * zoom + offset`.
//! - Zoom-around-pointer (the load-bearing trick): pick a `pivot` in
//!   SCREEN coords. Convert pivot to world via the OLD transform;
//!   change `zoom`; solve `offset` so the same world point still maps
//!   to the same `pivot`. Result:
//!   `offset' = pivot - world_pivot * zoom'`.
//!
//! Skip-list (defer to follow-ups):
//! - Rotation (Figma added rotate gestures recently; not needed for
//!   our recorder).
//! - Two-finger pinch — synthesised in wisp-interaction-web's adapter
//!   from the two `PointerId::Touch` event streams, not in here.

use glam::Vec2;

/// 2D world-to-screen transform under the controller's control.
///
/// `screen = world * zoom + offset`. Hosts initialise once and pass a
/// mutable reference to every controller call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport2D {
    /// Screen-space offset (translation applied after scale).
    pub offset: Vec2,
    /// Uniform scale factor (`> 0`).
    pub zoom: f32,
}

impl Viewport2D {
    /// Identity transform.
    #[must_use]
    pub fn identity() -> Self {
        Self {
            offset: Vec2::ZERO,
            zoom: 1.0,
        }
    }

    /// Convert a screen-space point to world space.
    #[must_use]
    pub fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        (screen - self.offset) / self.zoom
    }

    /// Convert a world-space point to screen space.
    #[must_use]
    pub fn world_to_screen(&self, world: Vec2) -> Vec2 {
        world * self.zoom + self.offset
    }
}

impl Default for Viewport2D {
    fn default() -> Self {
        Self::identity()
    }
}

/// Pan + zoom-around-pointer controller. Wire to your adapter:
///
/// 1. On pan-button press → [`PanZoomController::pan_begin`].
/// 2. On pointer move while a pan is in progress →
///    [`PanZoomController::pan_drag`].
/// 3. On pan release → [`PanZoomController::pan_end`].
/// 4. On wheel / pinch → [`PanZoomController::zoom_at_pointer`].
///
/// Mutations land directly on the supplied `Viewport2D`; no
/// `update`-tick is required.
#[derive(Debug, Default, Clone)]
pub struct PanZoomController {
    /// Per-tick wheel-zoom multiplier. Default `1.1`: every wheel
    /// notch zooms by 10%, matching Figma + Google Maps.
    pub zoom_step: f32,
    /// Lower bound on `zoom`.
    pub min_zoom: f32,
    /// Upper bound on `zoom`.
    pub max_zoom: f32,

    pan_anchor: Option<Vec2>,
}

impl PanZoomController {
    /// Construct with Figma defaults (`zoom_step` = 1.1, zoom range 0.01..100).
    #[must_use]
    pub fn new() -> Self {
        Self {
            zoom_step: 1.1,
            min_zoom: 0.01,
            max_zoom: 100.0,
            pan_anchor: None,
        }
    }

    /// `true` if a pan drag is currently active.
    #[must_use]
    pub fn is_panning(&self) -> bool {
        self.pan_anchor.is_some()
    }

    /// Start a pan drag at `pointer` (screen coords).
    pub fn pan_begin(&mut self, pointer: Vec2) {
        self.pan_anchor = Some(pointer);
    }

    /// Continue a pan: translate the viewport by the delta from the
    /// last anchor. Updates the anchor so successive calls keep
    /// tracking the pointer.
    pub fn pan_drag(&mut self, pointer: Vec2, viewport: &mut Viewport2D) {
        if let Some(anchor) = self.pan_anchor {
            viewport.offset += pointer - anchor;
            self.pan_anchor = Some(pointer);
        }
    }

    /// End a pan drag.
    pub fn pan_end(&mut self) {
        self.pan_anchor = None;
    }

    /// Zoom around a screen-space pivot by `factor` (>1 zooms in).
    /// Used for wheel + pinch-zoom alike. The pivot stays anchored:
    /// the world point under the pivot before the zoom is the same
    /// world point under it after.
    pub fn zoom_at_pointer(&self, pivot: Vec2, factor: f32, viewport: &mut Viewport2D) {
        let world_pivot = viewport.screen_to_world(pivot);
        let new_zoom = (viewport.zoom * factor).clamp(self.min_zoom, self.max_zoom);
        viewport.zoom = new_zoom;
        // Solve `pivot = world_pivot * zoom + offset` for `offset`.
        viewport.offset = pivot - world_pivot * new_zoom;
    }

    /// Convenience: take a wheel `y_delta` (typical browser
    /// convention: positive = scroll down = zoom out) and apply.
    pub fn wheel_zoom(&self, pivot: Vec2, y_delta: f32, viewport: &mut Viewport2D) {
        if y_delta == 0.0 {
            return;
        }
        let factor = if y_delta < 0.0 {
            self.zoom_step
        } else {
            1.0 / self.zoom_step
        };
        self.zoom_at_pointer(pivot, factor, viewport);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_round_trips_screen_to_world() {
        let v = Viewport2D::identity();
        let p = Vec2::new(123.0, 456.0);
        assert_eq!(v.screen_to_world(p), p);
        assert_eq!(v.world_to_screen(p), p);
    }

    #[test]
    fn pan_translates_offset_by_pointer_delta() {
        let mut ctrl = PanZoomController::new();
        let mut v = Viewport2D::identity();
        ctrl.pan_begin(Vec2::new(100.0, 100.0));
        assert!(ctrl.is_panning());
        ctrl.pan_drag(Vec2::new(150.0, 200.0), &mut v);
        assert_eq!(v.offset, Vec2::new(50.0, 100.0));
        ctrl.pan_drag(Vec2::new(160.0, 210.0), &mut v);
        // Delta from the updated anchor.
        assert_eq!(v.offset, Vec2::new(60.0, 110.0));
        ctrl.pan_end();
        assert!(!ctrl.is_panning());
    }

    #[test]
    fn zoom_at_pointer_keeps_pivot_anchored() {
        let ctrl = PanZoomController::new();
        let mut v = Viewport2D::identity();
        let pivot = Vec2::new(300.0, 200.0);
        // World point that's currently under the pivot.
        let world_before = v.screen_to_world(pivot);
        ctrl.zoom_at_pointer(pivot, 2.0, &mut v);
        // After the zoom, the same world point should still be under
        // the pivot — the whole point of zoom-around-pointer.
        let world_after = v.screen_to_world(pivot);
        assert!(
            (world_before - world_after).length() < 1e-4,
            "world pivot drifted: {world_before:?} → {world_after:?}"
        );
        // Zoom actually changed.
        assert!((v.zoom - 2.0).abs() < 1e-6);
    }

    #[test]
    fn zoom_clamps_at_max_and_min() {
        let ctrl = PanZoomController::new();
        let mut v = Viewport2D::identity();
        for _ in 0..50 {
            ctrl.zoom_at_pointer(Vec2::ZERO, 10.0, &mut v);
        }
        assert!((v.zoom - ctrl.max_zoom).abs() < 1e-4, "clamped at max");

        let mut v = Viewport2D::identity();
        for _ in 0..50 {
            ctrl.zoom_at_pointer(Vec2::ZERO, 0.1, &mut v);
        }
        assert!((v.zoom - ctrl.min_zoom).abs() < 1e-4, "clamped at min");
    }

    #[test]
    fn wheel_zoom_in_vs_out_picks_step_and_inverse() {
        let ctrl = PanZoomController::new();
        let mut v = Viewport2D::identity();
        ctrl.wheel_zoom(Vec2::ZERO, -1.0, &mut v);
        assert!((v.zoom - ctrl.zoom_step).abs() < 1e-6);
        ctrl.wheel_zoom(Vec2::ZERO, 1.0, &mut v);
        assert!((v.zoom - 1.0).abs() < 1e-6, "round-trip back to 1.0");
    }

    #[test]
    fn pan_drag_without_begin_is_noop() {
        let mut ctrl = PanZoomController::new();
        let mut v = Viewport2D::identity();
        ctrl.pan_drag(Vec2::new(50.0, 50.0), &mut v);
        assert_eq!(v.offset, Vec2::ZERO);
    }

    #[test]
    fn zero_wheel_delta_no_op() {
        let ctrl = PanZoomController::new();
        let mut v = Viewport2D::identity();
        ctrl.wheel_zoom(Vec2::new(10.0, 10.0), 0.0, &mut v);
        assert_eq!(v, Viewport2D::identity());
    }
}
