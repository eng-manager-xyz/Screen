//! `Mesh` — textured quad with 3D perspective rotation around the Y axis.
//!
//! M0.19 ships a fixed mesh shape (unit quad) with a perspective shader that
//! takes `rotation_y` (radians) and `perspective_strength` (focal-length-ish).
//! Generic per-Mesh custom WGSL is a v1+ chunk if/when we need it.

use crate::color::Color;
use crate::scene::container::Container;
use crate::texture::Texture;

/// Textured quad with 3D perspective rotation around the Y axis.
#[derive(Debug, Clone)]
pub struct Mesh {
    /// Transform / visibility container.
    pub container: Container,
    /// Texture sampled across the quad.
    pub texture: Texture,
    /// Multiplicative tint applied to the sampled texel.
    pub tint: Color,
    /// Rotation angle around the Y (vertical) axis, in radians.
    pub rotation_y: f32,
    /// Perspective strength: 0.0 = orthographic, 1.0 = strong foreshortening.
    pub perspective_strength: f32,
}

impl Mesh {
    /// Construct a Mesh from a texture, with no rotation and mild perspective.
    #[must_use]
    pub fn from_texture(texture: Texture) -> Self {
        Self {
            container: Container::default(),
            texture,
            tint: Color::WHITE,
            rotation_y: 0.0,
            perspective_strength: 0.4,
        }
    }

    /// Builder: set the Y-axis rotation.
    #[must_use]
    pub fn with_rotation_y(mut self, radians: f32) -> Self {
        self.rotation_y = radians;
        self
    }

    /// Builder: set the perspective strength (0.0–1.0 typical).
    #[must_use]
    pub fn with_perspective(mut self, strength: f32) -> Self {
        self.perspective_strength = strength;
        self
    }
}
