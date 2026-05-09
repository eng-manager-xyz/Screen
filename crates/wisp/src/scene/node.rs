//! Tagged union of scene-graph node variants.

use slotmap::new_key_type;

use crate::scene::container::Container;
use crate::scene::graphics::Graphics;
use crate::scene::mesh::Mesh;
use crate::scene::sprite::Sprite;
use crate::scene::text::Text;

new_key_type! {
    /// Stable handle for a scene-graph node owned by a [`crate::scene::Stage`].
    pub struct NodeId;
}

/// Scene-graph node — tagged union over the renderable types.
#[derive(Debug, Clone)]
pub enum Node {
    Container(Container),
    Sprite(Sprite),
    Graphics(Graphics),
    Text(Text),
    Mesh(Mesh),
}

impl Node {
    #[must_use]
    pub fn container(&self) -> &Container {
        match self {
            Self::Container(c) => c,
            Self::Sprite(s) => &s.container,
            Self::Graphics(g) => &g.container,
            Self::Text(t) => &t.container,
            Self::Mesh(m) => &m.container,
        }
    }

    pub fn container_mut(&mut self) -> &mut Container {
        match self {
            Self::Container(c) => c,
            Self::Sprite(s) => &mut s.container,
            Self::Graphics(g) => &mut g.container,
            Self::Text(t) => &mut t.container,
            Self::Mesh(m) => &mut m.container,
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

impl From<Graphics> for Node {
    fn from(g: Graphics) -> Self {
        Self::Graphics(g)
    }
}

impl From<Text> for Node {
    fn from(t: Text) -> Self {
        Self::Text(t)
    }
}

impl From<Mesh> for Node {
    fn from(m: Mesh) -> Self {
        Self::Mesh(m)
    }
}
