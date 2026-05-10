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

mod advanced_blend;
mod blend_pipeline;
mod blit;
mod clip;
mod graphics_pipeline;
mod mask_texture;
mod mesh_pipeline;
mod path_clip;
mod path_mask_texture;
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
use crate::scene::clip::MaskShape;
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
    advanced_blend: advanced_blend::AdvancedBlendPipelines,
    blit: blit::BlitPipeline,
    clip: clip::ClipPipeline,
    path_clip: path_clip::PathClipPipeline,
    mask_texture: mask_texture::MaskTexturePipeline,
    path_mask_texture: path_mask_texture::PathMaskTexturePipeline,
    output_format: wgpu::TextureFormat,
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
        let advanced_blend = advanced_blend::AdvancedBlendPipelines::new(app, output_format);
        let blit_pipeline = blit::BlitPipeline::new(app, output_format);
        let clip_pipeline = clip::ClipPipeline::new(app, output_format);
        let path_clip_pipeline = path_clip::PathClipPipeline::new(app, output_format);
        let mask_texture_pipeline = mask_texture::MaskTexturePipeline::new(app, output_format);
        let path_mask_texture_pipeline =
            path_mask_texture::PathMaskTexturePipeline::new(app, output_format);
        Ok(Self {
            triangle,
            quad,
            sprite,
            graphics,
            text,
            mesh,
            advanced_blend,
            blit: blit_pipeline,
            clip: clip_pipeline,
            path_clip: path_clip_pipeline,
            mask_texture: mask_texture_pipeline,
            path_mask_texture: path_mask_texture_pipeline,
            output_format,
        })
    }

    /// Compose two render-textures via an advanced (Tier C) blend mode.
    ///
    /// `backdrop` is the previously-rendered destination, `foreground`
    /// is this node's contribution rendered into its own RT, and the
    /// composite lands in `output`. All three must share dimensions
    /// and the format the renderer was constructed against.
    ///
    /// # Panics
    ///
    /// Panics if `mode` is a *standard* (GPU-native) blend mode — those
    /// don't have a per-mode pipeline registered. Use the standard
    /// pipelines (via `render_stage` + `Container::blend_mode`) for
    /// those.
    pub fn apply_advanced_blend(
        &self,
        app: &Application,
        mode: crate::blend::BlendMode,
        backdrop: &RenderTexture,
        foreground: &RenderTexture,
        output: &RenderTexture,
    ) {
        self.advanced_blend
            .apply(app, mode, backdrop, foreground, output);
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

    /// Clear the target, traverse `stage`, draw every visible node.
    ///
    /// Two paths internally:
    ///
    /// - **Fast path** (no advanced blend modes AND no clipped
    ///   containers): one render pass directly into `view`, batching by
    ///   pipeline + blend mode.
    /// - **Slow path** (any node uses an advanced blend mode OR has a
    ///   clip mask set): allocate internal `RenderTexture`s at
    ///   [`app.width()`/`app.height()`](Application::width), render the
    ///   scene minus the affected subtrees, then for each affected node
    ///   render its subtree into a foreground RT, optionally
    ///   [`apply_clip`](Self::apply_clip) it, and composite onto the
    ///   in-progress destination (advanced blend modes use
    ///   [`apply_advanced_blend`](Self::apply_advanced_blend); clipped
    ///   containers use source-over via the blit pipeline). Final blit
    ///   to `view`.
    ///
    /// Slow-path RT dimensions track `Application::width()` /
    /// `Application::height()`;
    /// for views whose dims diverge from the app config, use a matching
    /// `AppConfig` or pre-render into a fixed-size `RenderTexture`.
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
        let dispatched = collect_dispatched_nodes(stage);
        if dispatched.is_empty() {
            return self.render_stage_fast(app, view, clear, stage);
        }
        self.render_stage_with_advanced_dispatch(app, view, clear, stage, &dispatched)
    }

    /// Apply a [`MaskShape`] clip to `foreground`, writing the masked
    /// result to `output`. Pixels outside the mask have their alpha
    /// zeroed.
    ///
    /// Auto-dispatched by `render_stage` when a container's
    /// [`Container::clip`](crate::scene::Container) is set; this method
    /// is also exposed for callers who pre-render a foreground RT
    /// manually and want to mask it without going through the full
    /// scene-graph path.
    pub fn apply_clip(
        &self,
        app: &Application,
        shape: crate::scene::clip::MaskShape,
        foreground: &RenderTexture,
        output: &RenderTexture,
    ) {
        self.clip.apply(app, shape, foreground, output);
    }

    /// Composition primitive — render `base`, blurred only inside
    /// `shape`, into `output`. Outside the shape the pixels are
    /// preserved as-is.
    ///
    /// Started life as the AUT-20 rectangle privacy blur; AUT-21
    /// generalized it to any [`MaskShape`] (rounded rect today;
    /// ellipse / circle / freehand path follow in AUT-30/-34/-35).
    /// Calling with `MaskShape::Rect` reproduces the AUT-20 behavior;
    /// `MaskShape::RoundedRect` redacts with cinematic rounded corners
    /// matching modern app surfaces.
    ///
    /// Pipeline (all RTs match `base`'s dimensions at the renderer's
    /// output format):
    ///
    /// ```text
    ///   base ─ BlurFilter(radius) ─►  blur_rt
    ///                                   │
    ///                                   ├─ ClipPipeline(shape) ─►  masked_rt
    ///                                   │
    ///   base ─────────────────────────► output  (Blit::REPLACE)
    ///                                   │
    ///   masked_rt ────────────────────► output  (Blit::ALPHA_BLENDING — over)
    /// ```
    ///
    /// `shape` is in NDC `[-1, +1]²` (screen space). `radius` is the
    /// Gaussian blur radius in pixels; AUT-22 will expose this as a
    /// scene-data parameter rather than just a method argument.
    ///
    /// Use this when you've pre-rendered a frame into `base` (e.g.
    /// the recording surface) and want to redact a known region.
    /// Future enhancement: a [`Container`](crate::scene::Container)
    /// node type that triggers this automatically during scene
    /// traversal.
    pub fn apply_privacy_blur(
        &self,
        app: &Application,
        shape: MaskShape,
        radius: f32,
        base: &RenderTexture,
        output: &RenderTexture,
    ) {
        let format = self.output_format;
        let blur_rt = RenderTexture::with_format(app, base.width(), base.height(), format);
        let masked_rt = RenderTexture::with_format(app, base.width(), base.height(), format);

        // 1. Blur the whole frame.
        self.apply_filter(app, &crate::filter::BlurFilter::new(radius), base, &blur_rt);

        // 2. Mask the blur to the shape.
        self.clip.apply(app, shape, &blur_rt, &masked_rt);

        // 3. Copy base → output (replaces output's prior contents).
        self.blit.blit(app, base, output.view());

        // 4. Composite the masked blur over the base inside output.
        self.blit.compose_over(app, &masked_rt, output);
    }

    /// Composition primitive — render `base`, with `shape` filled by
    /// a flat `color` (M-MASK / AUT-23 solid redaction). Outside the
    /// shape the pixels are preserved as-is.
    ///
    /// The companion to [`Self::apply_privacy_blur`]. Privacy blur is
    /// *polish* (the redacted region still has texture); solid
    /// redaction is *trust* (the region is replaced with an opaque
    /// fill). Use this for content where partial reconstruction must
    /// be impossible — API keys, passwords, secrets.
    ///
    /// Pipeline:
    ///
    /// ```text
    ///   fill_rt    ← cleared to `color` (RP LoadOp::Clear)
    ///                                   │
    ///                                   ├─ ClipPipeline(shape) ─►  masked_rt
    ///                                   │
    ///   base ─────────────────────────► output  (Blit::REPLACE)
    ///                                   │
    ///   masked_rt ────────────────────► output  (Blit::ALPHA_BLENDING — over)
    /// ```
    ///
    /// Tip: use a fully-opaque `color` (alpha 1.0) for true redaction;
    /// a partial alpha will let `base` show through proportionally,
    /// which defeats the "trust" use case.
    pub fn apply_solid_redaction(
        &self,
        app: &Application,
        shape: MaskShape,
        color: Color,
        base: &RenderTexture,
        output: &RenderTexture,
    ) {
        let format = self.output_format;
        let fill_rt = RenderTexture::with_format(app, base.width(), base.height(), format);
        let masked_rt = RenderTexture::with_format(app, base.width(), base.height(), format);

        // 1. Clear fill_rt to `color` — no draws, just LoadOp::Clear.
        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp::redaction fill"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp::redaction fill pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: fill_rt.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(color.r),
                            g: f64::from(color.g),
                            b: f64::from(color.b),
                            a: f64::from(color.a),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        app.queue().submit(std::iter::once(encoder.finish()));

        // 2. Mask the fill to the shape.
        self.clip.apply(app, shape, &fill_rt, &masked_rt);

        // 3. Copy base → output.
        self.blit.blit(app, base, output.view());

        // 4. Compose the masked redaction over base inside output.
        self.blit.compose_over(app, &masked_rt, output);
    }

    /// Generate an alpha-mask `RenderTexture` for `shape` at
    /// `(w, h)` (M-DYN.1 / AUT-43). The texture stores coverage as
    /// `(m, m, m, m)` so consumers can either alpha-multiply (sample
    /// `.a`) or display as a grayscale silhouette.
    ///
    /// This primitive owns *only* coverage. Privacy blur, redaction,
    /// and spotlight composition layers (M-DYN.3+, M-VEC.4+) consume
    /// these textures separately. The cache (`AUT-44`) layers on top
    /// to avoid regenerating identical masks every frame.
    #[must_use]
    pub fn generate_mask_texture(
        &self,
        app: &Application,
        shape: MaskShape,
        w: u32,
        h: u32,
    ) -> RenderTexture {
        self.mask_texture
            .generate(app, shape, w, h, self.output_format)
    }

    /// Inverse variant of [`Self::generate_mask_texture`]: pixels
    /// outside the shape are opaque, inside are transparent. Used by
    /// spotlight / dim-outside composition (M-DYN.5).
    #[must_use]
    pub fn generate_mask_texture_inverted(
        &self,
        app: &Application,
        shape: MaskShape,
        w: u32,
        h: u32,
    ) -> RenderTexture {
        let rt = RenderTexture::with_format(app, w, h, self.output_format);
        self.mask_texture.render_into(app, shape, true, &rt);
        rt
    }

    /// Generate an alpha-mask `RenderTexture` for a freehand polygon
    /// (M-DYN.1 / AUT-43, path variant). Up to 32 vertices honored
    /// (uniform-buffer cap; same as `apply_path_clip`).
    #[must_use]
    pub fn generate_path_mask_texture(
        &self,
        app: &Application,
        points: &[glam::Vec2],
        w: u32,
        h: u32,
    ) -> RenderTexture {
        self.path_mask_texture
            .generate(app, points, w, h, self.output_format)
    }

    /// Apply a freehand polygon mask to `foreground`, writing the
    /// masked result to `output` (M-MASK / AUT-35).
    ///
    /// `points` is a closed polygon in NDC `[-1, +1]²`. Up to
    /// `MAX_PATH_POINTS` (32) vertices are honored; the rest are
    /// silently ignored. Pixels inside the polygon pass through
    /// `foreground` unchanged; pixels outside drop to alpha 0.
    ///
    /// Implementation note: the WGSL fragment shader runs a
    /// crossings-test point-in-polygon at every pixel — no
    /// tessellation, no SDF. AA is hard-edge for V1; the rasterized
    /// output is integer-pixel accurate (Jordan curve theorem).
    pub fn apply_path_clip(
        &self,
        app: &Application,
        points: &[glam::Vec2],
        foreground: &RenderTexture,
        output: &RenderTexture,
    ) {
        self.path_clip.apply(app, points, false, foreground, output);
    }

    /// Composition primitive — render `base`, with `points` (a closed
    /// polygon in NDC) filled by `color` (M-MASK / AUT-35 freehand
    /// solid redaction). Outside the polygon: base preserved.
    ///
    /// Sister to [`Self::apply_solid_redaction`] but with a freehand
    /// polygon instead of a `MaskShape`. Same four-stage composition
    /// (clear fill / mask / blit base / compose over) — the only
    /// difference is the masking pass uses [`Self::apply_path_clip`]
    /// instead of the SDF clip.
    pub fn apply_solid_redaction_path(
        &self,
        app: &Application,
        points: &[glam::Vec2],
        color: Color,
        base: &RenderTexture,
        output: &RenderTexture,
    ) {
        let format = self.output_format;
        let fill_rt = RenderTexture::with_format(app, base.width(), base.height(), format);
        let masked_rt = RenderTexture::with_format(app, base.width(), base.height(), format);

        // 1. Clear fill_rt to color.
        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp::path_redaction fill"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp::path_redaction fill pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: fill_rt.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(color.r),
                            g: f64::from(color.g),
                            b: f64::from(color.b),
                            a: f64::from(color.a),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        app.queue().submit(std::iter::once(encoder.finish()));

        // 2. Path-mask the fill.
        self.path_clip
            .apply(app, points, false, &fill_rt, &masked_rt);

        // 3. Copy base → output.
        self.blit.blit(app, base, output.view());

        // 4. Compose masked redaction over base inside output.
        self.blit.compose_over(app, &masked_rt, output);
    }

    /// Composition primitive — render `base`, dimmed everywhere
    /// *outside* `shape` (M-MASK / AUT-28 spotlight, AUT-29 dim
    /// outside). Inside the shape, pixels are preserved as-is.
    ///
    /// `dim_color` is the overlay shade applied outside the shape;
    /// its alpha controls the dim strength (0 = no effect, 1 = fully
    /// replaces the surrounding content). For "spotlight a button"
    /// effects `dim_color = Color::rgba(0.0, 0.0, 0.0, 0.65)` is a
    /// good cinematic default.
    ///
    /// Pipeline (mirrors solid redaction with an *inverted* clip):
    ///
    /// ```text
    ///   fill_rt    ← cleared to `dim_color`
    ///                                   │
    ///                                   ├─ ClipPipeline(shape, invert=true) ─►  masked_rt
    ///                                   │
    ///   base ─────────────────────────► output  (Blit::REPLACE)
    ///                                   │
    ///   masked_rt ────────────────────► output  (Blit::ALPHA_BLENDING — over)
    /// ```
    pub fn apply_spotlight(
        &self,
        app: &Application,
        shape: MaskShape,
        dim_color: Color,
        base: &RenderTexture,
        output: &RenderTexture,
    ) {
        let format = self.output_format;
        let fill_rt = RenderTexture::with_format(app, base.width(), base.height(), format);
        let masked_rt = RenderTexture::with_format(app, base.width(), base.height(), format);

        // 1. Clear fill_rt to dim_color.
        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp::spotlight fill"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp::spotlight fill pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: fill_rt.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(dim_color.r),
                            g: f64::from(dim_color.g),
                            b: f64::from(dim_color.b),
                            a: f64::from(dim_color.a),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        app.queue().submit(std::iter::once(encoder.finish()));

        // 2. Inverse-clip the dim fill — opaque OUTSIDE the shape.
        self.clip.apply_inverted(app, shape, &fill_rt, &masked_rt);

        // 3. Copy base → output.
        self.blit.blit(app, base, output.view());

        // 4. Compose dim overlay over the area outside the shape.
        self.blit.compose_over(app, &masked_rt, output);
    }

    /// Convenience wrapper over [`Self::apply_spotlight`] that
    /// consumes a [`DimOutside`](crate::scene::DimOutside) data value
    /// (M-MASK / AUT-29).
    ///
    /// Editor inspector controls and persisted documents work with
    /// `DimOutside` directly; this method exists so the app side
    /// never needs to convert a `DimStrength` enum back to an alpha
    /// itself.
    pub fn apply_dim_outside_data(
        &self,
        app: &Application,
        dim: &crate::scene::DimOutside,
        base: &RenderTexture,
        output: &RenderTexture,
    ) {
        let alpha = dim.strength.alpha();
        self.apply_spotlight(
            app,
            dim.shape,
            Color::rgba(0.0, 0.0, 0.0, alpha),
            base,
            output,
        );
    }

    /// Convenience wrapper over [`Self::apply_privacy_blur`] that
    /// consumes a [`PrivacyBlur`](crate::scene::PrivacyBlur) data
    /// value (M-MASK / AUT-22).
    ///
    /// Editor inspector controls and persisted documents work with
    /// `PrivacyBlur` structs directly; this method exists so the app
    /// side never needs to convert a `BlurStrength` enum back to a raw
    /// `f32` itself.
    pub fn apply_privacy_blur_data(
        &self,
        app: &Application,
        blur: &crate::scene::PrivacyBlur,
        base: &RenderTexture,
        output: &RenderTexture,
    ) {
        self.apply_privacy_blur(app, blur.shape, blur.strength.radius_px(), base, output);
    }

    /// Fast path: one render pass, no offscreen indirection.
    fn render_stage_fast(
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

    /// Slow path with auto-dispatch — see [`render_stage`](Self::render_stage).
    fn render_stage_with_advanced_dispatch(
        &self,
        app: &Application,
        view: &wgpu::TextureView,
        clear: Color,
        stage: &Stage,
        dispatched: &[crate::scene::NodeId],
    ) -> RenderStats {
        let (w, h) = (app.width(), app.height());
        let format = self.output_format;
        let mut dest_a = RenderTexture::with_format(app, w, h, format);
        let mut dest_b = RenderTexture::with_format(app, w, h, format);

        // Build the exclude set: each dispatched node's subtree is
        // handled separately, so the main pass skips them.
        let exclude: std::collections::HashSet<crate::scene::NodeId> =
            dispatched.iter().copied().collect();

        // Phase 1: render the scene, minus the dispatched subtrees,
        // into `dest_a`.
        let mut stats = self.draw_subtree_to_rt(app, &dest_a, clear, stage, stage.root(), &exclude);

        // Phase 2: for each dispatched node in pre-order, render its
        // subtree into a fresh foreground RT, optionally apply the
        // container's clip, then composite onto the in-progress dest.
        // Ping-pong dest_a ↔ dest_b so we don't read+write the same RT
        // in one pass.
        let foreground = RenderTexture::with_format(app, w, h, format);
        let masked = RenderTexture::with_format(app, w, h, format);
        let empty_exclude = std::collections::HashSet::new();
        for &node_id in dispatched {
            let Some(node) = stage.get(node_id) else {
                continue;
            };
            let container = node.container();
            let mode = container.blend_mode;
            let clip_shape = container.clip;

            let sub_stats = self.draw_subtree_to_rt(
                app,
                &foreground,
                Color::rgba(0.0, 0.0, 0.0, 0.0),
                stage,
                node_id,
                &empty_exclude,
            );
            stats.draw_calls += sub_stats.draw_calls;
            stats.sprites_drawn += sub_stats.sprites_drawn;
            stats.graphics_drawn += sub_stats.graphics_drawn;
            stats.glyphs_drawn += sub_stats.glyphs_drawn;
            stats.meshes_drawn += sub_stats.meshes_drawn;

            // If a clip is set, apply it: foreground → masked. Otherwise
            // the foreground is the source as-is.
            let composite_src = if let Some(shape) = clip_shape {
                self.clip.apply(app, shape, &foreground, &masked);
                &masked
            } else {
                &foreground
            };

            if mode.is_advanced() {
                // Advanced blend writes the composite into dest_b, swap.
                self.advanced_blend
                    .apply(app, mode, &dest_a, composite_src, &dest_b);
                std::mem::swap(&mut dest_a, &mut dest_b);
            } else {
                // Native blend (typically Normal for clip-only nodes):
                // source-over composite onto dest_a in place.
                self.blit.compose_over(app, composite_src, &dest_a);
            }
        }

        // Phase 3: blit final composited RT to the user-supplied view.
        self.blit.blit(app, &dest_a, view);
        stats
    }

    /// Draw a subtree of `stage` (rooted at `start`, skipping `exclude`)
    /// into `dest`. Used by both phases of the advanced-dispatch path.
    fn draw_subtree_to_rt(
        &self,
        app: &Application,
        dest: &RenderTexture,
        clear: Color,
        stage: &Stage,
        start: crate::scene::NodeId,
        exclude: &std::collections::HashSet<crate::scene::NodeId>,
    ) -> RenderStats {
        let mut stats = RenderStats::default();
        Self::with_clearing_pass(app, dest.view(), clear, |pass| {
            let (sprite_calls, sprites) =
                self.sprite.draw_subtree(app, pass, stage, start, exclude);
            let (graphics_calls, graphics) =
                self.graphics.draw_subtree(app, pass, stage, start, exclude);
            let (text_calls, glyphs) = self.text.draw_subtree(app, pass, stage, start, exclude);
            let (mesh_calls, meshes) = self.mesh.draw_subtree(app, pass, stage, start, exclude);
            stats.draw_calls = sprite_calls + graphics_calls + text_calls + mesh_calls;
            stats.sprites_drawn = sprites;
            stats.graphics_drawn = graphics;
            stats.glyphs_drawn = glyphs;
            stats.meshes_drawn = meshes;
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

/// Pre-order walk that returns every visible node which needs the
/// slow-path dispatch — either:
///
/// - the container has an advanced
///   [`BlendMode`](crate::blend::BlendMode) (Tier C — Overlay,
///   `ColorBurn`, …) requiring an offscreen backdrop sample, or
/// - the container has a [`MaskShape`] set in `Container::clip`,
///   requiring an offscreen mask pass.
///
/// Order is pre-order so the auto-dispatch composites in z-order: a
/// later dispatched node sees its earlier siblings (and their
/// composited results) as the backdrop.
fn collect_dispatched_nodes(stage: &Stage) -> Vec<crate::scene::NodeId> {
    let mut out = Vec::new();
    let mut stack: Vec<crate::scene::NodeId> = vec![stage.root()];
    while let Some(id) = stack.pop() {
        let Some(node) = stage.get(id) else { continue };
        let container = node.container();
        if !container.visible {
            continue;
        }
        let needs_dispatch = container.blend_mode.is_advanced() || container.clip.is_some();
        if needs_dispatch {
            out.push(id);
            // Don't recurse — the subtree is rendered separately by the
            // dispatcher with the parent's mode/clip applied at composition.
            continue;
        }
        for child in container.children().rev().collect::<Vec<_>>() {
            stack.push(child);
        }
    }
    out
}
