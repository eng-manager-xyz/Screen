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
mod mesh_pipeline;
mod quad_pipeline;
mod sprite_pipeline;
mod text_pipeline;
mod triangle_pipeline;

use graphics_pipeline::GraphicsPipeline;
use mesh_pipeline::MeshPipeline;
use quad_pipeline::QuadPipeline;
use sprite_pipeline::SpritePipeline;
use text_pipeline::TextPipeline;
use triangle_pipeline::TrianglePipeline;

use crate::application::Application;
use crate::color::Color;
use crate::error::Error;
use crate::filter::{Filter, FilterContext};
use crate::scene::Stage;
use crate::texture::Texture;
use crate::texture::render_texture::RenderTexture;

/// Frame statistics returned by [`Renderer::render_stage`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RenderStats {
    /// Number of `draw` calls submitted to the GPU.
    pub draw_calls: u32,
    /// Total sprites rendered across all batches.
    pub sprites_drawn: u32,
    /// Total graphics primitives rendered.
    pub graphics_drawn: u32,
    /// Total text glyphs rendered.
    pub glyphs_drawn: u32,
    /// Total meshes rendered.
    pub meshes_drawn: u32,
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
    text: TextPipeline,
    mesh: MeshPipeline,
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
        let text = TextPipeline::new(app, output_format);
        let mesh = MeshPipeline::new(app, output_format);
        Ok(Self {
            triangle,
            quad,
            sprite,
            graphics,
            text,
            mesh,
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
            let (text_calls, glyphs_drawn) = self.text.draw_stage(app, pass, stage);
            let (mesh_calls, meshes_drawn) = self.mesh.draw_stage(app, pass, stage);
            stats.draw_calls = sprite_calls + graphics_calls + text_calls + mesh_calls;
            stats.sprites_drawn = sprites_drawn;
            stats.graphics_drawn = graphics_drawn;
            stats.glyphs_drawn = glyphs_drawn;
            stats.meshes_drawn = meshes_drawn;
        });
        stats
    }

    /// Apply `filter` to `input`, writing the final result to `output`.
    ///
    /// Multi-pass filters allocate a scratch `RenderTexture` (same size +
    /// format as `output`) and ping-pong between it and `output`.
    pub fn apply_filter(
        &self,
        app: &Application,
        filter: &dyn Filter,
        input: &RenderTexture,
        output: &RenderTexture,
    ) {
        let n = filter.passes();
        debug_assert!(n >= 1, "Filter::passes() must be >= 1");

        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp::apply_filter encoder"),
            });

        if n == 1 {
            let mut ctx = FilterContext {
                app,
                encoder: &mut encoder,
            };
            filter.render_pass(&mut ctx, input, output, 0);
        } else {
            let scratch =
                RenderTexture::with_format(app, input.width(), input.height(), output.format());
            let mut ctx = FilterContext {
                app,
                encoder: &mut encoder,
            };
            // pass 0: input → scratch
            filter.render_pass(&mut ctx, input, &scratch, 0);
            // intermediate passes (rare for our M0.16 filters): scratch → scratch.
            // Ping-pong needs two scratches; for n=2 (BlurFilter) we don't hit this.
            for p in 1..(n - 1) {
                filter.render_pass(&mut ctx, &scratch, &scratch, p);
            }
            // last pass: scratch → output
            filter.render_pass(&mut ctx, &scratch, output, n - 1);
        }

        app.queue().submit(std::iter::once(encoder.finish()));
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
