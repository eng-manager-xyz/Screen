//! Renderer-side data for the privacy-blur composition primitive.
//!
//! [`PrivacyBlur`] bundles a [`MaskShape`] (where) with a
//! [`BlurStrength`] (how strong). A `PrivacyBlur` value is what the
//! editor inspector will eventually persist + animate; the renderer
//! consumes one via
//! [`Renderer::apply_privacy_blur_data`](crate::render::Renderer::apply_privacy_blur_data),
//! a one-liner wrapper over the lower-level
//! [`Renderer::apply_privacy_blur`](crate::render::Renderer::apply_privacy_blur).
//!
//! The strength enum has named presets (Soft/Medium/Strong) plus a
//! `Custom(f32)` escape hatch. Presets exist because the editor's
//! "blur strength" slider should snap to a small set of cinematic
//! values rather than expose raw pixel radii to the user — but the
//! renderer-data layer keeps both the symbolic name and the numeric
//! radius reachable so AUT-22's "story variants demonstrate different
//! blur strengths" stays deterministic.

use crate::scene::clip::MaskShape;

/// Strength of a privacy blur, in symbolic + numeric form.
///
/// The variants compile down to the same call into
/// [`BlurFilter::new`](crate::filter::BlurFilter::new) — the enum exists so
/// the *editor* can persist a stable name even when we later retune
/// the underlying pixel radii.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub enum BlurStrength {
    /// Cinematic soft blur — readable shapes still hint through.
    /// Useful for visual polish where strict redaction isn't needed.
    Soft,
    /// Balanced redaction — text becomes unreadable, broad shapes
    /// remain visible. The default for "blur this private region."
    #[default]
    Medium,
    /// Heavy redaction — a strong mosaic-feel blur that wipes nearly
    /// all detail. Use when publishing to an audience that may zoom in.
    Strong,
    /// Application-specified pixel radius. Clamped to a sane upper
    /// bound at render time to keep the offscreen RT cost finite.
    Custom(f32),
}

impl BlurStrength {
    /// Pixel radius this strength corresponds to.
    ///
    /// The Soft/Medium/Strong values are the "stable points" — if we
    /// retune them later we keep the editor's persisted enum stable
    /// and only the numbers move.
    #[must_use]
    pub fn radius_px(self) -> f32 {
        match self {
            Self::Soft => 6.0,
            Self::Medium => 12.0,
            Self::Strong => 24.0,
            Self::Custom(r) => r.clamp(0.0, 64.0),
        }
    }
}

/// Renderer data for a privacy-blur composition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrivacyBlur {
    /// Where the blur applies. NDC `[-1, +1]²`.
    pub shape: MaskShape,
    /// How strong the blur is.
    pub strength: BlurStrength,
}

impl PrivacyBlur {
    /// Convenience constructor for the most common case (rectangle +
    /// medium strength).
    #[must_use]
    pub fn rect(rect: crate::math::Rect) -> Self {
        Self {
            shape: MaskShape::rect(rect),
            strength: BlurStrength::default(),
        }
    }

    /// Convenience constructor for a rounded-rect blur with default
    /// (Medium) strength.
    #[must_use]
    pub fn rounded_rect(rect: crate::math::Rect, radius: f32) -> Self {
        Self {
            shape: MaskShape::rounded_rect(rect, radius),
            strength: BlurStrength::default(),
        }
    }

    /// Builder-style override of the strength.
    #[must_use]
    pub fn with_strength(mut self, strength: BlurStrength) -> Self {
        self.strength = strength;
        self
    }
}
