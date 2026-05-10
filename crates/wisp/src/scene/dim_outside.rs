//! Renderer-side data for the dim-outside composition primitive
//! (M-MASK / AUT-29). Companion to [`PrivacyBlur`](super::PrivacyBlur)
//! and the lower-level
//! [`Renderer::apply_spotlight`](crate::render::Renderer::apply_spotlight).
//!
//! [`DimOutside`] bundles a [`MaskShape`] (where the focus is) with a
//! [`DimStrength`] (how dark the surrounding context becomes). The
//! editor inspector will eventually persist + animate this struct;
//! the renderer consumes one via
//! [`Renderer::apply_dim_outside_data`](crate::render::Renderer::apply_dim_outside_data),
//! a one-line wrapper that calls `apply_spotlight` with a black
//! overlay at the right alpha.

use crate::scene::clip::MaskShape;

/// Strength of the dim applied outside the focus shape.
///
/// Like [`BlurStrength`](super::BlurStrength), the variants compile
/// down to numbers — but the symbolic enum is what the editor
/// persists, so retuning a preset later doesn't break project files.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub enum DimStrength {
    /// Light context dim — surrounding area still legible. Useful for
    /// "this is the active panel" gentle hints.
    Light,
    /// Balanced dim — surrounding area visibly faded but still
    /// recognizable. The default for "follow the walkthrough."
    #[default]
    Medium,
    /// Heavy dim — surrounding area nearly black. The cinematic
    /// "spotlight only" treatment.
    Heavy,
    /// Application-specified alpha. Clamped to `[0.0, 1.0]` at render
    /// time.
    Custom(f32),
}

impl DimStrength {
    /// Alpha (`0..=1`) of the black overlay applied outside the
    /// focus shape.
    #[must_use]
    pub fn alpha(self) -> f32 {
        match self {
            Self::Light => 0.4,
            Self::Medium => 0.7,
            Self::Heavy => 0.9,
            Self::Custom(a) => a.clamp(0.0, 1.0),
        }
    }
}

/// Renderer data for a dim-outside composition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DimOutside {
    /// The focus region — pixels inside stay clear, pixels outside
    /// get the overlay applied.
    pub shape: MaskShape,
    /// How strong the surrounding dim is.
    pub strength: DimStrength,
}

impl DimOutside {
    /// Convenience constructor: rectangle focus region, default
    /// (Medium) dim.
    #[must_use]
    pub fn rect(rect: crate::math::Rect) -> Self {
        Self {
            shape: MaskShape::rect(rect),
            strength: DimStrength::default(),
        }
    }

    /// Convenience constructor: rounded-rect focus region, default
    /// (Medium) dim.
    #[must_use]
    pub fn rounded_rect(rect: crate::math::Rect, radius: f32) -> Self {
        Self {
            shape: MaskShape::rounded_rect(rect, radius),
            strength: DimStrength::default(),
        }
    }

    /// Builder-style override of the strength.
    #[must_use]
    pub fn with_strength(mut self, strength: DimStrength) -> Self {
        self.strength = strength;
        self
    }
}
