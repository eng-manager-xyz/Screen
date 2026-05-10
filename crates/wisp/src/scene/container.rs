//! `Container` — scene-graph node holding children, transform, alpha, visible, blend mode.
//!
//! Filters (`Vec<Box<dyn Filter>>`) are declared in the design but land
//! in M0.16+ when the per-container filter pipeline exists. Mask
//! clipping ([`Container::clip`]) ships in M-MASK.1.

use crate::blend::BlendMode;
use crate::scene::Transform;
use crate::scene::clip::MaskShape;
use crate::scene::node::NodeId;

/// Scene-graph container.
///
/// Holds child node IDs and the per-node transform/alpha/visibility/blend
/// state. `parent` is `None` for orphans and for the stage root.
#[derive(Debug, Clone)]
pub struct Container {
    /// Local affine transform.
    pub transform: Transform,
    /// Alpha multiplier in `[0.0, 1.0]`. Multiplied with parent alpha at render.
    pub alpha: f32,
    /// Skip rendering when `false`. Children are also skipped.
    pub visible: bool,
    /// Blend mode used when compositing this node's output over its target.
    pub blend_mode: BlendMode,
    /// Optional mask region (M-MASK.1). When `Some`, the container's
    /// rendered subtree is clipped to the shape: pixels outside the
    /// mask have their alpha zeroed before being composited onto the
    /// parent. The renderer routes clipped containers through an
    /// offscreen pass — see `crate::render::clip`.
    pub clip: Option<MaskShape>,
    pub(crate) children: Vec<NodeId>,
    pub(crate) parent: Option<NodeId>,
}

impl Container {
    /// Construct a default container — identity transform, full alpha, visible, normal blend.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Iterate child node IDs in insertion order.
    #[must_use = "iterator must be consumed"]
    pub fn children(&self) -> impl DoubleEndedIterator<Item = NodeId> + ExactSizeIterator + '_ {
        self.children.iter().copied()
    }

    /// Number of direct children.
    #[must_use]
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Parent node ID, if any.
    #[must_use]
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }
}

impl Default for Container {
    fn default() -> Self {
        Self {
            transform: Transform::IDENTITY,
            alpha: 1.0,
            visible: true,
            blend_mode: BlendMode::Normal,
            clip: None,
            children: Vec::new(),
            parent: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let c = Container::default();
        assert_eq!(c.transform, Transform::IDENTITY);
        assert!((c.alpha - 1.0).abs() < f32::EPSILON);
        assert!(c.visible);
        assert_eq!(c.blend_mode, BlendMode::Normal);
        assert_eq!(c.child_count(), 0);
        assert_eq!(c.parent(), None);
    }

    #[test]
    fn new_equals_default() {
        let a = Container::new();
        let b = Container::default();
        assert_eq!(a.transform, b.transform);
        assert!((a.alpha - b.alpha).abs() < f32::EPSILON);
        assert_eq!(a.visible, b.visible);
        assert_eq!(a.blend_mode, b.blend_mode);
    }
}
