//! Highlight overlay presets (M-VEC.8 / AUT-60).
//!
//! Reusable [`Vector`] constructors for "guide the viewer's eye to
//! this region" overlays. Outputs are plain [`Vector`]s so callers
//! can chain `with_*` builders, drive masks, or feed into vector
//! storyboards.

use crate::color::Color;
use crate::scene::Fill;
use crate::scene::vector::{Vector, VectorShape, VectorStroke};

/// Highlight preset constructors.
///
/// All constructors return [`Vector`]s — pure data — so the caller
/// is free to apply transforms, opacity, or render-time decoration
/// before adding to the stage.
pub struct Highlight;

impl Highlight {
    /// Outline-ring highlight: a stroke-only outline with no fill.
    /// The classic "glowing border" effect for buttons / form fields.
    #[must_use]
    pub fn outline(shape: VectorShape, color: Color, width: f32) -> Vector {
        Vector::new(shape).with_stroke(VectorStroke::new(width, color))
    }

    /// Filled highlight: a translucent fill of the shape so the
    /// underlying content shows through. `alpha` is the multiplied
    /// opacity (clamped to `[0, 1]`); `color`'s own alpha is honored
    /// as well.
    #[must_use]
    pub fn filled(shape: VectorShape, color: Color, alpha: f32) -> Vector {
        let alpha = alpha.clamp(0.0, 1.0);
        Vector::new(shape).with_fill(Fill::Solid(color.with_alpha(color.a * alpha)))
    }

    /// "Pill" highlight — a rounded-rect filled overlay over a
    /// horizontal label. Convenience over `filled` for the common
    /// menu-item / chip use case.
    #[must_use]
    pub fn pill(rect: crate::math::Rect, color: Color, alpha: f32) -> Vector {
        let radius = (rect.size.y * 0.5).abs();
        Self::filled(VectorShape::rounded_rect(rect, radius), color, alpha)
    }

    /// Cheap soft-glow approximation: a stroke that's wider than a
    /// regular outline, with a low-alpha color. Not a true Gaussian
    /// glow — that's deferred to AUT-49 (M-DYN.7 feathered-edge mask)
    /// once feathering ships. Documented as preliminary.
    #[must_use]
    pub fn glow(shape: VectorShape, color: Color, width: f32) -> Vector {
        Vector::new(shape).with_stroke(VectorStroke::new(width, color.with_alpha(color.a * 0.4)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Rect;

    #[test]
    fn outline_has_stroke_no_fill() {
        let v = Highlight::outline(
            VectorShape::rect(Rect::new(-0.5, -0.5, 1.0, 1.0)),
            Color::WHITE,
            0.05,
        );
        assert!(v.fill.is_none());
        let s = v.stroke.expect("outline has stroke");
        assert!((s.width - 0.05).abs() < f32::EPSILON);
    }

    #[test]
    fn filled_alpha_multiplies_color() {
        let base = Color::rgba(0.5, 0.5, 0.5, 1.0);
        let v = Highlight::filled(VectorShape::circle(glam::Vec2::ZERO, 0.4), base, 0.5);
        let Some(Fill::Solid(c)) = v.fill else {
            panic!("expected solid fill");
        };
        assert!((c.a - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn pill_uses_half_height_radius() {
        let v = Highlight::pill(Rect::new(-0.5, -0.05, 1.0, 0.1), Color::WHITE, 0.6);
        match &v.shape {
            VectorShape::RoundedRect { radius, .. } => {
                assert!((radius - 0.05).abs() < f32::EPSILON);
            }
            _ => panic!("pill should be a rounded rect"),
        }
    }

    #[test]
    fn glow_alpha_scales_down_to_indicate_softness() {
        let v = Highlight::glow(
            VectorShape::rect(Rect::new(-0.5, -0.5, 1.0, 1.0)),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
            0.1,
        );
        let s = v.stroke.expect("glow has stroke");
        assert!((s.color.a - 0.4).abs() < 1e-3);
    }
}
