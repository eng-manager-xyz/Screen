//! `Wisp2dHitTest` — `HitTestBackend` for `wisp::Stage`.
//!
//! Build pass:
//! 1. Pre-order walk the stage from the root.
//! 2. Skip subtrees whose container is invisible.
//! 3. For each pickable node (entry in `PickableMap`, `enabled == true`),
//!    compose the world matrix by multiplying ancestor `to_mat3()`s.
//! 4. Compute the world-space AABB for the node's local-space
//!    [`HitShape`](super::HitShape) — used by the optional R-tree to
//!    drop obvious misses before the precise per-shape test.
//! 5. Record `(NodeId, world_matrix, depth)` where `depth` is a
//!    monotonically increasing counter — the LAST node visited (last
//!    drawn) gets the HIGHEST depth, which the dispatcher reads as
//!    "topmost first" by sorting hits descending on depth.
//!
//! Query pass (`pick`):
//! - With R-tree: query the tree for nodes whose world AABB contains
//!   the pointer, then run the precise local-space `HitShape::contains`
//!   on each candidate.
//! - Without R-tree: linear scan over the recorded entries; the precise
//!   test is run on every entry.
//!
//! Either way the resulting hits are sorted by depth descending so
//! index 0 is the topmost.
//!
//! Cost analysis:
//! - Build is `O(N)` in the number of stage nodes (one matrix
//!   multiply per node, plus one `HashMap` probe per pickable).
//! - Linear query is `O(P)` in the pickable count.
//! - R-tree query is `O(log P + K)` for K results; only a win once
//!   `P` is large (>~100).

use glam::{Mat3, Vec2};

use rstar::{AABB, RTree, RTreeObject};

use wisp::math::Rect;
use wisp::scene::transform::compose;
use wisp::scene::{NodeId, Stage};

use crate::hit_test::backend::HitTestBackend;
use crate::hit_test::pickable::PickableMap;
use crate::hit_test::shape::HitShape;
use crate::pointer::Hit;

/// One pickable node, snapshot at build time.
struct Entry {
    node: NodeId,
    /// World-to-local inverse — multiply the pointer by this to get
    /// local-space coords for the precise `HitShape::contains` test.
    inv_world: Mat3,
    /// World-space AABB. Used by the R-tree query, and also as the
    /// fast first-pass filter in the linear scan.
    world_aabb: Rect,
    /// Depth (monotone, higher = topmost).
    depth: usize,
    /// Owned clone of the per-node `HitShape` — cheap to clone
    /// (`Rect` / `Circle` / `Ellipse` are `Copy` payloads; `Polygon`
    /// is the only allocator).
    shape: HitShape,
}

/// R-tree element. Indexes the world-space AABB; payload is the
/// `Entry` index so the precise test stays out of the spatial query.
#[derive(Debug, Clone, Copy)]
struct IndexedAabb {
    min: [f32; 2],
    max: [f32; 2],
    entry_idx: u32,
}

impl RTreeObject for IndexedAabb {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(self.min, self.max)
    }
}

/// 2D hit-test backend over a `wisp::Stage` + `PickableMap`.
pub struct Wisp2dHitTest {
    entries: Vec<Entry>,
    index: Option<RTree<IndexedAabb>>,
}

impl Wisp2dHitTest {
    /// Build the backend (linear scan, no spatial index).
    ///
    /// Use this for scenes with up to ~100 pickable nodes — the
    /// constant factor of `RTree` insertion is not worth it below
    /// that. The Pixar Luxo / sketchpad / disney-bounce chapter
    /// examples in WI.10 all use the linear backend.
    #[must_use]
    pub fn new(stage: &Stage, pickable: &PickableMap) -> Self {
        let entries = build_entries(stage, pickable);
        Self {
            entries,
            index: None,
        }
    }

    /// Build the backend with an R-tree spatial index over the
    /// pickable nodes' world AABBs. Worth it for scenes with
    /// hundreds of pickable nodes (chart points, treemap cells).
    #[must_use]
    pub fn with_index(stage: &Stage, pickable: &PickableMap) -> Self {
        let entries = build_entries(stage, pickable);
        let index = build_rtree(&entries);
        Self {
            entries,
            index: Some(index),
        }
    }

    /// Number of pickable nodes the backend tracks. Diagnostics.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// `true` if the backend has an R-tree spatial index.
    #[must_use]
    pub fn has_index(&self) -> bool {
        self.index.is_some()
    }

    /// Internal: produce a `Hit` if `entry`'s shape contains the
    /// pointer transformed into its local space.
    fn try_hit(entry: &Entry, viewport_pointer: Vec2) -> Option<Hit> {
        // World AABB fast-reject before the precise test.
        if !entry.world_aabb.contains(viewport_pointer) {
            return None;
        }
        let local = entry.inv_world.transform_point2(viewport_pointer);
        if !entry.shape.contains(local) {
            return None;
        }
        Some(Hit {
            node: entry.node,
            // Cast: depth fits in f32 for any plausible scene size
            // (a stage with 2^24 nodes would already cost gigabytes).
            // f32-precision loss past 2^24 is acceptable for sort.
            #[allow(
                clippy::cast_precision_loss,
                reason = "depth is a sort key; precision past 2^24 nodes doesn't matter for ordering"
            )]
            depth: entry.depth as f32,
            local_pos: local,
        })
    }
}

impl HitTestBackend for Wisp2dHitTest {
    fn pick(&self, viewport_pointer: Vec2) -> Vec<Hit> {
        let mut hits: Vec<Hit> = if let Some(index) = &self.index {
            // Query the R-tree for AABBs containing the pointer, then
            // run the precise test on the candidates only. Treat the
            // probe as a degenerate (zero-area) AABB and ask rstar
            // for every envelope intersecting it.
            let probe: [f32; 2] = [viewport_pointer.x, viewport_pointer.y];
            let probe_env = AABB::from_point(probe);
            index
                .locate_in_envelope_intersecting(&probe_env)
                .filter_map(|node| {
                    let entry = &self.entries[node.entry_idx as usize];
                    Self::try_hit(entry, viewport_pointer)
                })
                .collect()
        } else {
            // Linear scan: every entry runs the AABB pre-check + precise
            // test inside `try_hit`.
            self.entries
                .iter()
                .filter_map(|entry| Self::try_hit(entry, viewport_pointer))
                .collect()
        };
        // Topmost-first ordering. f32 comparison is total here because
        // we constructed depth as a finite monotone counter.
        hits.sort_by(|a, b| {
            b.depth
                .partial_cmp(&a.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits
    }
}

/// Walk the stage and snapshot every pickable, enabled, visible node.
fn build_entries(stage: &Stage, pickable: &PickableMap) -> Vec<Entry> {
    let root = stage.root();
    let mut entries: Vec<Entry> = Vec::new();
    let mut depth_counter: usize = 0;
    walk(
        stage,
        pickable,
        root,
        Mat3::IDENTITY,
        &mut entries,
        &mut depth_counter,
    );
    entries
}

/// Recursive pre-order walk. Children visited in INSERTION ORDER —
/// matches wisp's `render_stage` draw order so the last visited node
/// is the topmost (highest depth).
fn walk(
    stage: &Stage,
    pickable: &PickableMap,
    node_id: NodeId,
    parent_world: Mat3,
    out: &mut Vec<Entry>,
    depth_counter: &mut usize,
) {
    let Some(node) = stage.get(node_id) else {
        return;
    };
    let container = node.container();

    // Skip invisible subtrees entirely (and their pickable
    // descendants). Matches the renderer's `visible == false` cull.
    if !container.visible {
        return;
    }

    let world = compose(parent_world, &container.transform);

    if let Some(entry) = pickable.get(node_id)
        && entry.enabled
        && let Some(local_bb) = entry.shape.local_aabb()
    {
        let world_aabb = transform_aabb(local_bb, world);
        // Inverse may be singular when the node was scaled to zero;
        // in that case the node is invisible and we skip it.
        let inv_world = world.inverse();
        if inv_world.is_finite() {
            *depth_counter += 1;
            out.push(Entry {
                node: node_id,
                inv_world,
                world_aabb,
                depth: *depth_counter,
                shape: entry.shape.clone(),
            });
        }
    }

    for child in container.children() {
        walk(stage, pickable, child, world, out, depth_counter);
    }
}

/// Transform an axis-aligned local rect by an affine world matrix
/// and return the AABB of the resulting (possibly-rotated) quad.
fn transform_aabb(local: Rect, world: Mat3) -> Rect {
    let max = local.max();
    let corners = [
        world.transform_point2(local.min),
        world.transform_point2(Vec2::new(max.x, local.min.y)),
        world.transform_point2(max),
        world.transform_point2(Vec2::new(local.min.x, max.y)),
    ];
    let mut min = corners[0];
    let mut maxv = corners[0];
    for c in &corners[1..] {
        min = min.min(*c);
        maxv = maxv.max(*c);
    }
    Rect::new(min.x, min.y, maxv.x - min.x, maxv.y - min.y)
}

/// Build the R-tree from world-space AABBs.
fn build_rtree(entries: &[Entry]) -> RTree<IndexedAabb> {
    // Cast: u32 is plenty for any realistic scene size (≥4B pickable
    // nodes is well past anything wisp could render in real time).
    #[allow(
        clippy::cast_possible_truncation,
        reason = "entry_idx fits in u32 — see above"
    )]
    let items: Vec<IndexedAabb> = entries
        .iter()
        .enumerate()
        .map(|(idx, e)| {
            let max = e.world_aabb.max();
            IndexedAabb {
                min: [e.world_aabb.min.x, e.world_aabb.min.y],
                max: [max.x, max.y],
                entry_idx: idx as u32,
            }
        })
        .collect();
    RTree::bulk_load(items)
}
