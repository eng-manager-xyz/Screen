//! Caption block — wrapped text on top of a rounded rect background
//! (M-TEXT.9 / AUT-83).
//!
//! Composition only; no new shaders. Layout works in NDC throughout:
//!
//! 1. Construct a [`CaptionBlock`] with `text`, `style`, `max_width_ndc`,
//!    and `padding_ndc`.
//! 2. Call [`CaptionBlock::layout`] with the
//!    [`crate::text::TextTexturePipeline`] to measure the text and
//!    produce a list of scene nodes:
//!    - a background `Graphics` rounded rect sized to the wrapped
//!      text + padding;
//!    - a `Sprite` carrying the rendered text texture, positioned
//!      inside the padding rect.
//! 3. Caller attaches both nodes to a `Container` (or directly to the
//!    stage); the container's transform handles position.
//!
//! Honors `WispTextStyle::align` (the text texture itself respects
//! the style's alignment) and `line_height` (cosmic-text handles
//! wrap + line metrics).

use std::sync::Arc;

use glam::Vec2;

use crate::application::Application;
use crate::color::Color;
use crate::math::Rect;
use crate::scene::{Graphics, Sprite};
use crate::text::texture::TextTexturePipeline;
use crate::text::{WispText, WispTextLayout, WispTextStyle};
use crate::texture::render_texture::RenderTexture;

/// Builder for a wrapped caption block with a rounded-rect background.
#[derive(Debug, Clone)]
pub struct CaptionBlock {
    /// The caption's content.
    pub text: WispText,
    /// Background fill (e.g. semi-transparent black for legibility).
    pub background_color: Color,
    /// Background corner radius in **local NDC**.
    pub corner_radius_ndc: f32,
    /// Padding inside the background, in local NDC (uniform on all
    /// four sides).
    pub padding_ndc: f32,
    /// Total width the background should occupy in **local NDC**. The
    /// text wraps to `width_ndc - 2 × padding_ndc`.
    pub width_ndc: f32,
    /// Texture resolution used to rasterize the text. Higher = sharper
    /// glyphs but more GPU memory + cache cost.
    pub text_resolution_px: (u32, u32),
}

impl Default for CaptionBlock {
    fn default() -> Self {
        Self {
            text: WispText::new(""),
            background_color: Color::rgba_u8(20, 22, 28, 220),
            corner_radius_ndc: 0.04,
            padding_ndc: 0.05,
            width_ndc: 0.8,
            text_resolution_px: (1280, 320),
        }
    }
}

/// Result of laying out a [`CaptionBlock`].
///
/// `background` and `text_sprite` are positioned in **local NDC**:
/// the background is at `Rect::new(0, 0, width_ndc, height_ndc)` and
/// the text sprite is inset by `padding_ndc`. Attach both to a
/// `Container` and use the container's transform to position the
/// caption in the scene.
pub struct CaptionLayout {
    /// Background pill / card.
    pub background: Graphics,
    /// Text rendered into a `RenderTexture`, wrapped in a `Sprite`.
    pub text_sprite: Sprite,
    /// Backing render-texture (kept alive for the duration of the
    /// caller's stage). The sprite holds a `Texture` clone from this.
    pub text_rt: Arc<RenderTexture>,
    /// Computed caption height in local NDC: `text_height + 2 × padding`.
    pub height_ndc: f32,
}

impl CaptionBlock {
    /// Construct a default block with text only — defaults match the
    /// in-app caption style (semi-transparent dark background, small
    /// rounded corners, ample padding).
    #[must_use]
    pub fn from_text(text: WispText) -> Self {
        Self {
            text,
            ..Self::default()
        }
    }

    /// Builder — replace the background color.
    #[must_use]
    pub fn with_background(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Builder — replace the corner radius.
    #[must_use]
    pub fn with_radius(mut self, radius_ndc: f32) -> Self {
        self.corner_radius_ndc = radius_ndc;
        self
    }

    /// Builder — replace the padding.
    #[must_use]
    pub fn with_padding(mut self, padding_ndc: f32) -> Self {
        self.padding_ndc = padding_ndc;
        self
    }

    /// Builder — replace the total width.
    #[must_use]
    pub fn with_width(mut self, width_ndc: f32) -> Self {
        self.width_ndc = width_ndc;
        self
    }

    /// Builder — replace the text content.
    #[must_use]
    pub fn with_text(mut self, text: WispText) -> Self {
        self.text = text;
        self
    }

    /// Builder — replace the style on the inner text.
    #[must_use]
    pub fn with_style(mut self, style: WispTextStyle) -> Self {
        self.text = self.text.with_style(style);
        self
    }

    /// Layout the caption. Measures the text inside `width_ndc -
    /// 2*padding`, sizes the background, and produces a
    /// [`CaptionLayout`] ready to be attached to a container.
    #[must_use]
    pub fn layout(&self, app: &Application, pipeline: &TextTexturePipeline) -> CaptionLayout {
        let text_max_w = (self.width_ndc - 2.0 * self.padding_ndc).max(0.01);

        // Force-wrap the text to the inner width so cosmic-text breaks
        // lines correctly. If the caller already set `with_wrap`, we
        // respect their value over ours.
        let mut text = self.text.clone();
        if text.max_width_ndc.is_none() {
            text = text.with_wrap(text_max_w);
        }

        let metrics = pipeline.engine().layout_concrete(&text).metrics();
        let text_h = metrics.total_height_ndc.max(text.style.size_ndc);

        let height_ndc = text_h + 2.0 * self.padding_ndc;

        let mut bg = Graphics::new();
        bg.fill(crate::Fill::Solid(self.background_color));
        bg.draw_rounded_rect(
            Rect::new(0.0, 0.0, self.width_ndc, height_ndc),
            self.corner_radius_ndc,
        );

        let (tex_w, tex_resolution_h) = self.text_resolution_px;
        let rt = pipeline.render(app, &text, tex_w, tex_resolution_h);
        let texture = rt.as_texture();

        let mut sprite = Sprite::from_texture(texture);
        // Sprite anchor at top-left, scale spans the inner padded area.
        sprite.with_anchor_set(Vec2::new(0.0, 0.0));
        sprite.container.transform.position =
            Vec2::new(self.padding_ndc, self.padding_ndc + text_h);
        // y-scale is negative because glyphon writes +y-down; sprite
        // samples +y-up. Standard text-texture convention.
        sprite.container.transform.scale = Vec2::new(text_max_w, -text_h);

        CaptionLayout {
            background: bg,
            text_sprite: sprite,
            text_rt: rt,
            height_ndc,
        }
    }
}

impl Sprite {
    /// Helper used by [`CaptionBlock`] to set the anchor mutably without
    /// needing a re-construction. Public so other compositions can
    /// reuse it; identical to [`Sprite::with_anchor`] but takes `&mut self`.
    pub fn with_anchor_set(&mut self, anchor: Vec2) -> &mut Self {
        self.anchor = anchor;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{AppConfig, Application};
    use crate::text::{WispText, WispTextStyle};
    use pollster::block_on;

    fn boot() -> (Application, TextTexturePipeline) {
        let app = block_on(Application::new(AppConfig::default())).expect("app");
        let pipeline = TextTexturePipeline::new(&app, wgpu::TextureFormat::Rgba8UnormSrgb);
        (app, pipeline)
    }

    #[test]
    fn caption_height_grows_with_more_lines() {
        let (app, pipeline) = boot();
        let style = WispTextStyle::default().with_size(0.06);

        let short = CaptionBlock::from_text(WispText::new("hello").with_style(style))
            .with_width(0.6)
            .with_padding(0.03);
        let long = CaptionBlock::from_text(
            WispText::new("hello world this is a much longer caption that must wrap")
                .with_style(style),
        )
        .with_width(0.6)
        .with_padding(0.03);

        let short_layout = short.layout(&app, &pipeline);
        let long_layout = long.layout(&app, &pipeline);
        assert!(
            long_layout.height_ndc > short_layout.height_ndc,
            "long caption height {} should exceed short {}",
            long_layout.height_ndc,
            short_layout.height_ndc,
        );
    }

    #[test]
    fn caption_height_includes_padding_on_both_sides() {
        let (app, pipeline) = boot();
        let style = WispTextStyle::default().with_size(0.08);
        let block = CaptionBlock::from_text(WispText::new("x").with_style(style))
            .with_width(0.5)
            .with_padding(0.1);

        let layout = block.layout(&app, &pipeline);
        // Expect height ≥ size_ndc + 2*padding = 0.08 + 0.20 = 0.28
        assert!(layout.height_ndc >= 0.28 - 1e-6);
    }

    #[test]
    fn builder_methods_chain_and_apply() {
        let style = WispTextStyle::default().with_size(0.05);
        let block = CaptionBlock::from_text(WispText::new("abc").with_style(style))
            .with_background(Color::RED)
            .with_radius(0.07)
            .with_padding(0.04)
            .with_width(0.9);
        assert_eq!(block.background_color, Color::RED);
        assert!((block.corner_radius_ndc - 0.07).abs() < 1e-6);
        assert!((block.padding_ndc - 0.04).abs() < 1e-6);
        assert!((block.width_ndc - 0.9).abs() < 1e-6);
    }

    #[test]
    fn explicit_wrap_overrides_block_inner_width() {
        let (app, pipeline) = boot();
        let style = WispTextStyle::default().with_size(0.06);
        // Caller-set wrap: 0.4 (narrower than the block's 0.7 - 2*0.05 = 0.6 inner).
        let text = WispText::new("hello world foo bar")
            .with_style(style)
            .with_wrap(0.4);
        let block = CaptionBlock::from_text(text)
            .with_width(0.7)
            .with_padding(0.05);
        let layout = block.layout(&app, &pipeline);
        // Just smoke-test that the layout produces a non-empty caption.
        assert!(layout.height_ndc > 0.0);
    }
}
