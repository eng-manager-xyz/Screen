//! Scene graph — `Container`, `Sprite`, `Graphics`, `Text`, `Mesh`, transforms, clip masks.
//!
//! M0.8 introduces the `Stage` root and the `Node`/`NodeId` storage. Submodules
//! host the concrete node types as they land.

pub mod callout;
pub mod clip;
pub mod container;
pub mod dim_outside;
pub mod flex_text;
pub mod graphics;
pub mod highlight;
pub mod mesh;
pub mod node;
pub mod path;
pub mod privacy_blur;
pub mod sprite;
pub mod text;
pub mod transform;
pub mod vector;

use slotmap::SlotMap;

pub use callout::Callout;
pub use clip::MaskShape;
pub use container::Container;
pub use dim_outside::{DimOutside, DimStrength};
pub use flex_text::FlexText;
pub use graphics::{Fill, Graphics, Stroke};
pub use highlight::Highlight;
pub use mesh::Mesh;
pub use node::{Node, NodeId};
pub use path::{Path, PathBuilder, PathCommand};
pub use privacy_blur::{BlurStrength, PrivacyBlur};
pub use sprite::Sprite;
pub use text::{Font, Text};
pub use transform::Transform;
pub use vector::{Vector, VectorShape, VectorStroke};

/// Scene graph root.
///
/// Owns all nodes via a [`SlotMap`] keyed by [`NodeId`]. The tree is rooted at
/// a `Container` returned by [`Stage::root`]. Child relationships are tracked
/// inside each `Container`.
pub struct Stage {
    nodes: SlotMap<NodeId, Node>,
    root: NodeId,
}

impl Stage {
    /// Construct a new stage with an empty root container.
    #[must_use]
    pub fn new() -> Self {
        let mut nodes = SlotMap::with_key();
        let root = nodes.insert(Node::from(Container::default()));
        Self { nodes, root }
    }

    /// The root node ID. Always a [`Container`].
    #[must_use]
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// Total node count across the whole graph (including the root).
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// `true` iff only the root exists.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.len() == 1
    }

    /// Borrow a node by ID.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Mutably borrow a node by ID.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    /// Insert `child` as a child of `parent`. Returns the new node ID.
    ///
    /// `child`'s `parent` field is set; `parent`'s `children` list is appended.
    ///
    /// Returns `None` if `parent` doesn't exist.
    pub fn add_child(&mut self, parent: NodeId, child: impl Into<Node>) -> Option<NodeId> {
        if !self.nodes.contains_key(parent) {
            return None;
        }
        let id = self.nodes.insert(child.into());
        if let Some(parent_node) = self.nodes.get_mut(parent) {
            parent_node.container_mut().children.push(id);
        }
        if let Some(child_node) = self.nodes.get_mut(id) {
            child_node.container_mut().parent = Some(parent);
        }
        Some(id)
    }

    /// Detach `child` from its parent without destroying it.
    ///
    /// After this call the child is an orphan still reachable via [`Stage::get`]
    /// but no longer in any container's `children` list. Returns `true` if the
    /// child was attached.
    pub fn detach(&mut self, child: NodeId) -> bool {
        let parent_id = self.nodes.get(child).and_then(|n| n.container().parent());
        let Some(parent_id) = parent_id else {
            return false;
        };
        if let Some(parent) = self.nodes.get_mut(parent_id) {
            parent.container_mut().children.retain(|&id| id != child);
        }
        if let Some(child_node) = self.nodes.get_mut(child) {
            child_node.container_mut().parent = None;
        }
        true
    }

    /// Destroy a node and (recursively) all its descendants. Detaches from
    /// parent first.
    ///
    /// Returns `true` if the node existed.
    pub fn destroy(&mut self, id: NodeId) -> bool {
        if id == self.root {
            return false;
        }
        if !self.nodes.contains_key(id) {
            return false;
        }
        self.detach(id);
        let descendants = self.collect_descendants(id);
        for d in descendants {
            self.nodes.remove(d);
        }
        self.nodes.remove(id);
        true
    }

    /// Pre-order traversal from a starting node. Visits the start first, then
    /// each child's subtree in insertion order.
    pub fn traverse_pre_order(&self, start: NodeId, mut visit: impl FnMut(NodeId, &Node)) {
        let mut stack: Vec<NodeId> = vec![start];
        while let Some(id) = stack.pop() {
            let Some(node) = self.nodes.get(id) else {
                continue;
            };
            visit(id, node);
            // Push children in reverse so they're popped in insertion order.
            for child in node.container().children.iter().rev() {
                stack.push(*child);
            }
        }
    }

    fn collect_descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack: Vec<NodeId> = self
            .nodes
            .get(id)
            .map(|n| n.container().children.clone())
            .unwrap_or_default();
        while let Some(curr) = stack.pop() {
            out.push(curr);
            if let Some(node) = self.nodes.get(curr) {
                stack.extend(node.container().children.iter().copied());
            }
        }
        out
    }
}

impl Default for Stage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stage_has_only_root() {
        let stage = Stage::new();
        assert_eq!(stage.len(), 1);
        assert!(stage.is_empty());
        assert!(stage.get(stage.root()).is_some());
    }

    #[test]
    fn add_child_returns_id_and_links_both_sides() {
        let mut stage = Stage::new();
        let root = stage.root();
        let child = stage.add_child(root, Container::default()).expect("added");

        assert_eq!(stage.len(), 2);
        assert!(!stage.is_empty());
        assert_eq!(stage.get(child).unwrap().container().parent(), Some(root));
        assert_eq!(stage.get(root).unwrap().container().child_count(), 1);
    }

    #[test]
    fn add_child_to_destroyed_parent_returns_none() {
        // Stale NodeIds (from destroy) carry an old generation and are
        // rejected by the slotmap guard inside add_child.
        let mut stage = Stage::new();
        let temp = stage.add_child(stage.root(), Container::default()).unwrap();
        assert!(stage.destroy(temp));
        assert!(stage.add_child(temp, Container::default()).is_none());
    }

    #[test]
    fn three_deep_hierarchy_traverses_pre_order() {
        let mut stage = Stage::new();
        let root = stage.root();
        let a = stage.add_child(root, Container::default()).unwrap();
        let b = stage.add_child(a, Container::default()).unwrap();
        let c = stage.add_child(b, Container::default()).unwrap();
        // Sibling of `a` to verify ordering.
        let d = stage.add_child(root, Container::default()).unwrap();

        let mut order = Vec::new();
        stage.traverse_pre_order(root, |id, _| order.push(id));
        assert_eq!(order, vec![root, a, b, c, d]);
    }

    #[test]
    fn detach_removes_from_parent_but_keeps_node() {
        let mut stage = Stage::new();
        let root = stage.root();
        let child = stage.add_child(root, Container::default()).unwrap();

        assert!(stage.detach(child));
        assert_eq!(stage.get(root).unwrap().container().child_count(), 0);
        assert_eq!(stage.get(child).unwrap().container().parent(), None);
        assert!(stage.get(child).is_some()); // still alive
    }

    #[test]
    fn destroy_cascades_to_descendants() {
        let mut stage = Stage::new();
        let root = stage.root();
        let a = stage.add_child(root, Container::default()).unwrap();
        let b = stage.add_child(a, Container::default()).unwrap();
        let _c = stage.add_child(b, Container::default()).unwrap();

        assert_eq!(stage.len(), 4);
        assert!(stage.destroy(a));
        // Root + nothing else.
        assert_eq!(stage.len(), 1);
        assert_eq!(stage.get(root).unwrap().container().child_count(), 0);
    }

    #[test]
    fn destroy_root_is_rejected() {
        let mut stage = Stage::new();
        assert!(!stage.destroy(stage.root()));
        assert_eq!(stage.len(), 1);
    }
}
