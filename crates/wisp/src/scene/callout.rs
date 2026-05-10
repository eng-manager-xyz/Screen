//! Callout primitives (M-VEC.9 / AUT-61).
//!
//! Reusable [`Vector`] constructors for tutorial-style annotations.
//! Outputs are `Vector`s so they can be added to the stage,
//! transformed, or used as masks. V1 ships the static shapes that
//! don't need stroke-along-path commands:
//!
//! - **Label box** — rounded-rect callout for text annotations.
//! - **Badge** — filled circle for numbered step markers.
//! - **Caption pill** — wide rounded rect for a single-line caption.
//!
//! Arrow / pointer-line callouts need stroke-along-path support
//! (M-VEC.10 / AUT-62). Once that lands, an `arrow_to(from, to)`
//! constructor can join this module without breaking changes.

use glam::Vec2;

use crate::color::Color;
use crate::math::Rect;
use crate::scene::Fill;
use crate::scene::vector::{Vector, VectorShape, VectorStroke};

/// Callout preset constructors.
pub struct Callout;

impl Callout {
    /// Filled rounded-rect label box. Optional stroke gives it an
    /// outlined "card" feel.
    #[must_use]
    pub fn label_box(rect: Rect, fill: Color, stroke: Option<VectorStroke>, radius: f32) -> Vector {
        let mut v =
            Vector::new(VectorShape::rounded_rect(rect, radius)).with_fill(Fill::Solid(fill));
        if let Some(s) = stroke {
            v = v.with_stroke(s);
        }
        v
    }

    /// Filled circle for numbered step markers, badges, dots.
    #[must_use]
    pub fn badge(center: Vec2, radius: f32, fill: Color) -> Vector {
        Vector::new(VectorShape::circle(center, radius)).with_fill(Fill::Solid(fill))
    }

    /// Wide rounded-rect caption pill. Convenience over
    /// `label_box` for the common single-line caption use case (radius
    /// is half the rect's height).
    #[must_use]
    pub fn caption_pill(rect: Rect, fill: Color) -> Vector {
        let radius = (rect.size.y * 0.5).abs();
        Vector::new(VectorShape::rounded_rect(rect, radius)).with_fill(Fill::Solid(fill))
    }

    /// Arrow from `from` to `to` (M-VEC.10 / AUT-62). Returns a
    /// stroked [`Graphics`](crate::scene::Graphics) — arrows are
    /// multi-segment and don't fit the single-`Vector` shape that
    /// other callouts use. Caller adds it to the stage like any
    /// other Graphics node.
    ///
    /// Arrowhead size auto-scales to ~18% of the stem length.
    #[must_use]
    pub fn arrow_to(
        from: glam::Vec2,
        to: glam::Vec2,
        width: f32,
        color: Color,
    ) -> crate::scene::Graphics {
        use crate::scene::path::PathBuilder;
        let stem = to - from;
        let stem_len = stem.length();
        if stem_len < 1e-6 {
            return crate::scene::Graphics::new();
        }
        let head = stem_len * 0.18;
        let dir = stem / stem_len;
        let perp = glam::Vec2::new(-dir.y, dir.x);
        let back = to - dir * head;
        let left = back + perp * head * 0.5;
        let right = back - perp * head * 0.5;
        PathBuilder::new()
            .move_to(from)
            .line_to(to)
            .move_to(left)
            .line_to(to)
            .line_to(right)
            .build()
            .stroke_to_graphics(width, color, 0.005)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_box_with_stroke_has_both() {
        let v = Callout::label_box(
            Rect::new(-0.5, -0.2, 1.0, 0.4),
            Color::rgba_u8(220, 220, 130, 230),
            Some(VectorStroke::new(0.012, Color::WHITE)),
            0.04,
        );
        assert!(v.fill.is_some());
        assert!(v.stroke.is_some());
    }

    #[test]
    fn label_box_without_stroke_omits_it() {
        let v = Callout::label_box(
            Rect::new(-0.5, -0.2, 1.0, 0.4),
            Color::rgba_u8(220, 220, 130, 230),
            None,
            0.04,
        );
        assert!(v.fill.is_some());
        assert!(v.stroke.is_none());
    }

    #[test]
    fn badge_is_circle_at_center() {
        let v = Callout::badge(Vec2::new(0.4, 0.5), 0.06, Color::rgba(1.0, 0.4, 0.0, 1.0));
        match &v.shape {
            VectorShape::Circle { center, radius } => {
                assert!((center.x - 0.4).abs() < f32::EPSILON);
                assert!((radius - 0.06).abs() < f32::EPSILON);
            }
            _ => panic!("badge should be a circle"),
        }
    }

    #[test]
    fn caption_pill_radius_is_half_height() {
        let v = Callout::caption_pill(
            Rect::new(-0.6, -0.04, 1.2, 0.08),
            Color::rgba_u8(255, 255, 255, 220),
        );
        match &v.shape {
            VectorShape::RoundedRect { radius, .. } => {
                assert!((radius - 0.04).abs() < f32::EPSILON);
            }
            _ => panic!("caption_pill should be a rounded rect"),
        }
    }
}
