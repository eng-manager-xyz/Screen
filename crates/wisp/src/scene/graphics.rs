//! `Graphics` — vector primitives (rect, rounded rect; ellipse/line/stroke land in M0.13).
//!
//! Composed over [`Container`]. A `Graphics` holds a list of primitives that
//! are rendered with a shared SDF-based pipeline.

use crate::color::Color;
use crate::math::Rect;
use crate::scene::container::Container;

/// Fill style. `Fill::Gradient` lands in M0.14.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fill {
    /// Single solid color (linear-srgb f32).
    Solid(Color),
}

impl Default for Fill {
    fn default() -> Self {
        Self::Solid(Color::WHITE)
    }
}

/// One drawable primitive within a `Graphics` node.
///
/// Internal — the public API is `Graphics::draw_rect` / `draw_rounded_rect`.
#[derive(Debug, Clone)]
pub(crate) enum Primitive {
    /// Axis-aligned rectangle. `radius == 0.0` = sharp corners.
    RoundedRect { rect: Rect, radius: f32, fill: Fill },
}

/// Vector-primitive node. Holds an ordered list of primitives sharing the
/// node's container transform.
#[derive(Debug, Clone, Default)]
pub struct Graphics {
    pub container: Container,
    current_fill: Fill,
    pub(crate) primitives: Vec<Primitive>,
}

impl Graphics {
    /// Construct an empty `Graphics` with default (white solid) fill.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the fill used by subsequent `draw_*` calls.
    pub fn fill(&mut self, fill: Fill) -> &mut Self {
        self.current_fill = fill;
        self
    }

    /// Append a filled rectangle primitive.
    pub fn draw_rect(&mut self, rect: Rect) -> &mut Self {
        self.primitives.push(Primitive::RoundedRect {
            rect,
            radius: 0.0,
            fill: self.current_fill,
        });
        self
    }

    /// Append a filled rounded rectangle primitive.
    pub fn draw_rounded_rect(&mut self, rect: Rect, radius: f32) -> &mut Self {
        self.primitives.push(Primitive::RoundedRect {
            rect,
            radius,
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
    use glam::Vec2;

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
    fn draw_rounded_rect_appends_one_primitive() {
        let mut g = Graphics::new();
        g.draw_rounded_rect(Rect::new(0.0, 0.0, 10.0, 10.0), 2.0);
        assert_eq!(g.primitive_count(), 1);
    }

    #[test]
    fn fill_persists_until_changed() {
        let mut g = Graphics::new();
        g.fill(Fill::Solid(Color::RED));
        g.draw_rect(Rect::new(0.0, 0.0, 1.0, 1.0));
        g.draw_rect(Rect::new(1.0, 0.0, 1.0, 1.0));
        // Both rects should have RED fill.
        for p in &g.primitives {
            let Primitive::RoundedRect { fill, .. } = p;
            assert_eq!(*fill, Fill::Solid(Color::RED));
        }
    }

    #[test]
    fn rect_uses_zero_radius() {
        let mut g = Graphics::new();
        g.draw_rect(Rect::new(0.0, 0.0, 1.0, 1.0));
        let Primitive::RoundedRect { radius, .. } = &g.primitives[0];
        assert!(radius.abs() < f32::EPSILON);
    }

    #[test]
    fn container_default_is_identity() {
        let g = Graphics::new();
        assert_eq!(g.container.transform, crate::scene::Transform::IDENTITY);
        // Vec2 import for compiler.
        let _ = Vec2::ZERO;
    }
}
