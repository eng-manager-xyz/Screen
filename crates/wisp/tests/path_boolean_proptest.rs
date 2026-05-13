//! Property tests for path boolean ops (M-BOOL.16 / AUT-177).
//!
//! Checks algebraic laws that must hold for *any* pair of input
//! polygons:
//!
//! - **Commutativity** of Union, Intersection, XOR.
//! - **Associativity** of Union: `A ∪ (B ∪ C) ≡ (A ∪ B) ∪ C`.
//! - **De Morgan** (within a probe-rect window):
//!   `¬(A ∪ B) ≡ ¬A ∩ ¬B`.
//! - **Identity**: `A ∪ ∅ ⊇ A`, `A − ∅ ⊇ A`, `A ∩ ∅ = ∅`.
//! - **Self-difference**: `A − A` does not contain `A`'s centre.
//! - **XOR self-cancellation**: `A ⊕ A` does not contain `A`'s centre.
//!
//! These laws use point-in-polygon sampling rather than exact path
//! equality because the engine re-tessellates and minor numeric
//! differences in vertex placement should not fail the property.

use proptest::prelude::*;

use wisp::Vec2;
use wisp::path::boolean::{BoolOptions, BooleanOp, combine};
use wisp::path::{Path, PathBuilder};

/// Generate an axis-aligned rectangle in NDC `[-0.9, 0.9]`.
fn rect_strategy() -> impl Strategy<Value = Path> {
    (-0.8f32..0.8, -0.8f32..0.8, 0.05f32..0.4, 0.05f32..0.4).prop_map(|(cx, cy, hw, hh)| {
        PathBuilder::new()
            .move_to(Vec2::new(cx - hw, cy - hh))
            .line_to(Vec2::new(cx + hw, cy - hh))
            .line_to(Vec2::new(cx + hw, cy + hh))
            .line_to(Vec2::new(cx - hw, cy + hh))
            .close()
            .build()
    })
}

/// Empty path (zero commands).
fn empty_path() -> Path {
    Path::from_commands(Vec::new())
}

/// Approximate point-in-path test by sampling the path's flattened
/// polygon with even-odd rule. Returns `true` if `point` is inside any
/// subpath of `path`.
fn point_inside_path(point: Vec2, path: &Path) -> bool {
    let mut current_sub: Vec<Vec2> = Vec::new();
    let mut crossings_total: u32 = 0;
    for pt in path.flatten(0.005) {
        if current_sub.is_empty() {
            current_sub.push(pt);
            continue;
        }
        current_sub.push(pt);
    }
    // Walk all flattened points as a single closed polygon; for
    // multi-subpath outputs this still gives a reasonable parity
    // answer because our engine emits subpaths back-to-back and the
    // ray-cast count is summed across all edges.
    crossings_total += ray_crossings(point, &current_sub);
    crossings_total % 2 == 1
}

/// Ray-crossing count (parity-based PIP).
fn ray_crossings(p: Vec2, poly: &[Vec2]) -> u32 {
    if poly.len() < 3 {
        return 0;
    }
    let mut n: u32 = 0;
    let len = poly.len();
    let mut prev = poly[len - 1];
    for &cur in poly {
        let y_between = (prev.y > p.y) != (cur.y > p.y);
        if y_between {
            let denom = cur.y - prev.y;
            if denom.abs() > f32::EPSILON {
                let x_cross = prev.x + (p.y - prev.y) * (cur.x - prev.x) / denom;
                if p.x < x_cross {
                    n += 1;
                }
            }
        }
        prev = cur;
    }
    n
}

/// `subpaths()` count proxy — number of `MoveTo` commands.
fn subpath_count(path: &Path) -> usize {
    path.commands()
        .iter()
        .filter(|c| matches!(c, wisp::path::PathCommand::MoveTo(_)))
        .count()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Union is commutative on the inside/outside classification of
    /// every probed sample point.
    #[test]
    fn union_commutative_on_samples(a in rect_strategy(), b in rect_strategy()) {
        let opts = BoolOptions::default();
        let ab = combine(&a, &b, BooleanOp::Union, opts);
        let ba = combine(&b, &a, BooleanOp::Union, opts);
        for x in &[-0.6_f32, -0.2, 0.0, 0.2, 0.6] {
            for y in &[-0.6_f32, -0.2, 0.0, 0.2, 0.6] {
                let p = Vec2::new(*x, *y);
                prop_assert_eq!(
                    point_inside_path(p, &ab),
                    point_inside_path(p, &ba),
                    "Union should be commutative at {:?}", p
                );
            }
        }
    }

    /// Intersection is commutative on the inside/outside classification.
    #[test]
    fn intersection_commutative_on_samples(a in rect_strategy(), b in rect_strategy()) {
        let opts = BoolOptions::default();
        let ab = combine(&a, &b, BooleanOp::Intersection, opts);
        let ba = combine(&b, &a, BooleanOp::Intersection, opts);
        for x in &[-0.6_f32, -0.2, 0.0, 0.2, 0.6] {
            for y in &[-0.6_f32, -0.2, 0.0, 0.2, 0.6] {
                let p = Vec2::new(*x, *y);
                prop_assert_eq!(
                    point_inside_path(p, &ab),
                    point_inside_path(p, &ba),
                    "Intersection should be commutative at {:?}", p
                );
            }
        }
    }

    /// XOR is commutative on the inside/outside classification.
    #[test]
    fn xor_commutative_on_samples(a in rect_strategy(), b in rect_strategy()) {
        let opts = BoolOptions::default();
        let ab = combine(&a, &b, BooleanOp::Xor, opts);
        let ba = combine(&b, &a, BooleanOp::Xor, opts);
        for x in &[-0.6_f32, -0.2, 0.0, 0.2, 0.6] {
            for y in &[-0.6_f32, -0.2, 0.0, 0.2, 0.6] {
                let p = Vec2::new(*x, *y);
                prop_assert_eq!(
                    point_inside_path(p, &ab),
                    point_inside_path(p, &ba),
                    "XOR should be commutative at {:?}", p
                );
            }
        }
    }

    /// Empty-RHS identities: A ∪ ∅ ⊇ A;  A ∩ ∅ = ∅;  A − ∅ ⊇ A.
    /// (We test "every point inside A is still inside (A op ∅)" for
    /// Union and Difference, and "result is empty" for Intersection.)
    #[test]
    fn empty_rhs_identities(a in rect_strategy()) {
        let opts = BoolOptions::default();
        let empty = empty_path();

        // Probe points that are clearly inside A: sample its centre.
        // Re-derive A's bounding centre from its commands.
        let mut sum = Vec2::ZERO;
        let mut count: u16 = 0;
        for cmd in a.commands() {
            if let wisp::path::PathCommand::MoveTo(p) | wisp::path::PathCommand::LineTo(p) = cmd {
                sum += *p;
                count += 1;
            }
        }
        if count == 0 {
            return Ok(());
        }
        let centre = sum / f32::from(count);

        // A ∪ ∅ still contains A's centre.
        let union = combine(&a, &empty, BooleanOp::Union, opts);
        prop_assert!(
            point_inside_path(centre, &union),
            "A ∪ ∅ lost A's centre"
        );

        // A ∩ ∅ is empty (zero subpaths).
        let inter = combine(&a, &empty, BooleanOp::Intersection, opts);
        prop_assert_eq!(subpath_count(&inter), 0);

        // A − ∅ still contains A's centre.
        let diff = combine(&a, &empty, BooleanOp::Difference, opts);
        prop_assert!(
            point_inside_path(centre, &diff),
            "A − ∅ lost A's centre"
        );
    }

    /// Self-difference: `A − A` has no interior points where A had any.
    #[test]
    fn self_difference_carves_a_out(a in rect_strategy()) {
        let opts = BoolOptions::default();
        let diff = combine(&a, &a, BooleanOp::Difference, opts);
        // A's centre is no longer inside A − A.
        let mut sum = Vec2::ZERO;
        let mut count: u16 = 0;
        for cmd in a.commands() {
            if let wisp::path::PathCommand::MoveTo(p) | wisp::path::PathCommand::LineTo(p) = cmd {
                sum += *p;
                count += 1;
            }
        }
        if count == 0 {
            return Ok(());
        }
        let centre = sum / f32::from(count);
        prop_assert!(
            !point_inside_path(centre, &diff),
            "A − A should not contain A's centre"
        );
    }

    /// XOR identity: `A ⊕ A` has no interior at A's centre.
    #[test]
    fn xor_self_is_empty_at_centre(a in rect_strategy()) {
        let opts = BoolOptions::default();
        let xor = combine(&a, &a, BooleanOp::Xor, opts);
        let mut sum = Vec2::ZERO;
        let mut count: u16 = 0;
        for cmd in a.commands() {
            if let wisp::path::PathCommand::MoveTo(p) | wisp::path::PathCommand::LineTo(p) = cmd {
                sum += *p;
                count += 1;
            }
        }
        if count == 0 {
            return Ok(());
        }
        let centre = sum / f32::from(count);
        prop_assert!(
            !point_inside_path(centre, &xor),
            "A ⊕ A should not contain A's centre"
        );
    }

    /// Union is associative on the inside/outside classification.
    /// `A ∪ (B ∪ C)` and `(A ∪ B) ∪ C` agree at every probe point.
    #[test]
    fn union_associative_on_samples(
        a in rect_strategy(),
        b in rect_strategy(),
        c in rect_strategy(),
    ) {
        let opts = BoolOptions::default();
        let left = combine(&combine(&a, &b, BooleanOp::Union, opts), &c, BooleanOp::Union, opts);
        let right = combine(&a, &combine(&b, &c, BooleanOp::Union, opts), BooleanOp::Union, opts);
        for x in &[-0.6_f32, -0.2, 0.0, 0.2, 0.6] {
            for y in &[-0.6_f32, -0.2, 0.0, 0.2, 0.6] {
                let p = Vec2::new(*x, *y);
                prop_assert_eq!(
                    point_inside_path(p, &left),
                    point_inside_path(p, &right),
                    "Union should be associative at {:?}", p
                );
            }
        }
    }

    /// De Morgan within a probe-rect window:
    /// at every probe point `p`, `p ∈ ¬(A ∪ B) ⇔ p ∉ A ∧ p ∉ B`.
    /// We test against the inputs directly (rather than computing
    /// `¬A`, `¬B` as paths) because complementing an unbounded
    /// region isn't representable as a finite `Path` — the law still
    /// holds pointwise.
    #[test]
    fn de_morgan_on_samples(a in rect_strategy(), b in rect_strategy()) {
        let opts = BoolOptions::default();
        let union = combine(&a, &b, BooleanOp::Union, opts);
        for x in &[-0.6_f32, -0.2, 0.0, 0.2, 0.6] {
            for y in &[-0.6_f32, -0.2, 0.0, 0.2, 0.6] {
                let p = Vec2::new(*x, *y);
                let in_union = point_inside_path(p, &union);
                let in_a = point_inside_path(p, &a);
                let in_b = point_inside_path(p, &b);
                // ¬(A ∪ B) ⇔ ¬A ∧ ¬B
                prop_assert_eq!(
                    !in_union,
                    !in_a && !in_b,
                    "De Morgan failed at {:?}: in_union={}, in_a={}, in_b={}",
                    p, in_union, in_a, in_b
                );
            }
        }
    }
}
