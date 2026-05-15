//! `Animatable` — the trait that says "I know how to interpolate
//! myself from `a` to `b` at parameter `t`".
//!
//! Implemented for scalars, glam vectors, and `wisp::Color`. Default
//! colour-space is `LinearRgb` because wisp composes and clears in
//! linear sRGB; perceptual-blend variants (Oklab / Oklch) ship in
//! [`crate::color_space`] (M-ANIM.13).

use glam::{Vec2, Vec3, Vec4};
use wisp::Color;

/// A value that can be interpolated between two endpoints at a
/// fractional parameter `t` in `[0.0, 1.0]`.
///
/// Implementations are pure, allocation-free, and should produce
/// the start value at `t = 0.0` and the end value at `t = 1.0`.
/// `t` outside `[0, 1]` is *not* clamped — that's the caller's
/// (`Driver` / `Tween`) responsibility — but every implementation
/// must remain finite for any real `t`.
pub trait Animatable: Clone {
    /// Linear interpolation `a + (b - a) * t`.
    #[must_use]
    fn lerp(a: &Self, b: &Self, t: f32) -> Self;
}

// ---------------------------------------------------------------------
// Scalar impls
// ---------------------------------------------------------------------

impl Animatable for f32 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        a + (b - a) * t
    }
}

impl Animatable for f64 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        a + (b - a) * f64::from(t)
    }
}

impl Animatable for i32 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        // Round half-away-from-zero so the endpoints land exactly.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss,
            reason = "i32 range fits f32 precision for the differences typical in UI animations"
        )]
        let result = f32::from(0_i16) + (*a as f32 + (*b - *a) as f32 * t).round();
        #[allow(
            clippy::cast_possible_truncation,
            reason = "result is bounded between a and b which are both i32"
        )]
        let out = result as Self;
        out
    }
}

// ---------------------------------------------------------------------
// Vector impls — component-wise lerp.
// ---------------------------------------------------------------------

impl Animatable for Vec2 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self::lerp(*a, *b, t)
    }
}

impl Animatable for Vec3 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self::lerp(*a, *b, t)
    }
}

impl Animatable for Vec4 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self::lerp(*a, *b, t)
    }
}

// ---------------------------------------------------------------------
// Colour — default `LinearRgb` (component-wise lerp). Perceptual
// blends live on `Tween<Color>` via `color_space::ColorSpace`.
// ---------------------------------------------------------------------

impl Animatable for Color {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self {
            r: f32::lerp(&a.r, &b.r, t),
            g: f32::lerp(&a.g, &b.g, t),
            b: f32::lerp(&a.b, &b.b, t),
            a: f32::lerp(&a.a, &b.a, t),
        }
    }
}

// ---------------------------------------------------------------------
// Tuple impls — let callers tween composite "(position, alpha)"
// pairs without writing a fresh struct.
// ---------------------------------------------------------------------

impl<A: Animatable, B: Animatable> Animatable for (A, B) {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        (A::lerp(&a.0, &b.0, t), B::lerp(&a.1, &b.1, t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_lerp_endpoints() {
        assert!((f32::lerp(&0.0, &10.0, 0.0) - 0.0).abs() < f32::EPSILON);
        assert!((f32::lerp(&0.0, &10.0, 1.0) - 10.0).abs() < f32::EPSILON);
        assert!((f32::lerp(&0.0, &10.0, 0.5) - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn f64_lerp_midpoint() {
        assert!((f64::lerp(&-1.0, &1.0, 0.5) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn i32_lerp_rounds_to_endpoints() {
        assert_eq!(i32::lerp(&0, &100, 0.0), 0);
        assert_eq!(i32::lerp(&0, &100, 1.0), 100);
        assert_eq!(i32::lerp(&0, &100, 0.5), 50);
    }

    #[test]
    fn vec2_lerp_is_componentwise() {
        // Fully-qualified path disambiguates from glam's inherent
        // `Vec2::lerp(self, rhs, t)`.
        let r = <Vec2 as Animatable>::lerp(&Vec2::new(0.0, 0.0), &Vec2::new(10.0, 20.0), 0.5);
        assert!((r.x - 5.0).abs() < f32::EPSILON);
        assert!((r.y - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn vec3_endpoints() {
        let a = Vec3::ZERO;
        let b = Vec3::new(1.0, 2.0, 3.0);
        let start = <Vec3 as Animatable>::lerp(&a, &b, 0.0);
        let end = <Vec3 as Animatable>::lerp(&a, &b, 1.0);
        assert!((start - a).length() < f32::EPSILON);
        assert!((end - b).length() < f32::EPSILON);
    }

    #[test]
    fn color_lerp_endpoints() {
        let a = Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        let b = Color {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        };
        let mid = Color::lerp(&a, &b, 0.5);
        assert!((mid.r - 0.5).abs() < f32::EPSILON);
        assert!((mid.g - 0.5).abs() < f32::EPSILON);
        assert!((mid.b - 0.0).abs() < f32::EPSILON);
        assert!((mid.a - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn tuple_lerp_propagates() {
        let r = <(f32, Vec2)>::lerp(&(0.0, Vec2::ZERO), &(10.0, Vec2::splat(1.0)), 0.25);
        assert!((r.0 - 2.5).abs() < f32::EPSILON);
        assert!((r.1.x - 0.25).abs() < f32::EPSILON);
    }
}
