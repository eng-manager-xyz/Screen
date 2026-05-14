//! `Graphics` — vector primitives (rect, rounded rect, ellipse, line, stroke).
//!
//! Composed over [`Container`]. A `Graphics` holds a list of primitives that
//! are rendered with a shared SDF-based pipeline. Primitives may have an
//! optional [`Stroke`] outline, set via [`Graphics::stroke`] before drawing.

use glam::Vec2;

use crate::color::Color;
use crate::math::Rect;
use crate::scene::container::Container;

/// Fill style for a `Graphics` primitive.
///
/// Gradient endpoints (`start`/`end` for linear, `center`/`radius` for radial)
/// are in primitive-local coordinates: `[-half_extents, +half_extents]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fill {
    /// Single solid color (linear-srgb f32).
    Solid(Color),
    /// Linear gradient between two colors along `start → end`.
    LinearGradient {
        /// Gradient start position (color = `color_a`).
        start: Vec2,
        /// Gradient end position (color = `color_b`).
        end: Vec2,
        /// Color at `start`.
        color_a: Color,
        /// Color at `end`.
        color_b: Color,
    },
    /// Radial gradient from `center` outward to `radius`, blending two colors.
    RadialGradient {
        /// Gradient center.
        center: Vec2,
        /// Distance at which the gradient reaches `color_b`.
        radius: f32,
        /// Color at `center`.
        color_a: Color,
        /// Color at `radius`.
        color_b: Color,
    },
}

impl Default for Fill {
    fn default() -> Self {
        Self::Solid(Color::WHITE)
    }
}

/// Outline style for fillable primitives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    /// Outline thickness in primitive-local units.
    pub width: f32,
    /// Outline color.
    pub color: Color,
}

impl Stroke {
    /// Construct a new `Stroke`.
    #[must_use]
    pub const fn new(width: f32, color: Color) -> Self {
        Self { width, color }
    }
}

/// One drawable primitive within a `Graphics` node.
///
/// Internal — the public API is `Graphics::draw_*`.
#[derive(Debug, Clone)]
pub(crate) enum Primitive {
    /// Axis-aligned rectangle. `radius == 0.0` = sharp corners.
    RoundedRect {
        rect: Rect,
        radius: f32,
        fill: Fill,
        stroke: Option<Stroke>,
    },
    /// Axis-aligned ellipse defined by center and radii.
    Ellipse {
        center: Vec2,
        radii: Vec2,
        fill: Fill,
        stroke: Option<Stroke>,
    },
    /// Line segment rendered as a rotated rect of given width.
    Line {
        from: Vec2,
        to: Vec2,
        width: f32,
        fill: Fill,
    },
    /// Annular sector — pie slice (`r_inner = 0`), donut slice
    /// (`r_inner > 0`), or stroked arc (`r_outer - r_inner` =
    /// stroke band thickness).
    ///
    /// Angles are radians; `0` aligns with `+x`, CCW positive.
    /// `start_angle < end_angle`; angular span clamps to
    /// `[0, 2π]`. When `end - start ≥ 2π` the primitive becomes a
    /// full ring (or filled disc when `r_inner = 0`).
    AnnularSector {
        center: Vec2,
        r_inner: f32,
        r_outer: f32,
        start_angle: f32,
        end_angle: f32,
        fill: Fill,
        stroke: Option<Stroke>,
    },
    /// Convex polygon defined by a vertex list in
    /// counter-clockwise winding order. The polygon is implicitly
    /// closed — the last vertex connects back to the first.
    ///
    /// **Convex-only for v1.** Non-convex input is undefined
    /// behaviour today (fan triangulation produces visible
    /// overlap). Non-convex support deferred to a follow-on
    /// tessellator chunk.
    ///
    /// No edge anti-aliasing in v1 — polygon edges show pixel
    /// jaggies. Apply an outline stroke (separate `draw_line`
    /// calls along the perimeter) when crisp edges matter; SDF
    /// AA polish is a follow-on.
    Polygon { vertices: Vec<Vec2>, fill: Fill },
}

/// Vector-primitive node. Holds an ordered list of primitives sharing the
/// node's container transform.
#[derive(Debug, Clone, Default)]
pub struct Graphics {
    /// Transform / visibility container.
    pub container: Container,
    current_fill: Fill,
    current_stroke: Option<Stroke>,
    pub(crate) primitives: Vec<Primitive>,
}

impl Graphics {
    /// Construct an empty `Graphics` with default (white solid) fill and no stroke.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the fill used by subsequent `draw_*` calls.
    pub fn fill(&mut self, fill: Fill) -> &mut Self {
        self.current_fill = fill;
        self
    }

    /// Set the stroke used by subsequent fillable `draw_*` calls.
    /// Pass `None` to clear; pass `Some(...)` to enable an outline.
    pub fn stroke(&mut self, stroke: Option<Stroke>) -> &mut Self {
        self.current_stroke = stroke;
        self
    }

    /// Append a filled (and optionally stroked) rectangle primitive.
    pub fn draw_rect(&mut self, rect: Rect) -> &mut Self {
        self.primitives.push(Primitive::RoundedRect {
            rect,
            radius: 0.0,
            fill: self.current_fill,
            stroke: self.current_stroke,
        });
        self
    }

    /// Append a filled (and optionally stroked) rounded rectangle primitive.
    pub fn draw_rounded_rect(&mut self, rect: Rect, radius: f32) -> &mut Self {
        self.primitives.push(Primitive::RoundedRect {
            rect,
            radius,
            fill: self.current_fill,
            stroke: self.current_stroke,
        });
        self
    }

    /// Append a filled (and optionally stroked) ellipse primitive.
    pub fn draw_ellipse(&mut self, center: Vec2, radii: Vec2) -> &mut Self {
        self.primitives.push(Primitive::Ellipse {
            center,
            radii,
            fill: self.current_fill,
            stroke: self.current_stroke,
        });
        self
    }

    /// Append a line segment of the given width, colored with the current fill.
    /// Strokes do not apply to lines.
    pub fn draw_line(&mut self, from: Vec2, to: Vec2, width: f32) -> &mut Self {
        self.primitives.push(Primitive::Line {
            from,
            to,
            width,
            fill: self.current_fill,
        });
        self
    }

    /// Append a filled (and optionally stroked) annular sector —
    /// a pie slice (when `r_inner = 0`) or donut slice. Angles in
    /// radians, `0 = +x` axis, CCW positive. The angular span is
    /// `end_angle - start_angle`; values are clamped to
    /// `[0, 2π]`.
    pub fn draw_annular_sector(
        &mut self,
        center: Vec2,
        r_inner: f32,
        r_outer: f32,
        start_angle: f32,
        end_angle: f32,
    ) -> &mut Self {
        self.primitives.push(Primitive::AnnularSector {
            center,
            r_inner,
            r_outer,
            start_angle,
            end_angle,
            fill: self.current_fill,
            stroke: self.current_stroke,
        });
        self
    }

    /// Append a stroked arc — circular segment of the given
    /// `radius`, drawn with `stroke_width` band thickness from
    /// `start_angle` to `end_angle`. Internally an annular sector
    /// with `r_inner = radius - stroke_width / 2`,
    /// `r_outer = radius + stroke_width / 2`.
    pub fn draw_arc(
        &mut self,
        center: Vec2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        stroke_width: f32,
    ) -> &mut Self {
        let half = stroke_width * 0.5;
        self.primitives.push(Primitive::AnnularSector {
            center,
            r_inner: (radius - half).max(0.0),
            r_outer: radius + half,
            start_angle,
            end_angle,
            fill: self.current_fill,
            stroke: None,
        });
        self
    }

    /// Append a filled **convex** polygon. Vertices listed in
    /// CCW winding order; the polygon is implicitly closed.
    ///
    /// Non-convex input is undefined behaviour for v1 — fan
    /// triangulation from the first vertex produces visible
    /// overlap when the polygon isn't convex. Strokes do not
    /// apply to polygons in v1; outline a polygon with `draw_line`
    /// segments along the perimeter when an outline is needed.
    pub fn draw_polygon(&mut self, vertices: &[Vec2]) -> &mut Self {
        if vertices.len() < 3 {
            return self;
        }
        self.primitives.push(Primitive::Polygon {
            vertices: vertices.to_vec(),
            fill: self.current_fill,
        });
        self
    }

    /// Number of primitives currently buffered.
    #[must_use]
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_empty() {
        let g = Graphics::new();
        assert_eq!(g.primitive_count(), 0);
    }

    #[test]
    fn draw_rect_appends_one_primitive() {
        let mut g = Graphics::new();
        g.draw_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(g.primitive_count(), 1);
    }

    #[test]
    fn draw_ellipse_appends_one_primitive() {
        let mut g = Graphics::new();
        g.draw_ellipse(Vec2::ZERO, Vec2::splat(5.0));
        assert_eq!(g.primitive_count(), 1);
    }

    #[test]
    fn draw_line_appends_one_primitive() {
        let mut g = Graphics::new();
        g.draw_line(Vec2::ZERO, Vec2::new(10.0, 0.0), 1.0);
        assert_eq!(g.primitive_count(), 1);
    }

    #[test]
    fn fill_persists_until_changed() {
        let mut g = Graphics::new();
        g.fill(Fill::Solid(Color::RED));
        g.draw_rect(Rect::new(0.0, 0.0, 1.0, 1.0));
        g.draw_rect(Rect::new(1.0, 0.0, 1.0, 1.0));
        for p in &g.primitives {
            if let Primitive::RoundedRect { fill, .. } = p {
                assert_eq!(*fill, Fill::Solid(Color::RED));
            }
        }
    }

    #[test]
    fn stroke_persists_until_cleared() {
        let mut g = Graphics::new();
        g.stroke(Some(Stroke::new(2.0, Color::BLACK)));
        g.draw_rect(Rect::new(0.0, 0.0, 1.0, 1.0));
        g.stroke(None);
        g.draw_rect(Rect::new(1.0, 0.0, 1.0, 1.0));

        match &g.primitives[0] {
            Primitive::RoundedRect { stroke, .. } => assert!(stroke.is_some()),
            _ => panic!("expected rounded rect"),
        }
        match &g.primitives[1] {
            Primitive::RoundedRect { stroke, .. } => assert!(stroke.is_none()),
            _ => panic!("expected rounded rect"),
        }
    }
}
