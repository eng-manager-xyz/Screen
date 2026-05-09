//! Axis-aligned rectangle.

use glam::Vec2;

/// Axis-aligned rectangle defined by minimum corner and size.
///
/// Coordinate system: top-left origin, `+Y` down (Pixi / screen convention).
/// [`Rect::contains`] is half-open: `[min, max)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Top-left corner.
    pub min: Vec2,
    /// `(width, height)`. Components should be non-negative.
    pub size: Vec2,
}

impl Rect {
    /// Construct from `(x, y, width, height)`.
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            min: Vec2::new(x, y),
            size: Vec2::new(width, height),
        }
    }

    /// Construct from `min` and `max` corners. Size = `max - min`.
    #[must_use]
    pub fn from_min_max(min: Vec2, max: Vec2) -> Self {
        Self {
            min,
            size: max - min,
        }
    }

    /// Maximum corner: `min + size`.
    #[must_use]
    pub fn max(&self) -> Vec2 {
        self.min + self.size
    }

    /// Geometric center.
    #[must_use]
    pub fn center(&self) -> Vec2 {
        self.min + self.size * 0.5
    }

    /// Width.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.size.x
    }

    /// Height.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.size.y
    }

    /// Whether the rect contains a point (half-open: `[min, max)`).
    #[must_use]
    pub fn contains(&self, p: Vec2) -> bool {
        let max = self.max();
        p.x >= self.min.x && p.y >= self.min.y && p.x < max.x && p.y < max.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_constructs_min_and_size() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(r.min, Vec2::new(10.0, 20.0));
        assert_eq!(r.size, Vec2::new(30.0, 40.0));
    }

    #[test]
    fn max_is_min_plus_size() {
        let r = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(r.max(), Vec2::new(100.0, 50.0));
    }

    #[test]
    fn from_min_max_round_trips() {
        let min = Vec2::new(5.0, 10.0);
        let max = Vec2::new(25.0, 40.0);
        let r = Rect::from_min_max(min, max);
        assert_eq!(r.min, min);
        assert_eq!(r.max(), max);
    }

    #[test]
    fn center_is_midpoint() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert_eq!(r.center(), Vec2::new(50.0, 50.0));
    }

    #[test]
    fn contains_inside_point() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains(Vec2::new(50.0, 50.0)));
    }

    #[test]
    fn contains_includes_min_edge() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains(Vec2::new(0.0, 0.0)));
    }

    #[test]
    fn contains_excludes_max_edge() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(!r.contains(Vec2::new(100.0, 50.0)));
        assert!(!r.contains(Vec2::new(50.0, 100.0)));
    }

    #[test]
    fn width_and_height_match_size() {
        let r = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert!((r.width() - 100.0).abs() < f32::EPSILON);
        assert!((r.height() - 50.0).abs() < f32::EPSILON);
    }
}
