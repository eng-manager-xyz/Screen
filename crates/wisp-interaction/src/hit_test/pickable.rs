//! `Pickable` side-table — nodes opt in to hit-testing here, NOT on
//! `wisp::Node` itself.
//!
//! Why a side-table:
//!
//! Adding a `pickable: bool` field to `wisp::Node` would publish the
//! interaction model permanently via the `screen-wisp` crate on
//! crates.io. Downstream users who only want a renderer would still
//! see (and have to think about) hit-testing concepts. Storing the
//! pickable flag in a separate `HashMap<NodeId, Pickable>` keeps
//! `wisp` interaction-free and lets `wisp-interaction` own the entire
//! input vocabulary on its own crate boundary.
//!
//! Lookups are one `HashMap` probe per node during hit-test backend
//! construction; rebuild only when the scene graph changes, not every
//! frame.

use std::collections::HashMap;

use wisp::scene::NodeId;

use crate::hit_test::shape::HitShape;

/// Per-node interaction descriptor. Stored in [`PickableMap`].
#[derive(Debug, Clone, PartialEq)]
pub struct Pickable {
    /// Local-space hit geometry.
    pub shape: HitShape,
    /// Temporary disable without removing the entry — set `false` to
    /// have the backend skip this node without losing the shape.
    pub enabled: bool,
}

impl Pickable {
    /// Convenience: a pickable node with the given shape, enabled.
    #[must_use]
    pub fn new(shape: HitShape) -> Self {
        Self {
            shape,
            enabled: true,
        }
    }

    /// Disabled variant — present in the map but skipped during
    /// hit-testing. Useful for "ghosted" UI states.
    #[must_use]
    pub fn disabled(shape: HitShape) -> Self {
        Self {
            shape,
            enabled: false,
        }
    }
}

/// Side-table keyed by `NodeId`. Hosts insert one entry per node they
/// want to receive pointer events on.
///
/// Entries for destroyed nodes are NOT auto-cleaned (the map doesn't
/// observe the `Stage`); call [`PickableMap::retain_nodes`] after a
/// destroy pass to keep the map in sync.
#[derive(Debug, Default, Clone)]
pub struct PickableMap {
    entries: HashMap<NodeId, Pickable>,
}

impl PickableMap {
    /// Empty map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of registered pickable nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` if no nodes registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Mark `node` as pickable with the given local-space shape.
    /// Replaces any prior entry.
    pub fn insert(&mut self, node: NodeId, pickable: Pickable) {
        self.entries.insert(node, pickable);
    }

    /// Shorthand: insert a `Pickable::new(shape)`.
    pub fn insert_shape(&mut self, node: NodeId, shape: HitShape) {
        self.insert(node, Pickable::new(shape));
    }

    /// Remove and return the entry for `node`.
    pub fn remove(&mut self, node: NodeId) -> Option<Pickable> {
        self.entries.remove(&node)
    }

    /// Lookup helper.
    #[must_use]
    pub fn get(&self, node: NodeId) -> Option<&Pickable> {
        self.entries.get(&node)
    }

    /// Borrow every entry. Used by the backend's build pass.
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &Pickable)> + '_ {
        self.entries.iter().map(|(k, v)| (*k, v))
    }

    /// Drop entries whose node no longer exists in the predicate.
    /// Typically called as `map.retain_nodes(|id| stage.get(id).is_some())`
    /// after a batch destroy.
    pub fn retain_nodes(&mut self, mut keep: impl FnMut(NodeId) -> bool) {
        self.entries.retain(|id, _| keep(*id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;
    use wisp::math::Rect;
    use wisp::scene::{Container, Stage};

    #[test]
    fn insert_get_remove_round_trip() {
        let mut stage = Stage::new();
        let root = stage.root();
        let n = stage.add_child(root, Container::new()).expect("add_child");

        let mut map = PickableMap::new();
        assert!(map.is_empty());
        map.insert_shape(n, HitShape::Rect(Rect::new(0.0, 0.0, 10.0, 10.0)));
        assert_eq!(map.len(), 1);
        assert!(map.get(n).is_some());
        let removed = map.remove(n).unwrap();
        assert_eq!(
            removed.shape,
            HitShape::Rect(Rect::new(0.0, 0.0, 10.0, 10.0))
        );
        assert!(map.is_empty());
    }

    #[test]
    fn retain_nodes_drops_orphans() {
        let mut stage = Stage::new();
        let root = stage.root();
        let live = stage.add_child(root, Container::new()).expect("live");
        let dead = stage.add_child(root, Container::new()).expect("dead");
        let mut map = PickableMap::new();
        map.insert_shape(
            live,
            HitShape::Circle {
                center: Vec2::ZERO,
                radius: 1.0,
            },
        );
        map.insert_shape(
            dead,
            HitShape::Circle {
                center: Vec2::ZERO,
                radius: 1.0,
            },
        );
        assert_eq!(map.len(), 2);

        // Simulate destroying `dead` without actually removing from the
        // scene — the retain_nodes contract is the caller's responsibility.
        map.retain_nodes(|id| id == live);
        assert_eq!(map.len(), 1);
        assert!(map.get(live).is_some());
        assert!(map.get(dead).is_none());
    }

    #[test]
    fn disabled_pickable_keeps_shape() {
        let mut stage = Stage::new();
        let root = stage.root();
        let n = stage.add_child(root, Container::new()).expect("n");
        let mut map = PickableMap::new();
        map.insert(
            n,
            Pickable::disabled(HitShape::Rect(Rect::new(0.0, 0.0, 1.0, 1.0))),
        );
        let entry = map.get(n).unwrap();
        assert!(!entry.enabled);
        assert_eq!(entry.shape, HitShape::Rect(Rect::new(0.0, 0.0, 1.0, 1.0)));
    }
}
