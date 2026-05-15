//! `Target` — the abstraction that binds an animation's output to
//! a mutable slot somewhere in the world. Built-in targets cover
//! every `wisp::scene::Container` property (alpha, translation,
//! rotation, scale).
//!
//! The trait is intentionally non-reflective: each `Property`
//! impl is a typed witness — there's no string lookup, no
//! `&'static str` field name, no `Any`. The dispatch cost per
//! property write is one match arm + a field assignment.

use glam::Vec2;
use wisp::scene::{NodeId, Stage, Transform};

/// A typed handle to a mutable slot in `Stage`. Implementations
/// can read the current value and write a new one.
pub trait Target<V> {
    /// Read the current value (used by springs / FLIP captures).
    fn read(&self, stage: &Stage) -> V;
    /// Write a new value.
    fn write(&self, stage: &mut Stage, value: V);
}

/// Witness type for one of `Container`'s properties. Concrete
/// constructors live below as `Property::*`.
///
/// `Ord` / `PartialOrd` are derived for [`BatchDriver`](crate::BatchDriver)'s
/// sort-based dedup; the variant declaration order is the sort
/// order and is part of the public contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Property {
    /// `Container::alpha` (`f32`).
    Alpha,
    /// `Container::transform.position` (`Vec2`).
    Translation,
    /// `Container::transform.rotation` (`f32`, radians).
    Rotation,
    /// `Container::transform.scale` (`Vec2`).
    Scale,
}

/// Binds a [`Property`] to a specific scene-graph node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeProperty {
    /// Target node.
    pub node: NodeId,
    /// Which property of the node's container.
    pub prop: Property,
}

impl NodeProperty {
    /// Construct a target for the alpha property.
    #[must_use]
    pub const fn alpha(node: NodeId) -> Self {
        Self {
            node,
            prop: Property::Alpha,
        }
    }

    /// Construct a target for the translation property.
    #[must_use]
    pub const fn translation(node: NodeId) -> Self {
        Self {
            node,
            prop: Property::Translation,
        }
    }

    /// Construct a target for the rotation property.
    #[must_use]
    pub const fn rotation(node: NodeId) -> Self {
        Self {
            node,
            prop: Property::Rotation,
        }
    }

    /// Construct a target for the scale property.
    #[must_use]
    pub const fn scale(node: NodeId) -> Self {
        Self {
            node,
            prop: Property::Scale,
        }
    }
}

impl Target<f32> for NodeProperty {
    fn read(&self, stage: &Stage) -> f32 {
        let Some(node) = stage.get(self.node) else {
            return 0.0;
        };
        match self.prop {
            Property::Alpha => node.container().alpha,
            Property::Rotation => node.container().transform.rotation,
            _ => 0.0,
        }
    }

    fn write(&self, stage: &mut Stage, value: f32) {
        if let Some(node) = stage.get_mut(self.node) {
            let c = node.container_mut();
            match self.prop {
                Property::Alpha => c.alpha = value,
                Property::Rotation => c.transform.rotation = value,
                _ => {}
            }
        }
    }
}

impl Target<Vec2> for NodeProperty {
    fn read(&self, stage: &Stage) -> Vec2 {
        let Some(node) = stage.get(self.node) else {
            return Vec2::ZERO;
        };
        match self.prop {
            Property::Translation => node.container().transform.position,
            Property::Scale => node.container().transform.scale,
            _ => Vec2::ZERO,
        }
    }

    fn write(&self, stage: &mut Stage, value: Vec2) {
        if let Some(node) = stage.get_mut(self.node) {
            let c = node.container_mut();
            match self.prop {
                Property::Translation => c.transform.position = value,
                Property::Scale => c.transform.scale = value,
                _ => {}
            }
        }
    }
}

impl Target<Transform> for NodeProperty {
    fn read(&self, stage: &Stage) -> Transform {
        stage
            .get(self.node)
            .map_or(Transform::IDENTITY, |n| n.container().transform)
    }

    fn write(&self, stage: &mut Stage, value: Transform) {
        if let Some(node) = stage.get_mut(self.node) {
            node.container_mut().transform = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wisp::Color;
    use wisp::scene::{Graphics, Stage};

    #[test]
    fn write_alpha_round_trips() {
        let mut stage = Stage::new();
        let root = stage.root();
        let g = Graphics::new();
        let node = stage.add_child(root, g).expect("add child");
        let target = NodeProperty::alpha(node);
        <NodeProperty as Target<f32>>::write(&target, &mut stage, 0.42);
        let read: f32 = <NodeProperty as Target<f32>>::read(&target, &stage);
        assert!((read - 0.42).abs() < f32::EPSILON);
    }

    #[test]
    fn write_translation_round_trips() {
        let mut stage = Stage::new();
        let root = stage.root();
        let g = Graphics::new();
        let node = stage.add_child(root, g).expect("add child");
        let target = NodeProperty::translation(node);
        <NodeProperty as Target<Vec2>>::write(&target, &mut stage, Vec2::new(10.0, 20.0));
        let read: Vec2 = <NodeProperty as Target<Vec2>>::read(&target, &stage);
        assert!((read.x - 10.0).abs() < f32::EPSILON);
        assert!((read.y - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn write_to_stale_node_is_noop() {
        let mut stage = Stage::new();
        let root = stage.root();
        let g = Graphics::new();
        let node = stage.add_child(root, g).expect("add child");
        stage.destroy(node);
        let target = NodeProperty::alpha(node);
        // Must not panic.
        <NodeProperty as Target<f32>>::write(&target, &mut stage, 0.5);
        // Read on a stale node yields the f32 default.
        let _read: f32 = <NodeProperty as Target<f32>>::read(&target, &stage);
    }

    #[test]
    fn rotation_writes_to_transform_field() {
        let mut stage = Stage::new();
        let root = stage.root();
        let g = Graphics::new();
        let node = stage.add_child(root, g).expect("add child");
        let target = NodeProperty::rotation(node);
        <NodeProperty as Target<f32>>::write(&target, &mut stage, 1.5);
        let rotation: f32 = <NodeProperty as Target<f32>>::read(&target, &stage);
        assert!((rotation - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn write_color_via_node_property_does_nothing() {
        // Color isn't a Container property — Target<Color> is not
        // implemented for NodeProperty. This is enforced by the
        // type system (this test exists to document intent).
        let _ = Color::WHITE;
    }
}
