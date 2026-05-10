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
/// AUT-31 ships only [`MaskShape::RoundedRect`]. Later issues
/// (`AUT-30` circle, `AUT-34` ellipse, `AUT-35` freehand path) extend
/// this enum.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum MaskShape {
    /// Rounded rectangle in NDC. `rect` is the axis-aligned bounding
    /// box, `radius` is the corner radius in NDC units (clamped at
    /// render-time to half the smaller side).
    RoundedRect {
        /// Axis-aligned bounding rect, NDC coords.
        rect: Rect,
        /// Corner radius in NDC units.
        radius: f32,
    },
}

impl MaskShape {
    /// Convenience constructor for a rounded rectangle.
    #[must_use]
    pub fn rounded_rect(rect: Rect, radius: f32) -> Self {
        Self::RoundedRect { rect, radius }
    }

    /// The axis-aligned bounding rect of the mask.
    #[must_use]
    pub fn bounds(self) -> Rect {
        match self {
            Self::RoundedRect { rect, .. } => rect,
        }
    }
}
