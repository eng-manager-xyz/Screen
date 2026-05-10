//! Mask shapes used by [`Container::clip`](super::container::Container)
//! to clip a node's rendered subtree to a region.
//!
//! Coordinates are NDC `[-1, +1]²` — the mask is in screen space, not
//! container-local space. The recording-quad use case (cinematic
//! rounded-corner crop on a fixed-position recording surface) is the
//! primary driver. Transform-aware clipping ("clip a moving sprite to
//! its own bounds") is a future enhancement.
//!
//! At render-time, a clipped container's subtree is rendered into a
//! foreground `RenderTexture`, then the [`MaskShape`]'s SDF is sampled
//! per-pixel and multiplied into the alpha channel before the composite
//! is blended back onto the parent. See `render::clip` for the
//! pipeline.

use crate::math::Rect;

/// Shape of a clip / mask region.
///
/// Variants land issue-by-issue:
///
/// - [`MaskShape::Rect`] — AUT-20 rectangle privacy mask.
/// - [`MaskShape::RoundedRect`] — AUT-31 rounded crop foundation.
/// - [`MaskShape::Circle`] — AUT-30 webcam circle mask.
/// - [`MaskShape::Ellipse`] — AUT-34 oval / ellipse mask.
///
/// Later issues (`AUT-35` freehand path) extend this enum further.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum MaskShape {
    /// Sharp-corner rectangle in NDC. The renderer treats this as a
    /// `RoundedRect` with `radius = 0` and routes through the same
    /// SDF pipeline.
    Rect {
        /// Axis-aligned bounding rect, NDC coords.
        rect: Rect,
    },
    /// Rounded rectangle in NDC. `rect` is the axis-aligned bounding
    /// box, `radius` is the corner radius in NDC units (clamped at
    /// render-time to half the smaller side).
    RoundedRect {
        /// Axis-aligned bounding rect, NDC coords.
        rect: Rect,
        /// Corner radius in NDC units.
        radius: f32,
    },
    /// Circle in NDC. The renderer treats this as a `RoundedRect`
    /// with a square bounding box and a corner radius equal to the
    /// half-extent — the rounded-rect SDF degenerates exactly to the
    /// circle SDF in that case (`length(p) - r`).
    Circle {
        /// Center, NDC coords.
        center: glam::Vec2,
        /// Radius, NDC units.
        radius: f32,
    },
    /// Axis-aligned ellipse in NDC. `half_extents` is `(a, b)` of the
    /// implicit equation `(x/a)^2 + (y/b)^2 = 1`. The renderer uses a
    /// scaled-quadratic pseudo-SDF that's anisotropic-correct and
    /// cheap (one length, one multiply); not Euclidean distance, but
    /// accurate enough for masking and AA.
    Ellipse {
        /// Center, NDC coords.
        center: glam::Vec2,
        /// Half-extents `(a, b)`, NDC units.
        half_extents: glam::Vec2,
    },
}

impl MaskShape {
    /// Convenience constructor for a sharp-corner rectangle.
    #[must_use]
    pub fn rect(rect: Rect) -> Self {
        Self::Rect { rect }
    }

    /// Convenience constructor for a rounded rectangle.
    #[must_use]
    pub fn rounded_rect(rect: Rect, radius: f32) -> Self {
        Self::RoundedRect { rect, radius }
    }

    /// Convenience constructor for a circle.
    #[must_use]
    pub fn circle(center: glam::Vec2, radius: f32) -> Self {
        Self::Circle { center, radius }
    }

    /// Convenience constructor for an axis-aligned ellipse.
    #[must_use]
    pub fn ellipse(center: glam::Vec2, half_extents: glam::Vec2) -> Self {
        Self::Ellipse {
            center,
            half_extents,
        }
    }

    /// The axis-aligned bounding rect of the mask.
    #[must_use]
    pub fn bounds(self) -> Rect {
        match self {
            Self::Rect { rect } | Self::RoundedRect { rect, .. } => rect,
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
        }
    }
}
