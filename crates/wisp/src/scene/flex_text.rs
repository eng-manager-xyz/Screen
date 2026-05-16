//! `FlexText` — late-pass textured-quad scene node.
//!
//! Structurally identical to [`Sprite`](crate::scene::Sprite): a
//! textured quad with anchor + tint sampled from its container's
//! transform. The difference is *when* the renderer paints it.
//!
//! Wisp's render-bucket order is **sprite → graphics → text**.
//! Charts emit their bars / gridlines / axis lines as
//! [`Graphics`](crate::scene::Graphics) primitives, so anything in
//! the sprite pass renders *under* them. That's correct for
//! background imagery but wrong for axis tick labels, legends, KPI
//! big numbers — any text that has to read on top of the chart.
//!
//! `FlexText` participates in a **fourth render pass** that runs
//! after [`Text`](crate::scene::Text) (the bitmap-font pipeline),
//! using the same instanced textured-quad shader as
//! [`Sprite`](crate::scene::Sprite). The intended producer is the
//! flexible-text path: rasterise via
//! [`crate::text::TextTexturePipeline::render`], call
//! [`crate::texture::render_texture::RenderTexture::as_texture`],
//! drop the resulting [`crate::texture::Texture`] into a `FlexText`.
//!
//! `FlexText` is *purposefully* not auto-named "Label" or "Overlay"
//! — call it what it is. Charts can compose it for axis labels,
//! callers can compose it for any "render this textured quad on top
//! of graphics" use case.

use glam::Vec2;

use crate::color::Color;
use crate::scene::container::Container;
use crate::texture::Texture;

/// Late-pass textured quad. Same data shape as
/// [`Sprite`](crate::scene::Sprite); the render pipeline puts it in
/// the fourth render pass so it paints after every
/// [`Graphics`](crate::scene::Graphics) primitive.
#[derive(Debug, Clone)]
pub struct FlexText {
    /// Scene-graph state (transform, alpha, visible, blend mode, parent / children).
    pub container: Container,
    /// GPU texture sampled in the fragment shader. Usually the
    /// output of
    /// [`crate::text::TextTexturePipeline::render`] +
    /// [`crate::texture::render_texture::RenderTexture::as_texture`].
    pub texture: Texture,
    /// Normalized anchor in `[0, 1]²`. `0,0` = top-left, `0.5, 0.5`
    /// = centre.
    pub anchor: Vec2,
    /// Multiplied with the sampled texel. Defaults to white so the
    /// glyph colour baked into the source texture wins.
    pub tint: Color,
}

impl FlexText {
    /// Construct a `FlexText` from `texture` with default container,
    /// top-left anchor, and white tint.
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
        let node = FlexText::from_texture(texture);
        assert_eq!(node.anchor, Vec2::ZERO);
        assert_eq!(node.tint, Color::WHITE);
    }

    #[test]
    fn builder_with_anchor() {
        let texture = boot_texture();
        let node = FlexText::from_texture(texture).with_anchor(Vec2::splat(0.5));
        assert_eq!(node.anchor, Vec2::splat(0.5));
    }

    #[test]
    fn builder_with_tint() {
        let texture = boot_texture();
        let node = FlexText::from_texture(texture).with_tint(Color::RED);
        assert_eq!(node.tint, Color::RED);
    }
}
