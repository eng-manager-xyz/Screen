//! `Sprite` — textured quad with anchor and tint.
//!
//! Composed over [`Container`] (no inheritance). Sprites participate in the
//! scene graph through their inner `container` field; the renderer's batcher
//! groups sprites that share a texture and blend mode.

use glam::Vec2;

use crate::color::Color;
use crate::scene::container::Container;
use crate::texture::Texture;

/// Textured quad node. Local rect is `[0, 1]²`; `anchor` shifts that local
/// origin (`0,0` = top-left, `1,1` = bottom-right). The sprite's
/// `Container::transform` is then applied.
#[derive(Debug, Clone)]
pub struct Sprite {
    /// Scene-graph state (transform, alpha, visible, blend mode, parent/children).
    pub container: Container,
    /// GPU texture sampled in the fragment shader.
    pub texture: Texture,
    /// Normalized anchor in `[0, 1]²`. `0,0` = top-left, `0.5, 0.5` = center.
    pub anchor: Vec2,
    /// Multiplied with the sampled texel.
    pub tint: Color,
}

impl Sprite {
    /// Construct a sprite from a texture with default container, top-left
    /// anchor, and white tint.
    #[must_use]
    pub fn from_texture(texture: Texture) -> Self {
        Self {
            container: Container::default(),
            texture,
            anchor: Vec2::ZERO,
            tint: Color::WHITE,
        }
    }

    /// Builder: set the anchor.
    #[must_use]
    pub fn with_anchor(mut self, anchor: Vec2) -> Self {
        self.anchor = anchor;
        self
    }

    /// Builder: set the tint.
    #[must_use]
    pub fn with_tint(mut self, tint: Color) -> Self {
        self.tint = tint;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{AppConfig, Application};

    fn boot_texture() -> Texture {
        let app = pollster::block_on(Application::new(AppConfig::default())).expect("init");
        let bytes = vec![255u8; 4 * 4 * 4];
        Texture::from_rgba(&app, 4, 4, &bytes)
    }

    #[test]
    fn from_texture_defaults() {
        let texture = boot_texture();
        let sprite = Sprite::from_texture(texture);
        assert_eq!(sprite.anchor, Vec2::ZERO);
        assert_eq!(sprite.tint, Color::WHITE);
        assert_eq!(
            sprite.container.transform,
            crate::scene::Transform::IDENTITY
        );
    }

    #[test]
    fn builder_with_anchor() {
        let texture = boot_texture();
        let sprite = Sprite::from_texture(texture).with_anchor(Vec2::splat(0.5));
        assert_eq!(sprite.anchor, Vec2::splat(0.5));
    }

    #[test]
    fn builder_with_tint() {
        let texture = boot_texture();
        let sprite = Sprite::from_texture(texture).with_tint(Color::RED);
        assert_eq!(sprite.tint, Color::RED);
    }
}
