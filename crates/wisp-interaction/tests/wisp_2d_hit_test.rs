//! `Wisp2dHitTest` integration tests (WI.3 / AUT-306).
//!
//! Builds tiny `wisp::Stage` graphs, registers `Pickable` entries with
//! various `HitShape` variants, then probes the backend with viewport
//! points and asserts the expected hit list.

use glam::Vec2;
use wisp::math::Rect;
use wisp::scene::transform::Transform;
use wisp::scene::{Container, Stage};

use wisp_interaction::{HitShape, HitTestBackend, Pickable, PickableMap, Wisp2dHitTest};

/// Helper: build a container at (x, y) with no scaling/rotation.
fn container_at(x: f32, y: f32) -> Container {
    let mut c = Container::new();
    c.transform = Transform {
        position: Vec2::new(x, y),
        scale: Vec2::splat(1.0),
        rotation: 0.0,
        pivot: Vec2::ZERO,
        skew: Vec2::ZERO,
    };
    c
}

#[test]
fn rect_pick_returns_node_under_pointer_and_empty_outside() {
    let mut stage = Stage::new();
    let root = stage.root();
    // Node at world (100, 100) with a 50×50 local rect anchored at origin.
    let n = stage.add_child(root, container_at(100.0, 100.0)).unwrap();

    let mut pickable = PickableMap::new();
    pickable.insert_shape(n, HitShape::Rect(Rect::new(0.0, 0.0, 50.0, 50.0)));

    let backend = Wisp2dHitTest::new(&stage, &pickable);
    assert_eq!(backend.entry_count(), 1);
    assert!(!backend.has_index());

    // Inside the rect (world 125, 125 → local 25, 25).
    let hits = backend.pick(Vec2::new(125.0, 125.0));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].node, n);
    assert!((hits[0].local_pos - Vec2::new(25.0, 25.0)).length() < 1e-5);

    // Outside.
    assert!(backend.pick(Vec2::new(10.0, 10.0)).is_empty());
}

#[test]
fn topmost_hit_first_when_overlap_in_z_order() {
    let mut stage = Stage::new();
    let root = stage.root();
    // Two containers at the same world position with overlapping rects.
    // Insertion order: bottom first, top second → top should be drawn
    // last → topmost.
    let bottom = stage.add_child(root, container_at(0.0, 0.0)).unwrap();
    let top = stage.add_child(root, container_at(0.0, 0.0)).unwrap();

    let mut pickable = PickableMap::new();
    pickable.insert_shape(bottom, HitShape::Rect(Rect::new(0.0, 0.0, 50.0, 50.0)));
    pickable.insert_shape(top, HitShape::Rect(Rect::new(0.0, 0.0, 50.0, 50.0)));

    let backend = Wisp2dHitTest::new(&stage, &pickable);
    let hits = backend.pick(Vec2::new(25.0, 25.0));
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].node, top, "topmost first");
    assert_eq!(hits[1].node, bottom);
    // Higher depth = topmost.
    assert!(hits[0].depth > hits[1].depth);
}

#[test]
fn child_inherits_parent_world_transform() {
    let mut stage = Stage::new();
    let root = stage.root();
    let parent = stage.add_child(root, container_at(100.0, 0.0)).unwrap();
    // Child positioned at (50, 50) IN PARENT SPACE → world (150, 50).
    let child = stage.add_child(parent, container_at(50.0, 50.0)).unwrap();

    let mut pickable = PickableMap::new();
    pickable.insert_shape(child, HitShape::Rect(Rect::new(0.0, 0.0, 20.0, 20.0)));

    let backend = Wisp2dHitTest::new(&stage, &pickable);
    // Point at world (160, 60) should land inside the child (local 10, 10).
    let hits = backend.pick(Vec2::new(160.0, 60.0));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].node, child);
    assert!((hits[0].local_pos - Vec2::new(10.0, 10.0)).length() < 1e-5);

    // Point at world (110, 10) is inside the parent's empty zone but
    // outside the child's 20×20 rect. Since the parent is NOT pickable,
    // this returns empty.
    assert!(backend.pick(Vec2::new(110.0, 10.0)).is_empty());
}

#[test]
fn invisible_parent_culls_pickable_descendants() {
    let mut stage = Stage::new();
    let root = stage.root();
    let parent = stage.add_child(root, container_at(0.0, 0.0)).unwrap();
    let child = stage.add_child(parent, container_at(0.0, 0.0)).unwrap();

    // Make the parent invisible.
    let parent_node = stage.get_mut(parent).unwrap();
    parent_node.container_mut().visible = false;

    let mut pickable = PickableMap::new();
    pickable.insert_shape(child, HitShape::Rect(Rect::new(0.0, 0.0, 50.0, 50.0)));

    let backend = Wisp2dHitTest::new(&stage, &pickable);
    assert_eq!(
        backend.entry_count(),
        0,
        "child must be culled by invisible parent"
    );
    assert!(backend.pick(Vec2::new(25.0, 25.0)).is_empty());
}

#[test]
fn disabled_pickable_entry_is_skipped() {
    let mut stage = Stage::new();
    let root = stage.root();
    let n = stage.add_child(root, container_at(0.0, 0.0)).unwrap();

    let mut pickable = PickableMap::new();
    pickable.insert(
        n,
        Pickable::disabled(HitShape::Rect(Rect::new(0.0, 0.0, 50.0, 50.0))),
    );

    let backend = Wisp2dHitTest::new(&stage, &pickable);
    assert_eq!(backend.entry_count(), 0);
    assert!(backend.pick(Vec2::new(25.0, 25.0)).is_empty());
}

#[test]
fn circle_and_ellipse_shapes_test_correctly() {
    let mut stage = Stage::new();
    let root = stage.root();
    let c = stage.add_child(root, container_at(50.0, 50.0)).unwrap();
    let e = stage.add_child(root, container_at(200.0, 50.0)).unwrap();

    let mut pickable = PickableMap::new();
    pickable.insert_shape(
        c,
        HitShape::Circle {
            center: Vec2::ZERO,
            radius: 10.0,
        },
    );
    pickable.insert_shape(
        e,
        HitShape::Ellipse {
            center: Vec2::ZERO,
            radii: Vec2::new(30.0, 5.0),
        },
    );

    let backend = Wisp2dHitTest::new(&stage, &pickable);
    // Circle: world (53, 54) → local (3, 4) → 3^2 + 4^2 = 25 = r^2 → inside.
    assert_eq!(backend.pick(Vec2::new(53.0, 54.0))[0].node, c);
    // Circle: world (60, 60) → local (10, 10) → 200 > 100 → outside.
    assert!(backend.pick(Vec2::new(60.0, 60.0)).is_empty());
    // Ellipse: world (220, 52) → local (20, 2) → (20/30)^2 + (2/5)^2 = 0.444 + 0.16 < 1 → inside.
    assert_eq!(backend.pick(Vec2::new(220.0, 52.0))[0].node, e);
}

#[test]
fn rtree_indexed_backend_returns_same_hits_as_linear() {
    let mut stage = Stage::new();
    let root = stage.root();
    let mut pickable = PickableMap::new();

    // Build a 10×10 grid of 5×5 cells, each pickable. 100 nodes total.
    for row in 0_u16..10 {
        for col in 0_u16..10 {
            let x = f32::from(col) * 10.0;
            let y = f32::from(row) * 10.0;
            let n = stage.add_child(root, container_at(x, y)).unwrap();
            pickable.insert_shape(n, HitShape::Rect(Rect::new(0.0, 0.0, 5.0, 5.0)));
        }
    }

    let linear = Wisp2dHitTest::new(&stage, &pickable);
    let indexed = Wisp2dHitTest::with_index(&stage, &pickable);
    assert!(!linear.has_index());
    assert!(indexed.has_index());

    // Probe a handful of points and assert both backends agree.
    for probe in [
        Vec2::new(2.0, 2.0),
        Vec2::new(7.0, 7.0), // outside all cells (cells are 5px, gap is 5px)
        Vec2::new(42.0, 12.0),
        Vec2::new(99.0, 99.0),
    ] {
        let lh = linear.pick(probe);
        let ih = indexed.pick(probe);
        let l_ids: Vec<_> = lh.iter().map(|h| h.node).collect();
        let i_ids: Vec<_> = ih.iter().map(|h| h.node).collect();
        assert_eq!(
            l_ids, i_ids,
            "linear vs R-tree disagreed at probe {probe:?}: linear={l_ids:?} rtree={i_ids:?}"
        );
    }
}

#[test]
fn polygon_l_shape_hits_inside_excludes_notch() {
    let mut stage = Stage::new();
    let root = stage.root();
    let n = stage.add_child(root, container_at(0.0, 0.0)).unwrap();

    // L-shape: outer square (0,0)-(20,20) with a (10,10)-(20,20) notch.
    let l_path = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(20.0, 0.0),
        Vec2::new(20.0, 10.0),
        Vec2::new(10.0, 10.0),
        Vec2::new(10.0, 20.0),
        Vec2::new(0.0, 20.0),
    ];

    let mut pickable = PickableMap::new();
    pickable.insert_shape(n, HitShape::Polygon(l_path));

    let backend = Wisp2dHitTest::new(&stage, &pickable);
    // (5, 5) is inside the L.
    assert_eq!(backend.pick(Vec2::new(5.0, 5.0))[0].node, n);
    // (15, 15) is in the notch.
    assert!(backend.pick(Vec2::new(15.0, 15.0)).is_empty());
}
