//! `HitShape` — per-node pickable geometry in LOCAL space.
//!
//! Hit-testing is data-driven: the [`PickableMap`](super::PickableMap)
//! stores one `HitShape` per pickable node, and the
//! [`Wisp2dHitTest`](super::Wisp2dHitTest) backend walks the stage,
//! transforms the pointer into each pickable node's local space, and
//! asks the shape whether the point is inside.
//!
//! Variants:
//!
//! - [`HitShape::None`] — node never hits (cheap opt-out).
//! - [`HitShape::Rect`] — axis-aligned local rectangle. Cheapest test.
//! - [`HitShape::Circle`] — `(center, radius)`. Used for round buttons
//!   + the gauge-needle example in WI.10.
//! - [`HitShape::Ellipse`] — `(center, radii)`. Used for `Graphics::ellipse`.
//! - [`HitShape::Polygon`] — even-odd fill-rule path test. Used for the
//!   bucket-fill `MacPaint` chapter, generic polygons, and `Graphics::polygon`.
//!
//! Pixel-alpha hit-testing (`HitShape::PixelAlpha`) is intentionally
//! deferred — it requires a CPU-side alpha probe per node, which is
//! both expensive and only useful for textured sprites with irregular
//! transparency. File a follow-up if a consumer asks.

use glam::Vec2;

use wisp::math::Rect;

/// Pickable geometry in a node's LOCAL coordinate space.
///
/// "Local" means the same coordinate space as the node's
/// `transform.position` origin — i.e., before the node's own
/// `to_mat3()` is applied. The backend transforms the pointer into
/// this space using the inverse of the node's accumulated world
/// matrix and then calls [`HitShape::contains`].
#[derive(Debug, Clone, PartialEq)]
pub enum HitShape {
    /// Never hits. Use to temporarily disable a node without removing
    /// its `Pickable` entry.
    None,
    /// Axis-aligned local rectangle. Half-open `[min, max)` per
    /// `Rect::contains`.
    Rect(Rect),
    /// Circle defined by local center + radius (radius in local units).
    Circle {
        /// Center in local space.
        center: Vec2,
        /// Radius in local units.
        radius: f32,
    },
    /// Axis-aligned ellipse defined by local center + per-axis radii.
    Ellipse {
        /// Center in local space.
        center: Vec2,
        /// Half-extents along x and y (local units).
        radii: Vec2,
    },
    /// Closed polygon. Hit-test uses the even-odd fill rule (matches
    /// SVG `fill-rule: evenodd` and the `MacPaint` bucket-fill chapter).
    Polygon(Vec<Vec2>),
}

impl HitShape {
    /// `true` if `local_point` is inside this shape under the
    /// half-open / even-odd conventions documented above.
    #[must_use]
    pub fn contains(&self, local_point: Vec2) -> bool {
        match self {
            Self::None => false,
            Self::Rect(r) => r.contains(local_point),
            Self::Circle { center, radius } => {
                (local_point - *center).length_squared() <= radius * radius
            }
            Self::Ellipse { center, radii } => {
                let d = local_point - *center;
                if radii.x <= 0.0 || radii.y <= 0.0 {
                    return false;
                }
                let nx = d.x / radii.x;
                let ny = d.y / radii.y;
                nx * nx + ny * ny <= 1.0
            }
            Self::Polygon(vertices) => point_in_polygon_even_odd(local_point, vertices),
        }
    }

    /// Local-space axis-aligned bounding box. The R-tree index uses
    /// this to drop obvious misses before the precise per-shape
    /// `contains` test.
    #[must_use]
    pub fn local_aabb(&self) -> Option<Rect> {
        match self {
            Self::None => None,
            Self::Rect(r) => Some(*r),
            Self::Circle { center, radius } => Some(Rect::new(
                center.x - radius,
                center.y - radius,
                radius * 2.0,
                radius * 2.0,
            )),
            Self::Ellipse { center, radii } => Some(Rect::new(
                center.x - radii.x,
                center.y - radii.y,
                radii.x * 2.0,
                radii.y * 2.0,
            )),
            Self::Polygon(vertices) => {
                if vertices.is_empty() {
                    return None;
                }
                let mut min = vertices[0];
                let mut max = vertices[0];
                for v in &vertices[1..] {
                    min = min.min(*v);
                    max = max.max(*v);
                }
                Some(Rect::new(min.x, min.y, max.x - min.x, max.y - min.y))
            }
        }
    }
}

/// Even-odd-fill polygon containment via ray casting along +x.
///
/// Counts edges crossed by a horizontal ray from `p` to `+∞`; odd
/// count = inside. Matches SVG `evenodd` and the `MacPaint` bucket-fill
/// chapter's narrative. Handles self-intersecting paths correctly.
fn point_in_polygon_even_odd(p: Vec2, vertices: &[Vec2]) -> bool {
    if vertices.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = vertices.len() - 1;
    for i in 0..vertices.len() {
        let vi = vertices[i];
        let vj = vertices[j];
        if (vi.y > p.y) != (vj.y > p.y) {
            let slope = (vj.x - vi.x) / (vj.y - vi.y);
            let x_intersect = vi.x + (p.y - vi.y) * slope;
            if p.x < x_intersect {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains_interior_excludes_far_outside() {
        let s = HitShape::Rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        assert!(s.contains(Vec2::new(5.0, 5.0)));
        assert!(!s.contains(Vec2::new(100.0, 100.0)));
    }

    #[test]
    fn circle_contains_within_radius_squared() {
        let s = HitShape::Circle {
            center: Vec2::ZERO,
            radius: 5.0,
        };
        assert!(s.contains(Vec2::new(3.0, 4.0))); // 3^2 + 4^2 = 25 = r^2
        assert!(!s.contains(Vec2::new(4.0, 4.0))); // 32 > 25
    }

    #[test]
    fn ellipse_contains_within_normalised_unit_disc() {
        let s = HitShape::Ellipse {
            center: Vec2::ZERO,
            radii: Vec2::new(10.0, 2.0),
        };
        assert!(s.contains(Vec2::new(5.0, 1.0)));
        assert!(!s.contains(Vec2::new(5.0, 2.0))); // y at the edge of radii.y but x is at 0.5 → 0.25 + 1.0 > 1
    }

    #[test]
    fn polygon_l_shape_even_odd_excludes_notch() {
        // L-shape: outer square (0,0)-(10,10) with a notch (5,5)-(10,10).
        let l = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 5.0),
            Vec2::new(5.0, 5.0),
            Vec2::new(5.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        let s = HitShape::Polygon(l);
        assert!(s.contains(Vec2::new(2.0, 2.0)), "inside L");
        assert!(!s.contains(Vec2::new(8.0, 8.0)), "in notch — outside L");
    }

    #[test]
    fn none_never_hits() {
        let s = HitShape::None;
        assert!(!s.contains(Vec2::ZERO));
        assert!(s.local_aabb().is_none());
    }

    #[test]
    fn aabb_for_circle_is_bounding_square() {
        let s = HitShape::Circle {
            center: Vec2::new(5.0, 5.0),
            radius: 3.0,
        };
        let bb = s.local_aabb().unwrap();
        assert!((bb.min.x - 2.0).abs() < 1e-6);
        assert!((bb.size.x - 6.0).abs() < 1e-6);
    }
}
