//! Path boolean ops (M-BOOL.0 / AUT-161 → M-BOOL.5 / AUT-166).
//!
//! In-house polygon-clipping engine. The full design rationale —
//! algorithm choice, alternatives rejected, follow-up tickets — is
//! captured in `_docs/adr/M-BOOL-backend.md`.
//!
//! # Quick API
//!
//! ```ignore
//! use wisp::path::boolean::{combine, BooleanOp, BoolOptions};
//! use wisp::path::PathBuilder;
//!
//! let circle_a = /* a closed Path */;
//! let circle_b = /* a closed Path */;
//! let union = combine(&circle_a, &circle_b, BooleanOp::Union, BoolOptions::default());
//! ```
//!
//! # Algorithm (v1, polygon-only)
//!
//! 1. Flatten each `Path` to a list of closed polylines (one per
//!    `MoveTo`-rooted subpath).
//! 2. Build a directed-edge list per polyline, labelled `Subject`
//!    (path A) or `Clip` (path B).
//! 3. Find every pair-wise edge intersection in O(n·m). Subdivide
//!    both edges at each intersection so every output fragment has
//!    integer-multiplicity endpoints.
//! 4. For each fragment, evaluate "inside A?" and "inside B?" at the
//!    fragment's midpoint via the parity (even-odd) point-in-polygon
//!    test against the *other* polygon's full edge list.
//! 5. Keep each fragment iff it lies on the boundary of the desired
//!    output region — i.e. the op rule evaluates differently on the
//!    two sides of the fragment.
//! 6. Stitch retained fragments tip-to-tail into closed contours;
//!    emit one `MoveTo`+`LineTo*`+`Close` subpath per contour.
//!
//! # Known v1 limitations
//!
//! All deferred to follow-up tickets (AUT-167..179):
//!
//! - Bezier curves flatten via [`crate::scene::path::Path::flatten`]
//!   before processing; curvature is lost. M-BOOL.7 lands a
//!   `flatten_subpaths` that preserves multi-subpath structure.
//! - Holes + `FillRule::NonZero` semantics are stubbed:
//!   `BoolOptions::fill_rule` is accepted but only `EvenOdd` is
//!   honoured today. M-BOOL.8 implements winding-number tracking.
//! - Self-intersecting inputs → undefined output. Match Clipper2 v1.
//! - O(n·m) intersection finding is fine for our typical path sizes
//!   (50–500 vertices). M-BOOL.17 benchmarks set the bar for a
//!   future Bentley-Ottmann sweep-line if needed.

use glam::Vec2;

use crate::scene::path::{Path, PathBuilder, PathCommand};

/// The four primitive boolean ops on two paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BooleanOp {
    /// `A ∪ B` — everything in either path.
    Union,
    /// `A ∩ B` — only where both paths overlap.
    Intersection,
    /// `A − B` — `A` minus the overlap with `B`.
    Difference,
    /// `A ⊕ B` — symmetric difference (in either but not both).
    Xor,
}

/// Fill-rule policy for self-overlapping inputs.
///
/// `EvenOdd` is parity-based: a point is "in" if a ray from it
/// crosses the boundary an odd number of times. Matches existing
/// `wisp::Graphics` defaults.
///
/// `NonZero` is winding-number-based: signed crossings sum non-zero.
/// Currently accepted but treated as `EvenOdd` until M-BOOL.8 lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FillRule {
    /// Parity rule. Default — matches `Graphics` today.
    #[default]
    EvenOdd,
    /// Winding-number rule. Deferred to M-BOOL.8.
    NonZero,
}

/// Tuning knobs for [`combine`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoolOptions {
    /// Geometric tolerance — points closer than this are treated as
    /// identical for intersection / endpoint-snap logic. In whatever
    /// unit the path lives in (NDC for typical wisp usage).
    pub tolerance: f32,
    /// How "interior" is determined when classifying fragments. See
    /// [`FillRule`].
    pub fill_rule: FillRule,
    /// Bezier-flattening tolerance forwarded to
    /// [`crate::scene::path::Path::flatten`]. Smaller = more output
    /// vertices but smoother curves.
    pub flatten_tolerance: f32,
}

impl Default for BoolOptions {
    fn default() -> Self {
        Self {
            tolerance: 1e-4,
            fill_rule: FillRule::EvenOdd,
            flatten_tolerance: 0.005,
        }
    }
}

/// Combine two paths via the given boolean op.
///
/// Returns a new `Path` whose subpaths bound the requested region.
/// An empty result (e.g. `Intersection` of disjoint shapes) is a
/// `Path` with no commands.
#[must_use]
pub fn combine(a: &Path, b: &Path, op: BooleanOp, opts: BoolOptions) -> Path {
    let polys_a = subpaths(a, opts.flatten_tolerance);
    let polys_b = subpaths(b, opts.flatten_tolerance);

    if polys_a.is_empty() && polys_b.is_empty() {
        return Path::from_commands(Vec::new());
    }
    // Op-specific empty-input shortcuts.
    if polys_a.is_empty() {
        return match op {
            BooleanOp::Intersection | BooleanOp::Difference => Path::from_commands(Vec::new()),
            BooleanOp::Union | BooleanOp::Xor => rebuild_from_polylines(&polys_b),
        };
    }
    if polys_b.is_empty() {
        return match op {
            BooleanOp::Intersection => Path::from_commands(Vec::new()),
            BooleanOp::Union | BooleanOp::Difference | BooleanOp::Xor => {
                rebuild_from_polylines(&polys_a)
            }
        };
    }

    let mut edges = build_edges(&polys_a, EdgeLabel::Subject);
    edges.extend(build_edges(&polys_b, EdgeLabel::Clip));
    let fragments = split_at_intersections(&edges, opts.tolerance);

    let kept: Vec<Edge> = fragments
        .into_iter()
        .filter(|frag| keep_fragment(frag, &polys_a, &polys_b, op))
        .collect();

    let contours = stitch(&kept, opts.tolerance);
    contours_to_path(&contours)
}

/// Convenience N-ary fold (M-BOOL.6 / AUT-167).
///
/// `combine_n(&[a, b, c, d], op)` is `combine(combine(combine(a, b), c), d)`.
/// Empty slice returns an empty `Path`; single-element slice returns
/// that path unchanged.
#[must_use]
pub fn combine_n(paths: &[&Path], op: BooleanOp, opts: BoolOptions) -> Path {
    match paths {
        [] => Path::from_commands(Vec::new()),
        [only] => (*only).clone(),
        [first, rest @ ..] => {
            let mut acc = (*first).clone();
            for next in rest {
                acc = combine(&acc, next, op, opts);
            }
            acc
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// Implementation
// ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeLabel {
    Subject,
    Clip,
}

#[derive(Debug, Clone, Copy)]
struct Edge {
    from: Vec2,
    to: Vec2,
    label: EdgeLabel,
}

/// Decompose a `Path` into closed polylines, one per `MoveTo`-rooted
/// subpath. Open subpaths (without `Close`) are silently dropped —
/// boolean ops are only meaningful on closed regions.
fn subpaths(path: &Path, flatten_tolerance: f32) -> Vec<Vec<Vec2>> {
    let mut subs: Vec<Vec<Vec2>> = Vec::new();
    let mut current: Option<Vec<Vec2>> = None;
    let mut subpath_start: Option<Vec2> = None;
    let mut last_point = Vec2::ZERO;
    for cmd in path.commands() {
        match *cmd {
            PathCommand::MoveTo(p) => {
                // An open subpath in progress is dropped — boolean ops
                // are only meaningful on closed regions.
                current = Some(vec![p]);
                subpath_start = Some(p);
                last_point = p;
            }
            PathCommand::LineTo(p) => {
                if let Some(v) = current.as_mut() {
                    v.push(p);
                }
                last_point = p;
            }
            PathCommand::QuadTo { control, end } => {
                if let Some(v) = current.as_mut() {
                    flatten_quad(last_point, control, end, flatten_tolerance, v);
                }
                last_point = end;
            }
            PathCommand::CubicTo { c1, c2, end } => {
                if let Some(v) = current.as_mut() {
                    flatten_cubic(last_point, c1, c2, end, flatten_tolerance, v);
                }
                last_point = end;
            }
            PathCommand::Close => {
                if let (Some(mut v), Some(start)) = (current.take(), subpath_start) {
                    if v.last().copied().unwrap_or(start) != start {
                        v.push(start);
                    }
                    // Drop near-duplicate trailing point so the
                    // close edge doesn't have zero length.
                    if v.len() >= 2 && (v[v.len() - 1] - v[v.len() - 2]).length() < 1e-9 {
                        v.pop();
                    }
                    if v.len() >= 3 {
                        subs.push(v);
                    }
                }
                if let Some(start) = subpath_start {
                    last_point = start;
                }
            }
        }
    }
    subs
}

fn build_edges(polys: &[Vec<Vec2>], label: EdgeLabel) -> Vec<Edge> {
    let mut edges = Vec::new();
    for poly in polys {
        for window in poly.windows(2) {
            let from = window[0];
            let to = window[1];
            if (to - from).length() > f32::EPSILON {
                edges.push(Edge { from, to, label });
            }
        }
        // Wrap from last → first (close).
        if poly.len() >= 2 {
            let from = poly[poly.len() - 1];
            let to = poly[0];
            if (to - from).length() > f32::EPSILON {
                edges.push(Edge { from, to, label });
            }
        }
    }
    edges
}

/// Split every edge at its intersections with every other edge.
/// Returns the (possibly larger) fragment list.
fn split_at_intersections(edges: &[Edge], tolerance: f32) -> Vec<Edge> {
    // Collect intersection parameters per edge.
    let mut splits: Vec<Vec<f32>> = vec![Vec::new(); edges.len()];
    for i in 0..edges.len() {
        for j in (i + 1)..edges.len() {
            if let Some((t_i, t_j, _pt)) = segment_intersection(
                edges[i].from,
                edges[i].to,
                edges[j].from,
                edges[j].to,
                tolerance,
            ) {
                if t_i > tolerance && t_i < 1.0 - tolerance {
                    splits[i].push(t_i);
                }
                if t_j > tolerance && t_j < 1.0 - tolerance {
                    splits[j].push(t_j);
                }
            }
        }
    }

    let mut fragments = Vec::new();
    for (i, edge) in edges.iter().enumerate() {
        let mut params: Vec<f32> = splits[i].clone();
        params.push(0.0);
        params.push(1.0);
        params.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        params.dedup_by(|a, b| (*a - *b).abs() < tolerance);
        for w in params.windows(2) {
            let t0 = w[0];
            let t1 = w[1];
            if t1 - t0 < tolerance {
                continue;
            }
            let p0 = edge.from + (edge.to - edge.from) * t0;
            let p1 = edge.from + (edge.to - edge.from) * t1;
            if (p1 - p0).length() < tolerance {
                continue;
            }
            fragments.push(Edge {
                from: p0,
                to: p1,
                label: edge.label,
            });
        }
    }
    fragments
}

/// Decide whether a fragment is retained for the given op.
fn keep_fragment(frag: &Edge, polys_a: &[Vec<Vec2>], polys_b: &[Vec<Vec2>], op: BooleanOp) -> bool {
    // Evaluate at the fragment midpoint nudged perpendicularly to
    // either side of the edge. If the two sides disagree on
    // "inside", this fragment is on the output boundary.
    let mid = (frag.from + frag.to) * 0.5;
    let dir = (frag.to - frag.from).normalize_or_zero();
    let normal = Vec2::new(-dir.y, dir.x);
    let eps = 1e-4;
    let p_pos = mid + normal * eps;
    let p_neg = mid - normal * eps;

    let lhs_in_a = inside_any(p_pos, polys_a);
    let rhs_in_a = inside_any(p_neg, polys_a);
    let lhs_in_b = inside_any(p_pos, polys_b);
    let rhs_in_b = inside_any(p_neg, polys_b);

    op_rule(op, lhs_in_a, lhs_in_b) != op_rule(op, rhs_in_a, rhs_in_b)
}

fn op_rule(op: BooleanOp, in_a: bool, in_b: bool) -> bool {
    match op {
        BooleanOp::Union => in_a || in_b,
        BooleanOp::Intersection => in_a && in_b,
        BooleanOp::Difference => in_a && !in_b,
        BooleanOp::Xor => in_a ^ in_b,
    }
}

fn inside_any(p: Vec2, polys: &[Vec<Vec2>]) -> bool {
    let mut crossings = 0;
    for poly in polys {
        crossings += ray_crossings(p, poly);
    }
    crossings % 2 == 1
}

/// Parity / even-odd point-in-polygon. Returns the number of times a
/// rightward ray from `point` crosses any edge of the polygon.
fn ray_crossings(point: Vec2, poly: &[Vec2]) -> usize {
    let mut crossings = 0;
    let len = poly.len();
    if len < 3 {
        return 0;
    }
    for i in 0..len {
        let edge_start = poly[i];
        let edge_end = poly[(i + 1) % len];
        let intersects = ((edge_start.y > point.y) != (edge_end.y > point.y)) && {
            let t = (point.y - edge_start.y) / (edge_end.y - edge_start.y);
            let x_at = edge_start.x + t * (edge_end.x - edge_start.x);
            point.x < x_at
        };
        if intersects {
            crossings += 1;
        }
    }
    crossings
}

/// Robust-ish two-segment intersection. Returns
/// `(t_along_first, t_along_second, intersection_point)` when the
/// segments cross in the interior of both. Endpoint-only touches
/// are filtered out by the caller via `t > tolerance &&
/// t < 1 - tolerance`. Returns `None` for parallel, collinear, or
/// near-collinear pairs.
fn segment_intersection(
    a0: Vec2,
    a1: Vec2,
    b0: Vec2,
    b1: Vec2,
    tolerance: f32,
) -> Option<(f32, f32, Vec2)> {
    let a_dir = a1 - a0;
    let b_dir = b1 - b0;
    let denom = a_dir.x * b_dir.y - a_dir.y * b_dir.x;
    if denom.abs() < tolerance.max(1e-9) {
        return None;
    }
    let delta = b0 - a0;
    let mut t = (delta.x * b_dir.y - delta.y * b_dir.x) / denom;
    let mut u = (delta.x * a_dir.y - delta.y * a_dir.x) / denom;
    if !(-tolerance..=1.0 + tolerance).contains(&t) {
        return None;
    }
    if !(-tolerance..=1.0 + tolerance).contains(&u) {
        return None;
    }
    t = t.clamp(0.0, 1.0);
    u = u.clamp(0.0, 1.0);
    let pt = a0 + a_dir * t;
    Some((t, u, pt))
}

/// Walk fragments tip-to-tail, building closed loops.
fn stitch(fragments: &[Edge], tolerance: f32) -> Vec<Vec<Vec2>> {
    let n = fragments.len();
    let mut used = vec![false; n];
    let mut contours: Vec<Vec<Vec2>> = Vec::new();

    for i in 0..n {
        if used[i] {
            continue;
        }
        let mut contour = vec![fragments[i].from, fragments[i].to];
        used[i] = true;
        loop {
            let tail = *contour.last().unwrap();
            // Find an unused fragment whose `from` matches the tail.
            let mut next: Option<usize> = None;
            for (j, frag) in fragments.iter().enumerate() {
                if used[j] {
                    continue;
                }
                if (frag.from - tail).length() < tolerance {
                    next = Some(j);
                    break;
                }
                if (frag.to - tail).length() < tolerance {
                    // Reversed traversal.
                    next = Some(j);
                    break;
                }
            }
            let Some(j) = next else { break };
            used[j] = true;
            let frag = fragments[j];
            let next_pt = if (frag.from - tail).length() < tolerance {
                frag.to
            } else {
                frag.from
            };
            if (next_pt - contour[0]).length() < tolerance {
                // Closed.
                break;
            }
            contour.push(next_pt);
        }
        if contour.len() >= 3 {
            contours.push(contour);
        }
    }

    contours
}

fn contours_to_path(contours: &[Vec<Vec2>]) -> Path {
    if contours.is_empty() {
        return Path::from_commands(Vec::new());
    }
    let mut builder = PathBuilder::new();
    for contour in contours {
        if contour.is_empty() {
            continue;
        }
        builder = builder.move_to(contour[0]);
        for p in &contour[1..] {
            builder = builder.line_to(*p);
        }
        builder = builder.close();
    }
    builder.build()
}

fn rebuild_from_polylines(polys: &[Vec<Vec2>]) -> Path {
    let mut builder = PathBuilder::new();
    for poly in polys {
        if poly.is_empty() {
            continue;
        }
        builder = builder.move_to(poly[0]);
        for p in &poly[1..] {
            builder = builder.line_to(*p);
        }
        builder = builder.close();
    }
    builder.build()
}

// Re-export from the parent so callers don't need a second import.
fn flatten_quad(p0: Vec2, p1: Vec2, p2: Vec2, tolerance: f32, out: &mut Vec<Vec2>) {
    if perp_distance(p1, p0, p2) <= tolerance {
        out.push(p2);
        return;
    }
    let p01 = (p0 + p1) * 0.5;
    let p12 = (p1 + p2) * 0.5;
    let mid = (p01 + p12) * 0.5;
    flatten_quad(p0, p01, mid, tolerance, out);
    flatten_quad(mid, p12, p2, tolerance, out);
}

fn flatten_cubic(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, tolerance: f32, out: &mut Vec<Vec2>) {
    let d1 = perp_distance(p1, p0, p3);
    let d2 = perp_distance(p2, p0, p3);
    if d1.max(d2) <= tolerance {
        out.push(p3);
        return;
    }
    let q01 = (p0 + p1) * 0.5;
    let q12 = (p1 + p2) * 0.5;
    let q23 = (p2 + p3) * 0.5;
    let r012 = (q01 + q12) * 0.5;
    let r123 = (q12 + q23) * 0.5;
    let mid = (r012 + r123) * 0.5;
    flatten_cubic(p0, q01, r012, mid, tolerance, out);
    flatten_cubic(mid, r123, q23, p3, tolerance, out);
}

fn perp_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len = ab.length();
    if len < f32::EPSILON {
        return (p - a).length();
    }
    let cross = (p.x - a.x) * ab.y - (p.y - a.y) * ab.x;
    cross.abs() / len
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Closed square centred at `c` with half-side `h`.
    fn square(c: Vec2, h: f32) -> Path {
        PathBuilder::new()
            .move_to(Vec2::new(c.x - h, c.y - h))
            .line_to(Vec2::new(c.x + h, c.y - h))
            .line_to(Vec2::new(c.x + h, c.y + h))
            .line_to(Vec2::new(c.x - h, c.y + h))
            .close()
            .build()
    }

    /// Count subpaths in a Path (`MoveTo`s).
    fn count_subpaths(path: &Path) -> usize {
        path.commands()
            .iter()
            .filter(|c| matches!(c, PathCommand::MoveTo(_)))
            .count()
    }

    /// Does any vertex of `path` lie inside the polygon `interior`?
    fn any_vertex_inside(path: &Path, interior: &[Vec2]) -> bool {
        for cmd in path.commands() {
            let p = match cmd {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => *p,
                _ => continue,
            };
            if ray_crossings(p, interior) % 2 == 1 {
                return true;
            }
        }
        false
    }

    #[test]
    fn defaults_are_sensible() {
        let opts = BoolOptions::default();
        assert!(opts.tolerance > 0.0);
        assert_eq!(opts.fill_rule, FillRule::EvenOdd);
        assert!(opts.flatten_tolerance > 0.0);
    }

    #[test]
    fn union_of_disjoint_squares_returns_two_subpaths() {
        let a = square(Vec2::new(-1.0, 0.0), 0.4);
        let b = square(Vec2::new(1.0, 0.0), 0.4);
        let u = combine(&a, &b, BooleanOp::Union, BoolOptions::default());
        assert_eq!(count_subpaths(&u), 2);
    }

    #[test]
    fn intersection_of_disjoint_squares_is_empty() {
        let a = square(Vec2::new(-1.0, 0.0), 0.4);
        let b = square(Vec2::new(1.0, 0.0), 0.4);
        let i = combine(&a, &b, BooleanOp::Intersection, BoolOptions::default());
        assert_eq!(count_subpaths(&i), 0);
    }

    #[test]
    fn intersection_of_overlapping_squares_is_a_smaller_square() {
        let a = square(Vec2::ZERO, 0.5); // x ∈ [-0.5, 0.5]
        let b = square(Vec2::new(0.3, 0.0), 0.5); // x ∈ [-0.2, 0.8]
        let i = combine(&a, &b, BooleanOp::Intersection, BoolOptions::default());
        assert_eq!(count_subpaths(&i), 1);
        // The intersection rectangle is x ∈ [-0.2, 0.5], y ∈ [-0.5, 0.5].
        // Centre point should be inside the intersection contour.
        let centre = Vec2::new(0.15, 0.0);
        let polys: Vec<Vec<Vec2>> = subpaths(&i, 0.005);
        assert!(inside_any(centre, &polys), "centre of overlap missing");
        let outside = Vec2::new(-0.4, 0.0);
        assert!(!inside_any(outside, &polys), "non-overlap region kept");
    }

    #[test]
    fn difference_carves_b_out_of_a() {
        let a = square(Vec2::ZERO, 0.5);
        let b = square(Vec2::new(0.3, 0.0), 0.5);
        let d = combine(&a, &b, BooleanOp::Difference, BoolOptions::default());
        assert!(
            count_subpaths(&d) >= 1,
            "diff should leave at least one contour"
        );
        let polys: Vec<Vec<Vec2>> = subpaths(&d, 0.005);
        // Point only in A: kept.
        assert!(inside_any(Vec2::new(-0.4, 0.0), &polys));
        // Point in overlap: removed.
        assert!(!inside_any(Vec2::new(0.15, 0.0), &polys));
        // Point only in B: not kept (difference is A − B).
        assert!(!inside_any(Vec2::new(0.7, 0.0), &polys));
    }

    #[test]
    fn xor_keeps_outer_regions_drops_overlap() {
        let a = square(Vec2::ZERO, 0.5);
        let b = square(Vec2::new(0.3, 0.0), 0.5);
        let x = combine(&a, &b, BooleanOp::Xor, BoolOptions::default());
        let polys: Vec<Vec<Vec2>> = subpaths(&x, 0.005);
        // Overlap dropped.
        assert!(!inside_any(Vec2::new(0.15, 0.0), &polys));
        // Outer fragment of A kept.
        assert!(inside_any(Vec2::new(-0.4, 0.0), &polys));
        // Outer fragment of B kept.
        assert!(inside_any(Vec2::new(0.7, 0.0), &polys));
    }

    #[test]
    fn empty_path_inputs_behave_sensibly() {
        let empty = Path::from_commands(Vec::new());
        let a = square(Vec2::ZERO, 0.5);
        let opts = BoolOptions::default();

        // Union with empty B returns A (modulo retessellation).
        let u = combine(&a, &empty, BooleanOp::Union, opts);
        assert_eq!(count_subpaths(&u), 1);

        // Intersection with empty B is empty.
        let i = combine(&a, &empty, BooleanOp::Intersection, opts);
        assert_eq!(count_subpaths(&i), 0);

        // Difference of empty A − anything is empty.
        let d = combine(&empty, &a, BooleanOp::Difference, opts);
        assert_eq!(count_subpaths(&d), 0);
    }

    #[test]
    fn combine_n_empty_slice_returns_empty() {
        let r = combine_n(&[], BooleanOp::Union, BoolOptions::default());
        assert_eq!(count_subpaths(&r), 0);
    }

    #[test]
    fn combine_n_single_returns_same_shape() {
        let a = square(Vec2::ZERO, 0.5);
        let r = combine_n(&[&a], BooleanOp::Union, BoolOptions::default());
        // Same number of MoveTo + commands.
        assert_eq!(r.commands().len(), a.commands().len());
    }

    #[test]
    fn combine_n_union_three_disjoint_is_three_subpaths() {
        let a = square(Vec2::new(-1.5, 0.0), 0.3);
        let b = square(Vec2::new(0.0, 0.0), 0.3);
        let c = square(Vec2::new(1.5, 0.0), 0.3);
        let r = combine_n(&[&a, &b, &c], BooleanOp::Union, BoolOptions::default());
        assert_eq!(count_subpaths(&r), 3);
    }

    #[test]
    fn ray_crossings_distinguishes_inside_outside() {
        let sq = vec![
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        ];
        assert_eq!(ray_crossings(Vec2::ZERO, &sq) % 2, 1, "centre is inside");
        assert_eq!(
            ray_crossings(Vec2::new(2.0, 0.0), &sq) % 2,
            0,
            "far right is outside"
        );
    }

    #[test]
    fn segment_intersection_finds_cross() {
        let r = segment_intersection(
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
            1e-6,
        );
        let (t, u, pt) = r.expect("crossed segments must intersect");
        assert!((t - 0.5).abs() < 1e-3);
        assert!((u - 0.5).abs() < 1e-3);
        assert!((pt - Vec2::ZERO).length() < 1e-3);
    }

    #[test]
    fn segment_intersection_misses_parallel() {
        let r = segment_intersection(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            1e-6,
        );
        assert!(r.is_none(), "parallel non-collinear must not cross");
    }

    #[test]
    fn any_vertex_inside_helper_works() {
        let unit_sq = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let path_inside = square(Vec2::new(0.5, 0.5), 0.1);
        assert!(any_vertex_inside(&path_inside, &unit_sq));
        let path_outside = square(Vec2::new(5.0, 5.0), 0.1);
        assert!(!any_vertex_inside(&path_outside, &unit_sq));
    }
}
