//! Shared scene traversal + transform helpers (M-TEXT.0 / AUT-74).
//!
//! Before this module, `sprite_pipeline`, `graphics_pipeline`,
//! `mesh_pipeline`, and `text_pipeline` each duplicated the same
//! pre-order subtree walk: stack-based DFS, exclude-set filter,
//! visibility skip, parent-world transform accumulation, and a local
//! `mat3_to_mat4` helper. This module owns those mechanics so the
//! upcoming text backends (`AtlasText`, `FlexibleText`) and any
//! future pipeline pick them up for free.
//!
//! No behavior change: every existing pipeline calls
//! [`walk_visible_subtree`] with the same `(id, node, world)` callback
//! semantics it had before.

use std::collections::HashSet;

use glam::{Mat3, Mat4, Vec4};

use crate::scene::{Node, NodeId, Stage};

/// Walk every visible node in the subtree rooted at `start` in
/// pre-order, calling `visit` with the node id, the borrowed
/// [`Node`], and its world-space transform (parent's world × local).
///
/// Nodes whose ids are in `exclude` (and their descendants) are
/// skipped entirely — used by the auto-dispatch advanced-blend
/// renderer to walk the scene "minus" the dispatched subtrees.
/// Invisible nodes (`container.visible = false`) skip themselves
/// *and* their subtree, matching previous per-pipeline behavior.
///
/// Children are pushed in *reverse* insertion order so siblings pop
/// in insertion order — locked-in z-order semantics.
pub(crate) fn walk_visible_subtree<F>(
    stage: &Stage,
    start: NodeId,
    exclude: &HashSet<NodeId>,
    mut visit: F,
) where
    F: FnMut(NodeId, &Node, Mat4),
{
    let mut stack: Vec<(NodeId, Mat4)> = vec![(start, Mat4::IDENTITY)];
    while let Some((id, parent_world)) = stack.pop() {
        if exclude.contains(&id) {
            continue;
        }
        let Some(node) = stage.get(id) else {
            continue;
        };
        let container = node.container();
        if !container.visible {
            continue;
        }
        let local = mat3_to_mat4(container.transform.to_mat3());
        let world = parent_world * local;

        visit(id, node, world);

        // Push children in reverse so siblings pop in insertion order.
        for child in container.children().rev().collect::<Vec<_>>() {
            stack.push((child, world));
        }
    }
}

/// Lift a 2-D affine [`Mat3`] into a 4-D homogeneous [`Mat4`] for
/// the WGPU-side vertex shaders that consume `mat4`s.
///
/// Lives here instead of being copy-pasted in every pipeline.
#[must_use]
pub(crate) fn mat3_to_mat4(m: Mat3) -> Mat4 {
    Mat4::from_cols(
        Vec4::new(m.x_axis.x, m.x_axis.y, 0.0, 0.0),
        Vec4::new(m.y_axis.x, m.y_axis.y, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(m.z_axis.x, m.z_axis.y, 0.0, 1.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Container, Stage};

    #[test]
    fn traversal_visits_every_visible_node_in_preorder() {
        let mut stage = Stage::new();
        let root = stage.root();
        let a = stage.add_child(root, Container::default()).unwrap();
        let b = stage.add_child(a, Container::default()).unwrap();
        let c = stage.add_child(b, Container::default()).unwrap();
        let d = stage.add_child(root, Container::default()).unwrap();

        let mut order = Vec::new();
        walk_visible_subtree(&stage, root, &HashSet::new(), |id, _, _| order.push(id));
        assert_eq!(order, vec![root, a, b, c, d]);
    }

    #[test]
    fn excluded_subtree_is_skipped() {
        let mut stage = Stage::new();
        let root = stage.root();
        let a = stage.add_child(root, Container::default()).unwrap();
        let b = stage.add_child(a, Container::default()).unwrap();
        let _c = stage.add_child(b, Container::default()).unwrap();
        let d = stage.add_child(root, Container::default()).unwrap();

        let mut exclude = HashSet::new();
        exclude.insert(a);

        let mut order = Vec::new();
        walk_visible_subtree(&stage, root, &exclude, |id, _, _| order.push(id));
        // a, b, c never visited; root and d still are.
        assert_eq!(order, vec![root, d]);
    }

    #[test]
    fn invisible_node_skips_itself_and_descendants() {
        let mut stage = Stage::new();
        let root = stage.root();
        let hidden_container = Container {
            visible: false,
            ..Container::default()
        };
        let hidden = stage.add_child(root, hidden_container).unwrap();
        let _child_of_hidden = stage.add_child(hidden, Container::default()).unwrap();
        let visible = stage.add_child(root, Container::default()).unwrap();

        let mut order = Vec::new();
        walk_visible_subtree(&stage, root, &HashSet::new(), |id, _, _| order.push(id));
        assert_eq!(order, vec![root, visible]);
    }

    #[test]
    fn world_transform_accumulates_along_chain() {
        use crate::scene::Transform;
        let mut stage = Stage::new();
        let root = stage.root();
        let child = Container {
            transform: Transform {
                position: glam::Vec2::new(0.5, 0.0),
                ..Transform::default()
            },
            ..Container::default()
        };
        let cid = stage.add_child(root, child).unwrap();

        let grand = Container {
            transform: Transform {
                position: glam::Vec2::new(0.5, 0.0),
                ..Transform::default()
            },
            ..Container::default()
        };
        let gid = stage.add_child(cid, grand).unwrap();

        let mut worlds = Vec::new();
        walk_visible_subtree(&stage, root, &HashSet::new(), |id, _, world| {
            worlds.push((id, world.w_axis));
        });
        // Grandchild's world translation = (0.5 + 0.5, 0).
        let (id, w) = worlds.into_iter().find(|(id, _)| *id == gid).unwrap();
        assert_eq!(id, gid);
        assert!((w.x - 1.0).abs() < 1e-6);
    }

    #[test]
    fn mat3_to_mat4_preserves_2d_affine_block() {
        let m = Mat3::from_cols(
            glam::Vec3::new(2.0, 0.0, 0.0),
            glam::Vec3::new(0.0, 3.0, 0.0),
            glam::Vec3::new(5.0, 7.0, 1.0),
        );
        let lifted = mat3_to_mat4(m);
        // Top-left scale.
        assert!((lifted.x_axis.x - 2.0).abs() < f32::EPSILON);
        assert!((lifted.y_axis.y - 3.0).abs() < f32::EPSILON);
        // Translation lands in z-axis (column 3 lower-left).
        assert!((lifted.w_axis.x - 5.0).abs() < f32::EPSILON);
        assert!((lifted.w_axis.y - 7.0).abs() < f32::EPSILON);
        // z-axis identity.
        assert!((lifted.z_axis.z - 1.0).abs() < f32::EPSILON);
    }
}
