//! `GanttPanController` — pan a Gantt chart's timeline body while
//! keeping the date header row and project gutter column frozen
//! (spreadsheet-style).
//!
//! ## Why a Gantt-specific controller
//!
//! [`wisp_interaction::PanZoomController`] applies a single world-to-
//! screen transform to the entire scene. That's the right shape for
//! infinite canvases and freeform editors. A Gantt chart needs a
//! richer model:
//!
//! - The **body** (timeline area) scrolls in both X and Y.
//! - The **header row** (week / month band at the top) scrolls only
//!   in X — it stays glued to the top of the viewport so users can
//!   see what date column they're in while scrolling down.
//! - The **gutter column** (project / row labels on the left) scrolls
//!   only in Y — it stays glued to the left edge so users can see
//!   what row they're on while scrolling right.
//! - The **corner** (header ∩ gutter) is fully frozen — it never
//!   moves.
//!
//! Same offset state, four different output transforms. That's the
//! conceptual core of every spreadsheet's "freeze panes" UX, all the
//! way back to `VisiCalc`.
//!
//! ## Diagonal pan support
//!
//! A naive horizontal-only pan locks vertical motion silently — drag
//! down and nothing happens. `pan_drag` accumulates the full
//! `viewport_pos - anchor` delta into both `body_offset.x` and
//! `body_offset.y`, then clamps each axis against its own content
//! extent. The result is the natural "drag the canvas around in a
//! window" feel users expect from Figma / Notion / Linear.
//!
//! ## Bounds + clamping
//!
//! Body offsets are clamped so the user can't drag the content
//! outside the viewport. The clamp range is:
//!
//! - `offset.x ∈ [viewport.x - gutter_width - content.x, 0]`
//!   — `0` shows the leftmost dates; the negative bound shows the
//!   rightmost.
//! - `offset.y ∈ [viewport.y - header_height - content.y, 0]`
//!   — `0` shows the top rows; the negative bound shows the bottom.
//!
//! When content fits in its plot area on either axis, the clamp
//! collapses to `[0, 0]` on that axis (no scroll needed).
//!
//! ## Lifecycle
//!
//! 1. Build with `GanttPanController::new(header_height, gutter_width,
//!    content_size, viewport_size)`.
//! 2. On pan-button press → [`GanttPanController::pan_begin`].
//! 3. On pointer move while panning → [`GanttPanController::pan_drag`]
//!    with the new pointer position; the controller mutates the
//!    `GanttViewport` you pass in.
//! 4. On release → [`GanttPanController::pan_end`].
//! 5. On viewport resize → [`GanttPanController::set_viewport_size`]
//!    so the clamps recompute (also re-clamps the current offset).
//! 6. On content change (rows added, time range expanded) →
//!    [`GanttPanController::set_content_size`].
//!
//! Per-frame, query the four pane transforms via
//! [`GanttPanController::body_offset`] / `header_offset` /
//! `gutter_offset` / `corner_offset` and apply them to the matching
//! scene-graph subtrees.

use glam::Vec2;

/// Pan offset state owned by the host.
///
/// `body_offset` is the load-bearing field. The four pane transforms
/// derive from it via masking — see the module-level docs.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GanttViewport {
    /// Pan offset applied to the body (timeline area). Both axes
    /// move. The header pane uses only `x`; the gutter pane uses
    /// only `y`; the corner pane uses neither.
    ///
    /// Convention: dragging the pointer RIGHT increases `body_offset.x`
    /// (content shifts right, exposing earlier dates on the left).
    /// Dragging the pointer DOWN increases `body_offset.y` (content
    /// shifts down, exposing earlier rows at the top).
    pub body_offset: Vec2,
}

impl GanttViewport {
    /// Construct at origin.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Pan controller for Gantt charts with frozen header + gutter panes.
///
/// See the module-level docs for the conceptual model and lifecycle.
#[derive(Debug, Clone)]
pub struct GanttPanController {
    /// Height of the frozen header row, in viewport pixels.
    pub header_height: f32,
    /// Width of the frozen project / label gutter, in viewport
    /// pixels.
    pub gutter_width: f32,
    /// Total chart content size in pixels (computed by the host
    /// from row count × `row_height` for Y and timeline width for X).
    pub content_size: Vec2,
    /// Current viewport size in pixels.
    pub viewport_size: Vec2,

    pan_anchor: Option<Vec2>,
}

impl GanttPanController {
    /// Construct with the given header / gutter / content / viewport
    /// dimensions.
    #[must_use]
    pub fn new(
        header_height: f32,
        gutter_width: f32,
        content_size: Vec2,
        viewport_size: Vec2,
    ) -> Self {
        Self {
            header_height,
            gutter_width,
            content_size,
            viewport_size,
            pan_anchor: None,
        }
    }

    /// `true` while a pan drag is in progress.
    #[must_use]
    pub fn is_panning(&self) -> bool {
        self.pan_anchor.is_some()
    }

    /// Begin a pan drag at `pointer` (screen coords).
    pub fn pan_begin(&mut self, pointer: Vec2) {
        self.pan_anchor = Some(pointer);
    }

    /// Continue a pan drag. Translates the body offset by the delta
    /// from the last anchor, clamps to content bounds, and updates
    /// the anchor so successive calls track the pointer.
    pub fn pan_drag(&mut self, pointer: Vec2, viewport: &mut GanttViewport) {
        if let Some(anchor) = self.pan_anchor {
            viewport.body_offset += pointer - anchor;
            self.clamp(viewport);
            self.pan_anchor = Some(pointer);
        }
    }

    /// End the pan drag.
    pub fn pan_end(&mut self) {
        self.pan_anchor = None;
    }

    /// Update the viewport size (e.g. on canvas resize). Re-clamps
    /// the current offset.
    pub fn set_viewport_size(&mut self, viewport_size: Vec2, viewport: &mut GanttViewport) {
        self.viewport_size = viewport_size;
        self.clamp(viewport);
    }

    /// Update the content size (e.g. when rows are added or the time
    /// range expands). Re-clamps the current offset.
    pub fn set_content_size(&mut self, content_size: Vec2, viewport: &mut GanttViewport) {
        self.content_size = content_size;
        self.clamp(viewport);
    }

    /// Transform applied to the BODY pane (timeline area). Both axes
    /// pan.
    #[must_use]
    pub fn body_offset(&self, viewport: &GanttViewport) -> Vec2 {
        viewport.body_offset
    }

    /// Transform applied to the HEADER pane (date / week / month
    /// band). Only the X component of the body offset is used —
    /// vertical pan does not move the header.
    #[must_use]
    pub fn header_offset(&self, viewport: &GanttViewport) -> Vec2 {
        Vec2::new(viewport.body_offset.x, 0.0)
    }

    /// Transform applied to the GUTTER pane (project / row labels on
    /// the left). Only the Y component of the body offset is used —
    /// horizontal pan does not move the gutter.
    #[must_use]
    pub fn gutter_offset(&self, viewport: &GanttViewport) -> Vec2 {
        Vec2::new(0.0, viewport.body_offset.y)
    }

    /// Transform applied to the CORNER pane (header ∩ gutter, the
    /// top-left intersection). Always zero — fully frozen.
    #[must_use]
    pub fn corner_offset(&self, _viewport: &GanttViewport) -> Vec2 {
        Vec2::ZERO
    }

    /// Width of the body pane in pixels (viewport width minus the
    /// gutter). Useful for scrollbar thumb sizing.
    #[must_use]
    pub fn body_width(&self) -> f32 {
        (self.viewport_size.x - self.gutter_width).max(0.0)
    }

    /// Height of the body pane in pixels (viewport height minus the
    /// header).
    #[must_use]
    pub fn body_height(&self) -> f32 {
        (self.viewport_size.y - self.header_height).max(0.0)
    }

    /// Maximum negative `x` offset (most "scrolled right" position).
    /// Returns `0.0` when content fits.
    #[must_use]
    pub fn min_offset_x(&self) -> f32 {
        (self.body_width() - self.content_size.x).min(0.0)
    }

    /// Maximum negative `y` offset (most "scrolled down" position).
    /// Returns `0.0` when content fits.
    #[must_use]
    pub fn min_offset_y(&self) -> f32 {
        (self.body_height() - self.content_size.y).min(0.0)
    }

    /// Fraction of horizontal content currently scrolled — `0.0` =
    /// fully left, `1.0` = fully right. Returns `0.0` when content
    /// fits. Useful for scrollbar thumb position.
    #[must_use]
    pub fn scroll_fraction_x(&self, viewport: &GanttViewport) -> f32 {
        let min = self.min_offset_x();
        if min >= 0.0 {
            return 0.0;
        }
        // viewport.offset.x ∈ [min, 0], 0 = fully left, min = fully right
        (viewport.body_offset.x / min).clamp(0.0, 1.0)
    }

    /// Fraction of vertical content currently scrolled.
    #[must_use]
    pub fn scroll_fraction_y(&self, viewport: &GanttViewport) -> f32 {
        let min = self.min_offset_y();
        if min >= 0.0 {
            return 0.0;
        }
        (viewport.body_offset.y / min).clamp(0.0, 1.0)
    }

    /// Apply the bounds clamp in-place. Public so hosts that have
    /// changed `body_offset` directly (programmatic scroll, keyboard
    /// nav) can re-clamp.
    pub fn clamp(&self, viewport: &mut GanttViewport) {
        let min_x = self.min_offset_x();
        let min_y = self.min_offset_y();
        viewport.body_offset.x = viewport.body_offset.x.clamp(min_x, 0.0);
        viewport.body_offset.y = viewport.body_offset.y.clamp(min_y, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference fixture: header 60, gutter 180, content 2000×500,
    /// viewport 800×400.
    fn ctrl() -> GanttPanController {
        GanttPanController::new(
            60.0,
            180.0,
            Vec2::new(2000.0, 500.0),
            Vec2::new(800.0, 400.0),
        )
    }

    #[test]
    fn default_viewport_is_origin() {
        let v = GanttViewport::new();
        assert_eq!(v.body_offset, Vec2::ZERO);
    }

    #[test]
    fn body_height_and_width_subtract_frozen_panes() {
        let c = ctrl();
        assert!((c.body_width() - 620.0).abs() < 1e-4); // 800 - 180
        assert!((c.body_height() - 340.0).abs() < 1e-4); // 400 - 60
    }

    #[test]
    fn pan_drag_translates_body_offset_both_axes() {
        let mut c = ctrl();
        let mut v = GanttViewport::new();
        c.pan_begin(Vec2::new(100.0, 100.0));
        assert!(c.is_panning());
        // Drag right + down — both axes should accumulate.
        c.pan_drag(Vec2::new(80.0, 50.0), &mut v);
        // Pointer went LEFT (delta.x = -20) and UP (delta.y = -50).
        // body_offset accumulates the delta, then clamps.
        assert!((v.body_offset.x - -20.0).abs() < 1e-4);
        assert!((v.body_offset.y - -50.0).abs() < 1e-4);
        c.pan_end();
        assert!(!c.is_panning());
    }

    #[test]
    fn diagonal_pan_works_user_bug_repro() {
        // User reported: "I can pan but can't go downwards or diagonal."
        // The bug shape: a horizontal-only controller silently drops
        // delta.y. This test enforces both axes move on diagonal drag.
        let mut c = ctrl();
        let mut v = GanttViewport::new();
        c.pan_begin(Vec2::new(500.0, 200.0));
        c.pan_drag(Vec2::new(440.0, 160.0), &mut v);
        // Pointer moved (-60, -40) — both axes must accumulate.
        assert!(
            v.body_offset.x < -1e-4,
            "x must move on diagonal drag: {:?}",
            v.body_offset
        );
        assert!(
            v.body_offset.y < -1e-4,
            "y must move on diagonal drag: {:?}",
            v.body_offset
        );
    }

    #[test]
    fn pan_clamps_to_content_bounds_horizontally() {
        let mut c = ctrl();
        let mut v = GanttViewport::new();
        c.pan_begin(Vec2::ZERO);
        // Drag the pointer far enough left that body_offset.x would
        // go past `min_offset_x`. Should clamp at min, not exceed.
        c.pan_drag(Vec2::new(-5_000.0, 0.0), &mut v);
        let min_x = c.min_offset_x();
        assert!(min_x < 0.0, "content must exceed body width for this test");
        assert!(
            (v.body_offset.x - min_x).abs() < 1e-4,
            "clamped at min_x={min_x}, got {}",
            v.body_offset.x
        );
        // And the opposite direction clamps at 0.
        c.pan_drag(Vec2::new(50_000.0, 0.0), &mut v);
        assert!(v.body_offset.x.abs() < 1e-4);
    }

    #[test]
    fn pan_clamps_to_content_bounds_vertically() {
        let mut c = ctrl();
        let mut v = GanttViewport::new();
        c.pan_begin(Vec2::ZERO);
        c.pan_drag(Vec2::new(0.0, -5_000.0), &mut v);
        let min_y = c.min_offset_y();
        assert!(min_y < 0.0);
        assert!((v.body_offset.y - min_y).abs() < 1e-4);
        c.pan_drag(Vec2::new(0.0, 50_000.0), &mut v);
        assert!(v.body_offset.y.abs() < 1e-4);
    }

    #[test]
    fn pan_clamps_collapse_when_content_fits() {
        // Content smaller than body — no scroll possible.
        let small = GanttPanController::new(
            60.0,
            180.0,
            Vec2::new(100.0, 100.0),
            Vec2::new(800.0, 400.0),
        );
        assert!((small.min_offset_x()).abs() < 1e-4);
        assert!((small.min_offset_y()).abs() < 1e-4);
        let mut v = GanttViewport::new();
        let mut s = small.clone();
        s.pan_begin(Vec2::ZERO);
        s.pan_drag(Vec2::new(-9_999.0, -9_999.0), &mut v);
        assert_eq!(v.body_offset, Vec2::ZERO);
    }

    #[test]
    fn frozen_pane_transforms_mask_correctly() {
        let c = ctrl();
        let v = GanttViewport {
            body_offset: Vec2::new(-100.0, -50.0),
        };
        // Body: both axes.
        assert_eq!(c.body_offset(&v), Vec2::new(-100.0, -50.0));
        // Header: only x.
        assert_eq!(c.header_offset(&v), Vec2::new(-100.0, 0.0));
        // Gutter: only y.
        assert_eq!(c.gutter_offset(&v), Vec2::new(0.0, -50.0));
        // Corner: fully frozen.
        assert_eq!(c.corner_offset(&v), Vec2::ZERO);
    }

    #[test]
    fn scroll_fraction_is_zero_at_start_and_one_at_end() {
        let c = ctrl();
        let mut v = GanttViewport::new();
        assert!((c.scroll_fraction_x(&v) - 0.0).abs() < 1e-4);
        assert!((c.scroll_fraction_y(&v) - 0.0).abs() < 1e-4);
        v.body_offset = Vec2::new(c.min_offset_x(), c.min_offset_y());
        assert!((c.scroll_fraction_x(&v) - 1.0).abs() < 1e-4);
        assert!((c.scroll_fraction_y(&v) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn pan_drag_without_pan_begin_is_noop() {
        let mut c = ctrl();
        let mut v = GanttViewport::new();
        c.pan_drag(Vec2::new(50.0, 50.0), &mut v);
        assert_eq!(v.body_offset, Vec2::ZERO);
    }

    #[test]
    fn set_viewport_size_reclamps_current_offset() {
        let mut c = ctrl();
        let mut v = GanttViewport::new();
        c.pan_begin(Vec2::ZERO);
        c.pan_drag(Vec2::new(-1000.0, -500.0), &mut v);
        // Now shrink the viewport — content overflows MORE, so clamps
        // get tighter. The current offset should still be valid.
        let old_min_x = c.min_offset_x();
        c.set_viewport_size(Vec2::new(400.0, 200.0), &mut v);
        let new_min_x = c.min_offset_x();
        // body_width shrunk so min_offset_x is closer to (more negative).
        // After reclamp, offset still in [new_min, 0] range.
        assert!(v.body_offset.x >= new_min_x - 1e-4);
        assert!(v.body_offset.x <= 0.0);
        // The OLD min was less negative; verify the new clamp moved
        // farther negative.
        assert!(new_min_x <= old_min_x);
    }

    #[test]
    fn set_content_size_grows_clamp_range() {
        let mut c = ctrl();
        let mut v = GanttViewport::new();
        c.pan_begin(Vec2::ZERO);
        c.pan_drag(Vec2::new(-2000.0, 0.0), &mut v);
        let pinned = v.body_offset.x;
        // Grow content — clamp loosens, offset stays where it was.
        c.set_content_size(Vec2::new(5000.0, 500.0), &mut v);
        assert!((v.body_offset.x - pinned).abs() < 1e-4);
    }
}
