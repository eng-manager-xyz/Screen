//! Tagged union of scene-graph node variants.
//!
//! As new node types land they're added as variants. M0.8 starts with only
//! `Container`; M0.9 adds `Sprite`. M0.12 adds `Graphics`, M0.15 adds `Text`,
//! M0.19 adds `Mesh`.

use slotmap::new_key_type;

use crate::scene::container::Container;
use crate::scene::sprite::Sprite;

new_key_type! {
    /// Stable handle for a scene-graph node owned by a [`crate::scene::Stage`].
    pub struct NodeId;
}

/// Scene-graph node — tagged union over the renderable types.
#[derive(Debug, Clone)]
pub enum Node {
    /// Plain container — groups children with a transform and visibility state.
    Container(Container),
    /// Textured quad sprite.
    Sprite(Sprite),
}

impl Node {
    /// Borrow the container aspect of this node.
    ///
    /// Every node type carries a `Container` for its scene-graph state; this
    /// is the uniform accessor.
    #[must_use]
    pub fn container(&self) -> &Container {
        match self {
            Self::Container(c) => c,
            Self::Sprite(s) => &s.container,
        }
    }

    /// Mutably borrow the container aspect of this node.
    pub fn container_mut(&mut self) -> &mut Container {
        match self {
            Self::Container(c) => c,
            Self::Sprite(s) => &mut s.container,
        }
    }
}

impl From<Container> for Node {
    fn from(c: Container) -> Self {
        Self::Container(c)
    }
}

impl From<Sprite> for Node {
    fn from(s: Sprite) -> Self {
        Self::Sprite(s)
    }
}
