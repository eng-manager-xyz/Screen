//! `Transform` — local 2D affine transform built from translate / scale / rotate / pivot / skew.
//!
//! Composition order matches Pixi:
//! `M = T(position) · S(scale) · R(rotation) · K(skew) · T(-pivot)`
//!
//! That is: move the pivot to the origin, skew, rotate, scale, then move the
//! pivot to its target `position` in parent space.

use glam::{Mat3, Vec2, Vec3};

/// Local 2D affine transform.
///
/// Defaults to identity: zero translation, unit scale, zero rotation/skew/pivot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Translation in parent coordinates. The pivot lands here.
    pub position: Vec2,
    /// Per-axis scale around the pivot.
    pub scale: Vec2,
    /// Rotation in radians (around the pivot).
    pub rotation: f32,
    /// Anchor point in local coordinates that stays fixed under rotation/scale.
    pub pivot: Vec2,
    /// Per-axis skew in radians.
    pub skew: Vec2,
}

impl Transform {
    /// Identity transform — `to_mat3` returns [`Mat3::IDENTITY`].
    pub const IDENTITY: Self = Self {
        position: Vec2::ZERO,
        scale: Vec2::ONE,
        rotation: 0.0,
        pivot: Vec2::ZERO,
        skew: Vec2::ZERO,
    };

    /// Construct a translation-only transform.
    #[must_use]
    pub const fn from_position(position: Vec2) -> Self {
        Self {
            position,
            ..Self::IDENTITY
        }
    }

    /// Construct a scale-only transform.
    #[must_use]
    pub const fn from_scale(scale: Vec2) -> Self {
        Self {
            scale,
            ..Self::IDENTITY
        }
    }

    /// Construct a rotation-only transform (radians).
    #[must_use]
    pub const fn from_rotation(rotation: f32) -> Self {
        Self {
            rotation,
            ..Self::IDENTITY
        }
    }

    /// Compose this transform into a 3×3 affine matrix.
    ///
    /// Order: `T(position) · S(scale) · R(rotation) · K(skew) · T(-pivot)`.
    #[must_use]
    pub fn to_mat3(&self) -> Mat3 {
        let translate = Mat3::from_translation(self.position);
        let scale = Mat3::from_scale(self.scale);
        let rotation = Mat3::from_angle(self.rotation);
        let skew = Mat3::from_cols(
            Vec3::new(1.0, self.skew.y.tan(), 0.0),
            Vec3::new(self.skew.x.tan(), 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        );
        let pivot_inverse = Mat3::from_translation(-self.pivot);

        translate * scale * rotation * skew * pivot_inverse
    }

    /// Transform a 2D point through this transform.
    #[must_use]
    pub fn transform_point(&self, p: Vec2) -> Vec2 {
        self.to_mat3().transform_point2(p)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Compose a parent world transform with a local transform → child world transform.
///
/// `world_child = world_parent · local_child` — apply `local` first, then `parent`.
#[must_use]
pub fn compose(world_parent: Mat3, local: &Transform) -> Mat3 {
    world_parent * local.to_mat3()
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI, TAU};

    use proptest::prelude::*;

    use super::*;

    fn approx_vec2(a: Vec2, b: Vec2, tol: f32) -> bool {
        (a - b).length_squared() < tol * tol
    }

    fn approx_mat3(a: Mat3, b: Mat3, tol: f32) -> bool {
        a.x_axis.distance_squared(b.x_axis) < tol * tol
            && a.y_axis.distance_squared(b.y_axis) < tol * tol
            && a.z_axis.distance_squared(b.z_axis) < tol * tol
    }

    #[test]
    fn identity_to_mat3_is_mat3_identity() {
        assert!(approx_mat3(
            Transform::IDENTITY.to_mat3(),
            Mat3::IDENTITY,
            1e-6
        ));
    }

    #[test]
    fn default_is_identity() {
        assert_eq!(Transform::default(), Transform::IDENTITY);
    }

    #[test]
    fn translation_only_moves_origin() {
        let t = Transform::from_position(Vec2::new(10.0, 20.0));
        assert!(approx_vec2(
            t.transform_point(Vec2::ZERO),
            Vec2::new(10.0, 20.0),
            1e-6
        ));
    }

    #[test]
    fn scale_only_scales_around_origin() {
        let t = Transform::from_scale(Vec2::new(2.0, 3.0));
        assert!(approx_vec2(
            t.transform_point(Vec2::new(4.0, 5.0)),
            Vec2::new(8.0, 15.0),
            1e-6
        ));
    }

    #[test]
    fn rotation_quarter_turn_swaps_axes() {
        let t = Transform::from_rotation(FRAC_PI_2);
        assert!(approx_vec2(t.transform_point(Vec2::X), Vec2::Y, 1e-5));
        assert!(approx_vec2(t.transform_point(Vec2::Y), -Vec2::X, 1e-5));
    }

    #[test]
    fn rotation_full_turn_returns_to_origin_point() {
        let t = Transform::from_rotation(TAU);
        let p = Vec2::new(3.0, 7.0);
        assert!(approx_vec2(t.transform_point(p), p, 1e-4));
    }

    #[test]
    fn pivot_keeps_anchor_at_position_under_rotation() {
        // Pivot at (5,5) in local, position at (10,10) in parent, rotated 180°.
        // The pivot itself maps to position regardless of rotation.
        let t = Transform {
            position: Vec2::new(10.0, 10.0),
            pivot: Vec2::new(5.0, 5.0),
            rotation: PI,
            ..Transform::IDENTITY
        };
        assert!(approx_vec2(
            t.transform_point(Vec2::new(5.0, 5.0)),
            Vec2::new(10.0, 10.0),
            1e-5
        ));
    }

    #[test]
    fn compose_identity_parent_yields_local() {
        let local = Transform::from_position(Vec2::new(3.0, 4.0));
        let world = compose(Mat3::IDENTITY, &local);
        assert!(approx_mat3(world, local.to_mat3(), 1e-6));
    }

    #[test]
    fn compose_translations_add() {
        let parent = Transform::from_position(Vec2::new(10.0, 20.0));
        let child = Transform::from_position(Vec2::new(3.0, 4.0));
        let world = compose(parent.to_mat3(), &child);
        // Child's local origin in world space = parent.position + child.position.
        let p = world.transform_point2(Vec2::ZERO);
        assert!(approx_vec2(p, Vec2::new(13.0, 24.0), 1e-6));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Pure translation: `transform_point(p) = p + position`.
        #[test]
        fn prop_translation_adds(
            tx in -1000.0_f32..1000.0,
            ty in -1000.0_f32..1000.0,
            px in -1000.0_f32..1000.0,
            py in -1000.0_f32..1000.0,
        ) {
            let t = Transform::from_position(Vec2::new(tx, ty));
            let p = Vec2::new(px, py);
            let expected = p + Vec2::new(tx, ty);
            prop_assert!(approx_vec2(t.transform_point(p), expected, 1e-3));
        }

        /// Pure scale: `transform_point(p) = p * scale` (around origin).
        #[test]
        fn prop_scale_multiplies(
            sx in 0.01_f32..100.0,
            sy in 0.01_f32..100.0,
            px in -1000.0_f32..1000.0,
            py in -1000.0_f32..1000.0,
        ) {
            let t = Transform::from_scale(Vec2::new(sx, sy));
            let p = Vec2::new(px, py);
            let expected = Vec2::new(px * sx, py * sy);
            prop_assert!(approx_vec2(t.transform_point(p), expected, 1e-2));
        }

        /// `compose(parent, local)` applied to a point equals
        /// `parent · local · p`.
        #[test]
        fn prop_compose_associates_with_application(
            parent_tx in -100.0_f32..100.0,
            parent_ty in -100.0_f32..100.0,
            child_tx in -100.0_f32..100.0,
            child_ty in -100.0_f32..100.0,
            px in -100.0_f32..100.0,
            py in -100.0_f32..100.0,
        ) {
            let parent = Transform::from_position(Vec2::new(parent_tx, parent_ty));
            let child = Transform::from_position(Vec2::new(child_tx, child_ty));
            let world = compose(parent.to_mat3(), &child);
            let p = Vec2::new(px, py);
            // World application = parent applied to (child applied to p).
            let direct = parent.transform_point(child.transform_point(p));
            let composed = world.transform_point2(p);
            prop_assert!(approx_vec2(composed, direct, 1e-2));
        }

        /// Rotating by θ then by -θ returns to the starting point.
        #[test]
        fn prop_rotation_inverse_round_trips(
            theta in -10.0_f32..10.0,
            px in -100.0_f32..100.0,
            py in -100.0_f32..100.0,
        ) {
            let forward = Transform::from_rotation(theta);
            let backward = Transform::from_rotation(-theta);
            let p = Vec2::new(px, py);
            let round_trip = backward.transform_point(forward.transform_point(p));
            prop_assert!(approx_vec2(round_trip, p, 1e-3));
        }
    }
}
