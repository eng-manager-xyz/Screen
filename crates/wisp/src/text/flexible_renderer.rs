//! Flexible text **renderer** — glyphon rasterization (M-TEXT.3 / AUT-77).
//!
//! Pairs with [`FlexibleTextEngine`](super::FlexibleTextEngine). The
//! engine produces [`FlexibleTextLayout`] shaped buffers; this
//! renderer hands those buffers to `glyphon` to rasterize into a
//! wgpu render target.
//!
//! # Lifecycle
//!
//! ```text
//! let engine   = FlexibleTextEngine::new();
//! let renderer = FlexibleTextRenderer::new(&app, format,
//!                                          engine.font_system_handle());
//! renderer.set_resolution(width_px, height_px);
//! let layout   = engine.layout_concrete(&text);
//! renderer.draw(&target_view, &[(&layout, position_ndc, color)]);
//! ```
//!
//! The engine and renderer share the same `FontSystem` through an
//! `Arc<Mutex<…>>` so layout-time metrics match the glyphs glyphon
//! eventually rasterizes.
//!
//! # Why this is a sibling of `Renderer`, not a method on it
//!
//! `glyphon::TextRenderer` keeps its own GPU pipeline + atlas separate
//! from wisp's existing batching pipelines. Putting it on
//! `crate::render::Renderer` would force every renderer instance to
//! pay the glyphon initialization cost; instead we keep
//! `FlexibleTextRenderer` as an opt-in resource owned by whichever
//! caller needs it (the editor for caption rendering, the export path
//! for burn-in text, M-TEXT.5's RT cache for filter/mask
//! composition).

use std::sync::{Arc, Mutex};

use cosmic_text::FontSystem;
use glam::Vec2;
use glyphon::{
    Cache, Color as GlyphonColor, Resolution, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer as GlyphonTextRenderer, Viewport,
};

use super::flexible::{FlexibleTextLayout, REFERENCE_PX};
use crate::application::Application;
use crate::color::Color;

/// Wgpu glyph rasterizer wrapping glyphon's pipeline.
///
/// Owns the glyphon-side GPU resources (atlas, viewport, renderer,
/// swash cache) plus a shared handle to the font system used by the
/// paired [`FlexibleTextEngine`](super::FlexibleTextEngine).
pub struct FlexibleTextRenderer {
    font_system: Arc<Mutex<FontSystem>>,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    viewport: Viewport,
    text_renderer: GlyphonTextRenderer,
    device: wgpu::Device,
    queue: wgpu::Queue,
    resolution: Resolution,
}

impl std::fmt::Debug for FlexibleTextRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlexibleTextRenderer")
            .field(
                "resolution",
                &(self.resolution.width, self.resolution.height),
            )
            .finish_non_exhaustive()
    }
}

impl FlexibleTextRenderer {
    /// Build the renderer for a target color format.
    ///
    /// `format` MUST match the format of every `TextureView` later
    /// passed to [`Self::draw`]. Multisample is fixed at 1× and depth
    /// stencil is unused — glyphon writes directly to the color
    /// attachment.
    #[must_use]
    pub fn new(
        app: &Application,
        format: wgpu::TextureFormat,
        font_system: Arc<Mutex<FontSystem>>,
    ) -> Self {
        let device = app.device().clone();
        let queue = app.queue().clone();
        let cache = Cache::new(&device);
        let mut atlas = TextAtlas::new(&device, &queue, &cache, format);
        let viewport = Viewport::new(&device, &cache);
        let text_renderer =
            GlyphonTextRenderer::new(&mut atlas, &device, wgpu::MultisampleState::default(), None);
        Self {
            font_system,
            swash_cache: SwashCache::new(),
            atlas,
            viewport,
            text_renderer,
            device,
            queue,
            resolution: Resolution {
                width: 1,
                height: 1,
            },
        }
    }

    /// Update the rasterization resolution. Call whenever the target
    /// view's dimensions change.
    pub fn set_resolution(&mut self, width_px: u32, height_px: u32) {
        self.resolution = Resolution {
            width: width_px,
            height: height_px,
        };
        self.viewport.update(&self.queue, self.resolution);
    }

    /// Stage `layouts` and emit draw calls into a fresh render pass on
    /// `target_view`. `clear` chooses whether to clear the target
    /// before drawing — set `false` when compositing on top of
    /// existing content.
    ///
    /// Each entry is `(layout, position_ndc, color)`:
    /// - `position_ndc` is the top-left of the layout box in NDC
    ///   (`[-1, +1]`). It's converted to glyphon pixel space using
    ///   the current resolution.
    /// - `color` provides the default per-glyph color.
    pub fn draw(
        &mut self,
        target_view: &wgpu::TextureView,
        layouts: &[(&FlexibleTextLayout, Vec2, Color)],
        clear: bool,
    ) {
        let resolution = self.resolution;
        let mut areas: Vec<TextArea<'_>> = Vec::with_capacity(layouts.len());
        for (layout, pos_ndc, color) in layouts {
            // NDC -> pixel: ndc.x in [-1, +1] -> px in [0, width].
            // y in NDC has +y up; glyphon expects +y down origin at top-left.
            #[expect(
                clippy::cast_precision_loss,
                reason = "resolution dimensions are < 2^23 in practice"
            )]
            let width_px = resolution.width as f32;
            #[expect(
                clippy::cast_precision_loss,
                reason = "resolution dimensions are < 2^23 in practice"
            )]
            let height_px = resolution.height as f32;
            let left_px = (pos_ndc.x * 0.5 + 0.5) * width_px;
            let top_px = (0.5 - pos_ndc.y * 0.5) * height_px;
            // Scale the layout (which was shaped at REFERENCE_PX basis)
            // to the current resolution.
            let scale = height_px / REFERENCE_PX;
            areas.push(TextArea {
                buffer: &layout.buffer,
                left: left_px,
                top: top_px,
                scale,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    #[expect(
                        clippy::cast_possible_wrap,
                        reason = "resolution dims < i32::MAX in practice"
                    )]
                    right: resolution.width as i32,
                    #[expect(
                        clippy::cast_possible_wrap,
                        reason = "resolution dims < i32::MAX in practice"
                    )]
                    bottom: resolution.height as i32,
                },
                default_color: wisp_color_to_glyphon(*color),
                custom_glyphs: &[],
            });
        }

        {
            let mut fs = self
                .font_system
                .lock()
                .expect("FlexibleTextRenderer font_system poisoned");
            self.text_renderer
                .prepare(
                    &self.device,
                    &self.queue,
                    &mut fs,
                    &mut self.atlas,
                    &self.viewport,
                    areas,
                    &mut self.swash_cache,
                )
                .expect("glyphon prepare");
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp::FlexibleTextRenderer encoder"),
            });
        {
            let load = if clear {
                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
            } else {
                wgpu::LoadOp::Load
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp::FlexibleTextRenderer pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut pass)
                .expect("glyphon render");
        }
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Trim atlas LRU entries that haven't been referenced since the
    /// last call. Call between frames to keep the atlas bounded.
    pub fn trim_atlas(&mut self) {
        self.atlas.trim();
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Color components are clamped to [0, 1] then * 255 — fits in u8"
)]
fn wisp_color_to_glyphon(color: Color) -> GlyphonColor {
    let red = (color.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let green = (color.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let blue = (color.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    let alpha = (color.a.clamp(0.0, 1.0) * 255.0).round() as u8;
    GlyphonColor::rgba(red, green, blue, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{AppConfig, Application};
    use crate::text::{FlexibleTextEngine, WispText};
    use crate::texture::render_texture::RenderTexture;

    fn boot() -> Application {
        pollster::block_on(Application::new(AppConfig::default())).expect("init")
    }

    #[test]
    fn renderer_constructs_against_default_app() {
        let app = boot();
        let engine = FlexibleTextEngine::new();
        let _r = FlexibleTextRenderer::new(
            &app,
            wgpu::TextureFormat::Rgba8Unorm,
            engine.font_system_handle(),
        );
    }

    #[test]
    fn empty_draw_does_not_panic() {
        let app = boot();
        let engine = FlexibleTextEngine::new();
        let mut r = FlexibleTextRenderer::new(
            &app,
            wgpu::TextureFormat::Rgba8Unorm,
            engine.font_system_handle(),
        );
        r.set_resolution(64, 64);
        let rt = RenderTexture::with_format(&app, 64, 64, wgpu::TextureFormat::Rgba8Unorm);
        r.draw(rt.view(), &[], true);
    }

    #[test]
    fn draw_hello_paints_some_non_zero_pixels() {
        let app = boot();
        let engine = FlexibleTextEngine::new();
        let mut r = FlexibleTextRenderer::new(
            &app,
            wgpu::TextureFormat::Rgba8Unorm,
            engine.font_system_handle(),
        );
        let rt = RenderTexture::with_format(&app, 256, 64, wgpu::TextureFormat::Rgba8Unorm);
        r.set_resolution(rt.width(), rt.height());
        let layout = engine.layout_concrete(&WispText::new("Hello"));
        r.draw(
            rt.view(),
            &[(&layout, Vec2::new(-0.5, 0.5), Color::WHITE)],
            true,
        );
        // Best-effort: read back, count non-zero alpha pixels. With
        // system fonts loaded we should see >0 pixels of glyph
        // coverage. If the host has no system fonts at all (extremely
        // unusual on a dev box / CI runner with `fontconfig`), this
        // assertion is the surface that will catch it.
        let bytes = rt.read_pixels(&app);
        let non_zero_alpha = bytes.chunks_exact(4).filter(|p| p[3] > 0).count();
        assert!(
            non_zero_alpha > 0,
            "expected glyphon to paint some non-zero alpha pixels"
        );
    }
}
