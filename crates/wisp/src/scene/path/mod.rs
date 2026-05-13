//! Path commands + adaptive Bezier flattening (M-VEC.10 / AUT-62).
//!
//! Supports `move_to`, `line_to`, `quad_to`, `cubic_to`, `close`.
//! Two consumer paths:
//!
//! - **Flatten to a polygon** for masking via
//!   [`crate::scene::VectorShape::Path`].
//! - **Render as a stroked `Graphics`** for visible arrow / freehand
//!   geometry (joins between segments are butt-style for V1; mitered
//!   joins are a future enhancement).
//!
//! Adaptive subdivision uses a flatness test on the control polygon:
//! a Bezier curve is "flat enough" when the maximum perpendicular
//! distance from a control point to the chord is below `tolerance`.
//!
//! Boolean ops on paths (`union`, `intersection`, `difference`,
//! `xor`) live in the [`boolean`] submodule (M-BOOL.0 / AUT-161).

pub mod boolean;

use glam::Vec2;

use crate::color::Color;
use crate::scene::graphics::{Fill, Graphics};

/// One command in a path. The path is a sequence of these; rendering
/// / flattening walks them in order.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum PathCommand {
    /// Lift the pen and move to `p`. Starts a new subpath.
    MoveTo(Vec2),
    /// Draw a straight line from the current point to `p`.
    LineTo(Vec2),
    /// Quadratic Bezier from current point through `control` to
    /// `end`.
    QuadTo {
        /// Control point.
        control: Vec2,
        /// End point.
        end: Vec2,
    },
    /// Cubic Bezier from current point through `c1` and `c2` to
    /// `end`.
    CubicTo {
        /// First control point.
        c1: Vec2,
        /// Second control point.
        c2: Vec2,
        /// End point.
        end: Vec2,
    },
    /// Close the current subpath with a line back to its `MoveTo`.
    Close,
}

/// Builder for a [`Path`]. Each method returns `Self` for chaining.
#[derive(Debug, Clone, Default)]
pub struct PathBuilder {
    commands: Vec<PathCommand>,
}

impl PathBuilder {
    /// Construct an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a `MoveTo` command.
    #[must_use]
    pub fn move_to(mut self, p: Vec2) -> Self {
        self.commands.push(PathCommand::MoveTo(p));
        self
    }

    /// Append a `LineTo` command.
    #[must_use]
    pub fn line_to(mut self, p: Vec2) -> Self {
        self.commands.push(PathCommand::LineTo(p));
        self
    }

    /// Append a `QuadTo` command.
    #[must_use]
    pub fn quad_to(mut self, control: Vec2, end: Vec2) -> Self {
        self.commands.push(PathCommand::QuadTo { control, end });
        self
    }

    /// Append a `CubicTo` command.
    #[must_use]
    pub fn cubic_to(mut self, c1: Vec2, c2: Vec2, end: Vec2) -> Self {
        self.commands.push(PathCommand::CubicTo { c1, c2, end });
        self
    }

    /// Append a `Close` command.
    #[must_use]
    pub fn close(mut self) -> Self {
        self.commands.push(PathCommand::Close);
        self
    }

    /// Finish the builder.
    #[must_use]
    pub fn build(self) -> Path {
        Path {
            commands: self.commands,
        }
    }
}

/// Owned path data — list of [`PathCommand`]s plus consumer methods.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    commands: Vec<PathCommand>,
}

impl Path {
    /// Construct from a raw command vector. Prefer
    /// [`PathBuilder`] for fluent construction.
    #[must_use]
    pub fn from_commands(commands: Vec<PathCommand>) -> Self {
        Self { commands }
    }

    /// Borrow the underlying commands.
    #[must_use]
    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    /// Flatten Beziers to line segments **per subpath**, preserving
    /// `MoveTo` boundaries. Each `MoveTo` starts a new
    /// `Vec<Vec2>`; subsequent `LineTo` / `QuadTo` / `CubicTo`
    /// commands append into the current subpath; `Close` appends a
    /// final point back to the subpath's start.
    ///
    /// This is the multi-subpath variant of [`Path::flatten`]
    /// (M-BOOL.7 / AUT-168). Boolean ops, masks, and any consumer
    /// that needs to distinguish disjoint regions of a single
    /// `Path` (e.g. holes, multi-shape clips) should use this
    /// instead of the flat-concatenation [`Path::flatten`].
    ///
    /// `tolerance` semantics match [`Path::flatten`]. Empty paths
    /// return an empty `Vec`; paths with only a `MoveTo` return a
    /// single one-element subpath.
    #[must_use]
    pub fn flatten_subpaths(&self, tolerance: f32) -> Vec<Vec<Vec2>> {
        let mut subs: Vec<Vec<Vec2>> = Vec::new();
        let mut current: Option<Vec<Vec2>> = None;
        let mut subpath_start: Vec2 = Vec2::ZERO;
        let mut last_point: Vec2 = Vec2::ZERO;

        for cmd in &self.commands {
            match *cmd {
                PathCommand::MoveTo(p) => {
                    if let Some(v) = current.take()
                        && !v.is_empty()
                    {
                        subs.push(v);
                    }
                    current = Some(vec![p]);
                    subpath_start = p;
                    last_point = p;
                }
                PathCommand::LineTo(p) => {
                    if let Some(v) = current.as_mut() {
                        v.push(p);
                    }
                    last_point = p;
                }
                PathCommand::QuadTo { control, end } => {
                    if let Some(v) = current.as_mut() {
                        flatten_quad(last_point, control, end, tolerance, v);
                    }
                    last_point = end;
                }
                PathCommand::CubicTo { c1, c2, end } => {
                    if let Some(v) = current.as_mut() {
                        flatten_cubic(last_point, c1, c2, end, tolerance, v);
                    }
                    last_point = end;
                }
                PathCommand::Close => {
                    if let Some(v) = current.as_mut() {
                        v.push(subpath_start);
                    }
                    last_point = subpath_start;
                }
            }
        }
        if let Some(v) = current
            && !v.is_empty()
        {
            subs.push(v);
        }
        subs
    }

    /// Flatten Beziers to line segments and return the resulting
    /// polygon (a `Vec<Vec2>`). `tolerance` is the maximum
    /// perpendicular distance from a control point to the chord
    /// before the curve subdivides further. NDC units; `0.005` is a
    /// reasonable default.
    ///
    /// Open subpaths (no `Close`) end at the last point. `Close`
    /// adds a line back to the most recent `MoveTo`.
    ///
    /// For consumers that need to distinguish individual subpaths
    /// (boolean ops, holes, multi-shape masks), use
    /// [`Path::flatten_subpaths`].
    #[must_use]
    pub fn flatten(&self, tolerance: f32) -> Vec<Vec2> {
        let mut out: Vec<Vec2> = Vec::new();
        let mut current: Vec2 = Vec2::ZERO;
        let mut subpath_start: Vec2 = Vec2::ZERO;

        for cmd in &self.commands {
            match *cmd {
                PathCommand::MoveTo(p) => {
                    out.push(p);
                    current = p;
                    subpath_start = p;
                }
                PathCommand::LineTo(p) => {
                    out.push(p);
                    current = p;
                }
                PathCommand::QuadTo { control, end } => {
                    flatten_quad(current, control, end, tolerance, &mut out);
                    current = end;
                }
                PathCommand::CubicTo { c1, c2, end } => {
                    flatten_cubic(current, c1, c2, end, tolerance, &mut out);
                    current = end;
                }
                PathCommand::Close => {
                    out.push(subpath_start);
                    current = subpath_start;
                }
            }
        }
        out
    }

    /// Rasterize this path as a stroked [`Graphics`] node. Each
    /// flattened segment becomes a [`Graphics::draw_line`] call.
    /// `width` is the stroke width in NDC units; `tolerance` controls
    /// flatness of curves.
    #[must_use]
    pub fn stroke_to_graphics(&self, width: f32, color: Color, tolerance: f32) -> Graphics {
        let pts = self.flatten(tolerance);
        let mut g = Graphics::new();
        if pts.len() < 2 {
            return g;
        }
        // `draw_line` colors with the current fill (strokes apply to
        // fillable primitives only). Set fill to the stroke color
        // before emitting the segments.
        g.fill(Fill::Solid(color));
        for pair in pts.windows(2) {
            g.draw_line(pair[0], pair[1], width);
        }
        g
    }

    /// Convert to a [`crate::scene::VectorShape::Path`] for use as
    /// a mask via the path-mask machinery. Caller is responsible for
    /// keeping the vertex count under `MAX_PATH_POINTS` (32 vertices —
    /// see `path_clip.wgsl`). Anything beyond the cap is silently
    /// truncated by the path-mask shader.
    #[must_use]
    pub fn to_mask_polygon(&self, tolerance: f32) -> Vec<Vec2> {
        self.flatten(tolerance)
    }
}

/// Adaptive flattening of a quadratic Bezier. Subdivides until the
/// control point is within `tolerance` of the chord.
fn flatten_quad(p0: Vec2, p1: Vec2, p2: Vec2, tolerance: f32, out: &mut Vec<Vec2>) {
    if perp_distance(p1, p0, p2) <= tolerance {
        out.push(p2);
        return;
    }
    let p01 = (p0 + p1) * 0.5;
    let p12 = (p1 + p2) * 0.5;
    let mid = (p01 + p12) * 0.5;
    flatten_quad(p0, p01, mid, tolerance, out);
    flatten_quad(mid, p12, p2, tolerance, out);
}

/// Adaptive flattening of a cubic Bezier — same idea, subdivide
/// until both control points are within `tolerance` of the chord.
fn flatten_cubic(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, tolerance: f32, out: &mut Vec<Vec2>) {
    let d1 = perp_distance(p1, p0, p3);
    let d2 = perp_distance(p2, p0, p3);
    if d1.max(d2) <= tolerance {
        out.push(p3);
        return;
    }
    let q01 = (p0 + p1) * 0.5;
    let q12 = (p1 + p2) * 0.5;
    let q23 = (p2 + p3) * 0.5;
    let r012 = (q01 + q12) * 0.5;
    let r123 = (q12 + q23) * 0.5;
    let mid = (r012 + r123) * 0.5;
    flatten_cubic(p0, q01, r012, mid, tolerance, out);
    flatten_cubic(mid, r123, q23, p3, tolerance, out);
}

/// Perpendicular distance from `p` to the line through `a` and `b`.
fn perp_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len = ab.length();
    if len < f32::EPSILON {
        return (p - a).length();
    }
    // |(p - a) × (b - a)| / |b - a|, in 2D the cross is a scalar.
    let cross = (p.x - a.x) * ab.y - (p.y - a.y) * ab.x;
    cross.abs() / len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_only_path_produces_control_points() {
        let path = PathBuilder::new()
            .move_to(Vec2::new(0.0, 0.0))
            .line_to(Vec2::new(1.0, 0.0))
            .line_to(Vec2::new(1.0, 1.0))
            .build();
        let pts = path.flatten(0.01);
        assert_eq!(pts.len(), 3);
        assert!((pts[2].y - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn close_appends_back_to_start() {
        let path = PathBuilder::new()
            .move_to(Vec2::new(0.0, 0.0))
            .line_to(Vec2::new(0.5, 0.0))
            .line_to(Vec2::new(0.5, 0.5))
            .close()
            .build();
        let pts = path.flatten(0.01);
        // 3 explicit points + close → start = 4 points.
        assert_eq!(pts.len(), 4);
        assert!((pts[3].x - 0.0).abs() < f32::EPSILON);
        assert!((pts[3].y - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn quad_subdivides_when_control_off_chord() {
        // Quad with control well off the chord — should produce
        // multiple subdivision points.
        let path = PathBuilder::new()
            .move_to(Vec2::new(0.0, 0.0))
            .quad_to(Vec2::new(0.5, 1.0), Vec2::new(1.0, 0.0))
            .build();
        let pts = path.flatten(0.01);
        assert!(
            pts.len() > 2,
            "off-chord quad should subdivide, got {} pts",
            pts.len()
        );
    }

    #[test]
    fn cubic_with_zero_curvature_emits_two_points() {
        // Cubic where all control points are on the chord — flat,
        // no subdivision.
        let path = PathBuilder::new()
            .move_to(Vec2::new(0.0, 0.0))
            .cubic_to(
                Vec2::new(0.33, 0.0),
                Vec2::new(0.66, 0.0),
                Vec2::new(1.0, 0.0),
            )
            .build();
        let pts = path.flatten(0.01);
        assert_eq!(pts.len(), 2, "flat cubic should not subdivide");
    }

    #[test]
    fn perp_distance_computes_correctly() {
        let d = perp_distance(Vec2::new(0.0, 1.0), Vec2::ZERO, Vec2::new(1.0, 0.0));
        // Distance from (0,1) to the x-axis = 1.0.
        assert!((d - 1.0).abs() < 1e-3);
    }

    #[test]
    fn flatten_subpaths_separates_two_movetos() {
        let path = PathBuilder::new()
            .move_to(Vec2::new(0.0, 0.0))
            .line_to(Vec2::new(0.5, 0.0))
            .line_to(Vec2::new(0.5, 0.5))
            .close()
            .move_to(Vec2::new(1.0, 1.0))
            .line_to(Vec2::new(1.5, 1.0))
            .line_to(Vec2::new(1.5, 1.5))
            .close()
            .build();
        let subs = path.flatten_subpaths(0.01);
        assert_eq!(subs.len(), 2, "two MoveTos → two subpaths");
        // Each subpath has 4 points (3 explicit + close-back-to-start).
        assert_eq!(subs[0].len(), 4);
        assert_eq!(subs[1].len(), 4);
        // First subpath stays in its own bucket — no point from the
        // second subpath leaked in.
        assert!(subs[0].iter().all(|p| p.x < 0.6 && p.y < 0.6));
        assert!(subs[1].iter().all(|p| p.x > 0.9 && p.y > 0.9));
    }

    #[test]
    fn flatten_subpaths_preserves_bezier_curvature_per_subpath() {
        let path = PathBuilder::new()
            .move_to(Vec2::new(0.0, 0.0))
            .quad_to(Vec2::new(0.5, 1.0), Vec2::new(1.0, 0.0))
            .close()
            .move_to(Vec2::new(2.0, 0.0))
            .line_to(Vec2::new(3.0, 0.0))
            .close()
            .build();
        let subs = path.flatten_subpaths(0.01);
        assert_eq!(subs.len(), 2);
        // First subpath subdivides the quad → > 3 points.
        assert!(
            subs[0].len() > 3,
            "quad subpath should subdivide, got {} pts",
            subs[0].len()
        );
        // Second subpath is just two lines → 3 points (2 explicit +
        // close back to start).
        assert_eq!(subs[1].len(), 3);
    }

    #[test]
    fn flatten_subpaths_empty_path_is_empty() {
        let path = Path::from_commands(Vec::new());
        assert!(path.flatten_subpaths(0.01).is_empty());
    }

    #[test]
    fn stroke_to_graphics_emits_line_per_segment() {
        let path = PathBuilder::new()
            .move_to(Vec2::new(0.0, 0.0))
            .line_to(Vec2::new(1.0, 0.0))
            .line_to(Vec2::new(1.0, 1.0))
            .build();
        let g = path.stroke_to_graphics(0.02, Color::WHITE, 0.01);
        // 3 points → 2 segments → 2 lines.
        assert_eq!(g.primitives.len(), 2);
    }
}
