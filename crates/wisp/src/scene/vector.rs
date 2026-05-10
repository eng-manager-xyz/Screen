//! Vector shape primitive model — Screen's shared shape language
//! (M-VEC.1 / AUT-53).
//!
//! This is *not* SVG support. It is Wisp's own deterministic vector
//! primitive model: rect / rounded-rect / circle / ellipse / path /
//! group, with fill / stroke / opacity / transform.
//!
//! One shape language drives every visual tool in Screen — masks,
//! crops, highlights, callouts, cursor effects — so each tool
//! doesn't invent its own geometry model. Subsequent issues build
//! on this:
//!
//! - **M-VEC.2 / AUT-54** renders [`Vector`] into the scene as
//!   visible geometry.
//! - **M-VEC.3 / AUT-55** renders [`Vector`] into alpha-mask textures
//!   (bridge to M-DYN.1).
//! - **M-VEC.4..6** refactors privacy blur / redaction / spotlight
//!   onto the vector mask path.
//!
//! ## Relationship to existing types
//!
//! - [`MaskShape`] (the analytic SDF subset) can be derived from a
//!   compatible [`VectorShape`] via [`VectorShape::as_mask_shape`].
//!   This lets the existing `apply_clip` / `apply_solid_redaction` /
//!   etc. consume vector data without a separate code path.
//! - [`Path` points](VectorShape::Path) carry an owned `Vec<Vec2>`
//!   so `VectorShape` is `Clone`, not `Copy` (intentional — the path
//!   variant is the reason `MaskShape::Path` was never added; see
//!   M-MASK.10's chapter).

use glam::Vec2;

use crate::color::Color;
use crate::math::Rect;
use crate::scene::clip::MaskShape;
use crate::scene::graphics::Fill;
use crate::scene::transform::Transform;

/// Geometric shape — no paint, no transform. The "what" of a vector
/// primitive.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum VectorShape {
    /// Sharp-corner rectangle in NDC.
    Rect {
        /// Axis-aligned bounding rect.
        rect: Rect,
    },
    /// Rounded rectangle in NDC.
    RoundedRect {
        /// Axis-aligned bounding rect.
        rect: Rect,
        /// Corner radius in NDC units.
        radius: f32,
    },
    /// Circle in NDC.
    Circle {
        /// Center, NDC.
        center: Vec2,
        /// Radius, NDC.
        radius: f32,
    },
    /// Anisotropic ellipse in NDC.
    Ellipse {
        /// Center, NDC.
        center: Vec2,
        /// Half-extents `(a, b)` of the implicit equation
        /// `(x/a)² + (y/b)² = 1`.
        half_extents: Vec2,
    },
    /// Closed polygon in NDC. Up to 32 vertices honored by the mask
    /// path (matching `path_clip.wgsl`'s uniform cap).
    Path {
        /// Vertex list. `Vec` (not slice) so the variant is owned;
        /// pay the allocation once per shape construction.
        points: Vec<Vec2>,
    },
}

impl VectorShape {
    /// Convenience constructor.
    #[must_use]
    pub fn rect(rect: Rect) -> Self {
        Self::Rect { rect }
    }

    /// Convenience constructor.
    #[must_use]
    pub fn rounded_rect(rect: Rect, radius: f32) -> Self {
        Self::RoundedRect { rect, radius }
    }

    /// Convenience constructor.
    #[must_use]
    pub fn circle(center: Vec2, radius: f32) -> Self {
        Self::Circle { center, radius }
    }

    /// Convenience constructor.
    #[must_use]
    pub fn ellipse(center: Vec2, half_extents: Vec2) -> Self {
        Self::Ellipse {
            center,
            half_extents,
        }
    }

    /// Convenience constructor.
    #[must_use]
    pub fn path(points: Vec<Vec2>) -> Self {
        Self::Path { points }
    }

    /// Axis-aligned bounding rect of the shape. For paths, computed
    /// from the point list (returns a zero rect for an empty path).
    #[must_use]
    pub fn bounds(&self) -> Rect {
        match self {
            Self::Rect { rect } | Self::RoundedRect { rect, .. } => *rect,
            Self::Circle { center, radius } => Rect::new(
                center.x - radius,
                center.y - radius,
                radius * 2.0,
                radius * 2.0,
            ),
            Self::Ellipse {
                center,
                half_extents,
            } => Rect::new(
                center.x - half_extents.x,
                center.y - half_extents.y,
                half_extents.x * 2.0,
                half_extents.y * 2.0,
            ),
            Self::Path { points } => points_bounds(points),
        }
    }

    /// If this shape has an analytic SDF (rect / rounded / circle /
    /// ellipse), return it as a [`MaskShape`] so the existing mask /
    /// clip / privacy-blur / redaction / spotlight machinery can
    /// consume it directly. Returns `None` for [`Self::Path`] —
    /// callers should use the path-clip / path-mask-texture variants
    /// instead.
    #[must_use]
    pub fn as_mask_shape(&self) -> Option<MaskShape> {
        match self {
            Self::Rect { rect } => Some(MaskShape::Rect { rect: *rect }),
            Self::RoundedRect { rect, radius } => Some(MaskShape::RoundedRect {
                rect: *rect,
                radius: *radius,
            }),
            Self::Circle { center, radius } => Some(MaskShape::Circle {
                center: *center,
                radius: *radius,
            }),
            Self::Ellipse {
                center,
                half_extents,
            } => Some(MaskShape::Ellipse {
                center: *center,
                half_extents: *half_extents,
            }),
            Self::Path { .. } => None,
        }
    }

    /// If this is a path, return the points slice. Returns `None`
    /// for analytic SDF shapes.
    #[must_use]
    pub fn as_path_points(&self) -> Option<&[Vec2]> {
        match self {
            Self::Path { points } => Some(points),
            _ => None,
        }
    }
}

fn points_bounds(points: &[Vec2]) -> Rect {
    if points.is_empty() {
        return Rect::new(0.0, 0.0, 0.0, 0.0);
    }
    let mut min = points[0];
    let mut max = points[0];
    for p in &points[1..] {
        min = min.min(*p);
        max = max.max(*p);
    }
    Rect::new(min.x, min.y, max.x - min.x, max.y - min.y)
}

/// Stroke paint — width (NDC units) plus solid color.
///
/// Gradient strokes are deferred (M-VEC.15 / P2 polish).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VectorStroke {
    /// Stroke width, NDC units.
    pub width: f32,
    /// Stroke color (RGBA in linear f32).
    pub color: Color,
}

impl VectorStroke {
    /// Convenience constructor.
    #[must_use]
    pub fn new(width: f32, color: Color) -> Self {
        Self { width, color }
    }
}

/// Full vector primitive — shape + fill + stroke + opacity +
/// transform. The "what" + the "how it paints."
///
/// `fill` and `stroke` are optional so a `Vector` can be a fill-only
/// silhouette, a stroke-only outline, or both. For mask use cases
/// neither is consulted — only [`Vector::shape`] matters.
#[derive(Debug, Clone, PartialEq)]
pub struct Vector {
    /// Geometry.
    pub shape: VectorShape,
    /// Optional fill paint. `None` means no fill (stroke-only).
    pub fill: Option<Fill>,
    /// Optional stroke paint. `None` means no stroke (fill-only).
    pub stroke: Option<VectorStroke>,
    /// Multiplicative opacity in `[0, 1]`. Combines with any alpha
    /// already present in `fill` / `stroke`.
    pub opacity: f32,
    /// Local transform applied to the shape before rasterization.
    pub transform: Transform,
}

impl Vector {
    /// Construct a `Vector` from a shape; no fill, no stroke, full
    /// opacity, identity transform. Builder methods set the rest.
    #[must_use]
    pub fn new(shape: VectorShape) -> Self {
        Self {
            shape,
            fill: None,
            stroke: None,
            opacity: 1.0,
            transform: Transform::default(),
        }
    }

    /// Builder: set the fill paint.
    #[must_use]
    pub fn with_fill(mut self, fill: Fill) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Builder: set the stroke paint.
    #[must_use]
    pub fn with_stroke(mut self, stroke: VectorStroke) -> Self {
        self.stroke = Some(stroke);
        self
    }

    /// Builder: set the opacity. Clamped to `[0, 1]` at apply-time
    /// (the renderer uses `f32::clamp` when reading).
    #[must_use]
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// Builder: set the local transform.
    #[must_use]
    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_bounds_match_rect() {
        let r = Rect::new(-0.5, -0.3, 1.0, 0.6);
        let bounds = VectorShape::rect(r).bounds();
        assert!((bounds.min.x - r.min.x).abs() < 1e-6);
        assert!((bounds.size.x - r.size.x).abs() < 1e-6);
    }

    #[test]
    fn circle_bounds_are_2r_square() {
        let bounds = VectorShape::circle(Vec2::new(0.1, -0.2), 0.3).bounds();
        assert!((bounds.size.x - 0.6).abs() < 1e-6);
        assert!((bounds.size.y - 0.6).abs() < 1e-6);
        assert!((bounds.min.x - (-0.2)).abs() < 1e-6);
        assert!((bounds.min.y - (-0.5)).abs() < 1e-6);
    }

    #[test]
    fn ellipse_bounds_are_anisotropic() {
        let bounds = VectorShape::ellipse(Vec2::ZERO, Vec2::new(0.7, 0.3)).bounds();
        assert!((bounds.size.x - 1.4).abs() < 1e-6);
        assert!((bounds.size.y - 0.6).abs() < 1e-6);
    }

    #[test]
    fn path_bounds_min_max_extent() {
        let pts = vec![
            Vec2::new(-0.4, 0.1),
            Vec2::new(0.2, -0.3),
            Vec2::new(0.5, 0.6),
        ];
        let bounds = VectorShape::path(pts).bounds();
        assert!((bounds.min.x - (-0.4)).abs() < 1e-6);
        assert!((bounds.min.y - (-0.3)).abs() < 1e-6);
        assert!((bounds.size.x - 0.9).abs() < 1e-6);
        assert!((bounds.size.y - 0.9).abs() < 1e-6);
    }

    #[test]
    fn empty_path_bounds_is_zero() {
        let bounds = VectorShape::path(Vec::new()).bounds();
        assert!(bounds.size.x.abs() < 1e-6);
        assert!(bounds.size.y.abs() < 1e-6);
    }

    #[test]
    fn analytic_shapes_round_trip_to_mask_shape() {
        let r = Rect::new(-0.5, -0.5, 1.0, 1.0);
        assert!(matches!(
            VectorShape::rect(r).as_mask_shape(),
            Some(MaskShape::Rect { .. })
        ));
        assert!(matches!(
            VectorShape::rounded_rect(r, 0.2).as_mask_shape(),
            Some(MaskShape::RoundedRect { .. })
        ));
        assert!(matches!(
            VectorShape::circle(Vec2::ZERO, 0.4).as_mask_shape(),
            Some(MaskShape::Circle { .. })
        ));
        assert!(matches!(
            VectorShape::ellipse(Vec2::ZERO, Vec2::new(0.7, 0.3)).as_mask_shape(),
            Some(MaskShape::Ellipse { .. })
        ));
    }

    #[test]
    fn path_does_not_round_trip_to_mask_shape() {
        let path = VectorShape::path(vec![Vec2::ZERO, Vec2::new(0.5, 0.5)]);
        assert!(path.as_mask_shape().is_none());
        assert_eq!(path.as_path_points().map(<[_]>::len), Some(2));
    }

    #[test]
    fn vector_default_opacity_is_one() {
        let v = Vector::new(VectorShape::rect(Rect::new(-0.5, -0.5, 1.0, 1.0)));
        assert!((v.opacity - 1.0).abs() < f32::EPSILON);
        assert!(v.fill.is_none());
        assert!(v.stroke.is_none());
    }

    #[test]
    fn vector_builder_chains() {
        let v = Vector::new(VectorShape::circle(Vec2::ZERO, 0.5))
            .with_fill(Fill::Solid(Color::rgba(1.0, 0.5, 0.0, 1.0)))
            .with_stroke(VectorStroke::new(0.02, Color::WHITE))
            .with_opacity(0.8);
        assert!(v.fill.is_some());
        assert!(v.stroke.is_some());
        assert!((v.opacity - 0.8).abs() < f32::EPSILON);
    }
}
