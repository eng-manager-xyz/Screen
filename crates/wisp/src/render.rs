//! Renderer — pipeline cache, draw-call batcher, filter pass orchestrator.
//!
//! Evolution:
//!   M0.5: hardcoded triangle.
//!   M0.6: textured-quad path (`render_quad`).
//!   M0.9: sprite batcher with scene-graph traversal (`render_stage`).
//!   M0.16: filter pass orchestrator.

pub mod batcher;
pub mod pass;
pub mod pipeline;

mod graphics_pipeline;
mod quad_pipeline;
mod sprite_pipeline;
mod triangle_pipeline;

use graphics_pipeline::GraphicsPipeline;
use quad_pipeline::QuadPipeline;
use sprite_pipeline::SpritePipeline;
use triangle_pipeline::TrianglePipeline;

use crate::application::Application;
use crate::color::Color;
use crate::error::Error;
use crate::scene::Stage;
use crate::texture::Texture;

/// Frame statistics returned by [`Renderer::render_stage`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RenderStats {
    /// Number of `draw` calls submitted to the GPU.
    pub draw_calls: u32,
    /// Total sprites rendered across all batches.
    pub sprites_drawn: u32,
    /// Total graphics primitives rendered.
    pub graphics_drawn: u32,
}

/// 2D renderer.
///
/// Owns the GPU pipelines used to draw scenes onto a [`wgpu::TextureView`].
/// Construct one per output format (surface or `RenderTexture`).
pub struct Renderer {
    triangle: TrianglePipeline,
    quad: QuadPipeline,
    sprite: SpritePipeline,
    graphics: GraphicsPipeline,
}

impl Renderer {
    /// Construct a renderer that targets the given color format.
    ///
    /// # Errors
    ///
    /// Currently infallible; reserved for future pipeline-creation failures.
    pub fn new(app: &Application, output_format: wgpu::TextureFormat) -> Result<Self, Error> {
        let triangle = TrianglePipeline::new(app, output_format);
        let quad = QuadPipeline::new(app, output_format);
        let sprite = SpritePipeline::new(app, output_format);
        let graphics = GraphicsPipeline::new(app, output_format);
        Ok(Self {
            triangle,
            quad,
            sprite,
            graphics,
        })
    }

    /// Clear the target with `clear`, then draw the M0.5 hardcoded triangle.
    pub fn render(&self, app: &Application, view: &wgpu::TextureView, clear: Color) {
        Self::with_clearing_pass(app, view, clear, |pass| self.triangle.draw(pass));
    }

    /// Clear the target with `clear`, then draw a single textured quad.
    pub fn render_quad(
        &self,
        app: &Application,
        view: &wgpu::TextureView,
        clear: Color,
        texture: &Texture,
        model: glam::Mat4,
        tint: Color,
    ) {
        Self::with_clearing_pass(app, view, clear, |pass| {
            self.quad.draw(app, pass, texture, model, tint);
        });
    }

    /// Clear the target, traverse `stage`, and draw every visible sprite,
    /// batching sprites with the same texture and blend mode.
    ///
    /// Returns [`RenderStats`] with the resulting draw-call and sprite counts.
    #[must_use]
    pub fn render_stage(
        &self,
        app: &Application,
        view: &wgpu::TextureView,
        clear: Color,
        stage: &Stage,
    ) -> RenderStats {
        let mut stats = RenderStats::default();
        Self::with_clearing_pass(app, view, clear, |pass| {
            let (sprite_calls, sprites_drawn) = self.sprite.draw_stage(app, pass, stage);
            let (graphics_calls, graphics_drawn) = self.graphics.draw_stage(app, pass, stage);
            stats.draw_calls = sprite_calls + graphics_calls;
            stats.sprites_drawn = sprites_drawn;
            stats.graphics_drawn = graphics_drawn;
        });
        stats
    }

    fn with_clearing_pass(
        app: &Application,
        view: &wgpu::TextureView,
        clear: Color,
        draw: impl FnOnce(&mut wgpu::RenderPass<'_>),
    ) {
        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp::Renderer encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp::Renderer main pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(clear.r),
                            g: f64::from(clear.g),
                            b: f64::from(clear.b),
                            a: f64::from(clear.a),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            draw(&mut pass);
        }
        app.queue().submit(std::iter::once(encoder.finish()));
    }
}
