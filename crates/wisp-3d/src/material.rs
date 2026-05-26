//! `Material3D` — user-supplied WGSL fragment + uniforms, with a
//! built-in `PaletteRampMaterial` matching the engmanager.xyz 404
//! pyramid shader (W3D.4 / AUT-296).
//!
//! ## Why a trait
//!
//! `Render3DPass`'s default shader is a flat lambert — fine for
//! storybook stories, useless for the 404 page's painterly look.
//! Rather than fork the pass per material, the pass exposes a
//! [`draw_material`] entry point that compiles a user material's
//! WGSL into its own pipeline, caches it keyed on `TypeId`, and
//! re-uses it for every subsequent draw of the same material type.
//!
//! ## Reference material
//!
//! [`PaletteRampMaterial`] ports the 404 fragment shader:
//! 5-stop palette ramp along a fixed diagonal + warm-band overlay +
//! fake directional lambert + rim + value-noise grain. Time uniform
//! lives in the same `Palette` UBO so the storybook story (W3D.7)
//! can drive it from a `wisp_animation::Driver`.

use std::any::TypeId;
use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wisp::application::Application;

use crate::camera::{Camera3D, ViewProj};
use crate::mesh::{Mesh3D, Vertex3D};
use crate::render::{DEPTH_FORMAT, ModelUniform};

/// Pluggable material. Implementors supply:
/// - a stable WGSL source string (returned by [`Self::wgsl_source`]),
/// - a `#[repr(C)]` uniform struct (the `Uniforms` associated type),
/// - a per-frame [`Self::uniforms`] accessor that snapshots state.
///
/// The pass takes care of compiling the shader, building a pipeline
/// keyed on the implementor's `TypeId`, allocating + writing the
/// uniform UBO, and binding it at group 2.
pub trait Material3D: 'static {
    /// `#[repr(C)]` UBO struct uploaded at group 2 binding 0.
    type Uniforms: Pod + Zeroable;

    /// Full WGSL source — must contain `main_vs` + `main_fs` entry
    /// points and declare the standard bind groups (`view_proj` at
    /// group 0, `model` at group 1, the material's own UBO at group
    /// 2). See `shaders/material_palette.wgsl` for the canonical
    /// shape.
    fn wgsl_source() -> &'static str;

    /// Per-frame uniform snapshot.
    fn uniforms(&self) -> Self::Uniforms;
}

/// Material pipeline cache. Owned by `MaterialRenderer`; keys on
/// `(TypeId, output_format, msaa_samples)` so the same material
/// recompiles when the swapchain config flips.
#[derive(Default)]
pub struct MaterialCache {
    entries: HashMap<MaterialKey, MaterialPipeline>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct MaterialKey {
    type_id: TypeId,
    output_format: wgpu::TextureFormat,
    msaa_samples: u32,
}

struct MaterialPipeline {
    pipeline: wgpu::RenderPipeline,
    material_layout: wgpu::BindGroupLayout,
}

/// High-level helper that builds + caches material pipelines and
/// exposes a `draw_one` that consumes view-proj + model + uniforms +
/// a mesh and encodes a single render pass.
///
/// Kept separate from `Render3DPass` so the default shader stays the
/// fast path. Use this when you want shader customization.
pub struct MaterialRenderer {
    cache: MaterialCache,
    view_proj_layout: wgpu::BindGroupLayout,
    model_layout: wgpu::BindGroupLayout,
    view_proj_buffer: wgpu::Buffer,
    view_proj_bg: wgpu::BindGroup,
}

impl MaterialRenderer {
    /// Construct the renderer's shared layouts + view-proj UBO.
    #[must_use]
    pub fn new(app: &Application) -> Self {
        let device = app.device();
        let view_proj_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp_3d::material::view_proj_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<ViewProj>() as u64),
                },
                count: None,
            }],
        });
        let model_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp_3d::material::model_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        std::mem::size_of::<ModelUniform>() as u64
                    ),
                },
                count: None,
            }],
        });
        let view_proj_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp_3d::material::view_proj_buffer"),
            size: std::mem::size_of::<ViewProj>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_proj_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp_3d::material::view_proj_bg"),
            layout: &view_proj_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: view_proj_buffer.as_entire_binding(),
            }],
        });
        Self {
            cache: MaterialCache::default(),
            view_proj_layout,
            model_layout,
            view_proj_buffer,
            view_proj_bg,
        }
    }

    /// Number of cached pipelines (for tests + diagnostics).
    #[must_use]
    pub fn cached_pipeline_count(&self) -> usize {
        self.cache.entries.len()
    }

    /// Encode a single-mesh draw using material `M`.
    ///
    /// Builds (or re-uses) the pipeline for `M`, uploads the
    /// view-proj UBO, builds per-draw vertex/index/model/material
    /// buffers, encodes one render pass with a depth attachment
    /// (caller-owned via `depth_view`), and submits no commands of
    /// its own — the caller owns `encoder` + `queue.submit`.
    ///
    /// Use this for one-off material draws (storybook stories,
    /// examples). For per-frame compositions with many objects, batch
    /// into one render pass yourself; the pipeline lookup is cheap
    /// (`HashMap` by `TypeId`).
    #[allow(
        clippy::too_many_arguments,
        reason = "Render-call surface intentionally exposes every wgpu attachment / state input; wrapping in a builder would just defer the same set of fields."
    )]
    pub fn draw_one<M: Material3D>(
        &mut self,
        app: &Application,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        camera: &Camera3D,
        material: &M,
        mesh: &Mesh3D,
        model: Mat4,
        tint: [f32; 4],
        clear_color: wgpu::Color,
        output_format: wgpu::TextureFormat,
        msaa_samples: u32,
    ) {
        let device = app.device();

        // 1. Upload view-proj.
        let vp = camera.view_proj_uniform();
        app.queue()
            .write_buffer(&self.view_proj_buffer, 0, bytemuck::bytes_of(&vp));

        // 2. Get / build the pipeline.
        let key = MaterialKey {
            type_id: TypeId::of::<M>(),
            output_format,
            msaa_samples,
        };
        // Two-step to avoid double-borrow on self.cache (we need
        // self.view_proj_layout etc. for the build path).
        if !self.cache.entries.contains_key(&key) {
            let entry = self.build_pipeline::<M>(device, output_format, msaa_samples);
            self.cache.entries.insert(key, entry);
        }
        let pipeline_entry = &self.cache.entries[&key];

        // 3. Per-mesh resources.
        let vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp_3d::material::vbuf"),
            size: (mesh.vertex_buffer().len() * std::mem::size_of::<Vertex3D>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        app.queue()
            .write_buffer(&vbuf, 0, bytemuck::cast_slice(&mesh.vertex_buffer()));
        let ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp_3d::material::ibuf"),
            size: (mesh.indices.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        app.queue()
            .write_buffer(&ibuf, 0, bytemuck::cast_slice(&mesh.indices));

        // 4. Model + material UBOs.
        let model_uniform = ModelUniform::new(model, tint);
        let model_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp_3d::material::model_buf"),
            size: std::mem::size_of::<ModelUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        app.queue()
            .write_buffer(&model_buf, 0, bytemuck::bytes_of(&model_uniform));
        let model_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp_3d::material::model_bg"),
            layout: &self.model_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buf.as_entire_binding(),
            }],
        });

        let mat_uniform = material.uniforms();
        let mat_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp_3d::material::mat_buf"),
            size: std::mem::size_of::<M::Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        app.queue()
            .write_buffer(&mat_buf, 0, bytemuck::bytes_of(&mat_uniform));
        let mat_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp_3d::material::mat_bg"),
            layout: &pipeline_entry.material_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: mat_buf.as_entire_binding(),
            }],
        });

        // 5. Pass.
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wisp_3d::material::pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&pipeline_entry.pipeline);
        pass.set_bind_group(0, &self.view_proj_bg, &[]);
        pass.set_bind_group(1, &model_bg, &[]);
        pass.set_bind_group(2, &mat_bg, &[]);
        pass.set_vertex_buffer(0, vbuf.slice(..));
        pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "indices.len() is bounded well below u32::MAX for any realistic mesh"
        )]
        let idx_count = mesh.indices.len() as u32;
        pass.draw_indexed(0..idx_count, 0, 0..1);
    }

    /// Build the pipeline for material `M`. Called once per
    /// `(TypeId, output_format, msaa_samples)` from `draw_one`'s
    /// cache-miss path.
    fn build_pipeline<M: Material3D>(
        &self,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        msaa_samples: u32,
    ) -> MaterialPipeline {
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp_3d::material::material_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        std::mem::size_of::<M::Uniforms>() as u64
                    ),
                },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp_3d::material::shader"),
            source: wgpu::ShaderSource::Wgsl(M::wgsl_source().into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wisp_3d::material::pipeline_layout"),
            bind_group_layouts: &[&self.view_proj_layout, &self.model_layout, &material_layout],
            push_constant_ranges: &[],
        });
        let attrs = Mesh3D::wgpu_attributes();
        let vbo_layout = Mesh3D::wgpu_vertex_buffer_layout(&attrs);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wisp_3d::material::pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("main_vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[vbo_layout],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: msaa_samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("main_fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        MaterialPipeline {
            pipeline,
            material_layout,
        }
    }
}

// ─── PaletteRampMaterial — reference impl matching the 404 ─────────

/// 5-stop palette uniform. Layout matches the WGSL `Palette` struct
/// in `shaders/material_palette.wgsl`.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PaletteUniform {
    /// 5 stop RGBA colors in order: peach, maroon, pink, mauve, blue.
    pub stops: [[f32; 4]; 5],
    /// `.x` = elapsed seconds, `.yzw` = padding.
    pub time: [f32; 4],
}

/// Reference `Material3D` impl. Five Catppuccin-inspired stops driving
/// a palette ramp; matches the engmanager.xyz 404 fragment shader.
#[derive(Clone, Debug)]
pub struct PaletteRampMaterial {
    /// Five stop colors (peach, maroon, pink, mauve, blue) as `[f32; 4]`
    /// RGBA. Use [`PaletteRampMaterial::engmanager_404`] for the
    /// hex-port defaults.
    pub stops: [[f32; 4]; 5],
    /// Elapsed seconds for the time-dependent `t` offset + grain.
    pub time_seconds: f32,
}

impl PaletteRampMaterial {
    /// Defaults matching `not-found.js`:
    /// `#fe640b, #e64553, #ea76cb, #8839ef, #1e66f5`. Time defaults
    /// to `0.0`; set per frame via [`Self::with_time`].
    #[must_use]
    pub fn engmanager_404() -> Self {
        Self {
            stops: [
                hex_to_rgba("#fe640b"),
                hex_to_rgba("#e64553"),
                hex_to_rgba("#ea76cb"),
                hex_to_rgba("#8839ef"),
                hex_to_rgba("#1e66f5"),
            ],
            time_seconds: 0.0,
        }
    }

    /// Builder-style time setter.
    #[must_use]
    pub fn with_time(mut self, t: f32) -> Self {
        self.time_seconds = t;
        self
    }
}

impl Material3D for PaletteRampMaterial {
    type Uniforms = PaletteUniform;

    fn wgsl_source() -> &'static str {
        include_str!("../shaders/material_palette.wgsl")
    }

    fn uniforms(&self) -> PaletteUniform {
        PaletteUniform {
            stops: self.stops,
            time: [self.time_seconds, 0.0, 0.0, 0.0],
        }
    }
}

/// Parse a hex string of the form `#rrggbb` into linear-ish RGBA.
/// Used by [`PaletteRampMaterial::engmanager_404`] so the JS hex
/// literals can be copy-pasted without manual conversion.
#[must_use]
fn hex_to_rgba(hex: &str) -> [f32; 4] {
    let stripped = hex.strip_prefix('#').unwrap_or(hex);
    if stripped.len() != 6 {
        return [1.0, 0.0, 1.0, 1.0]; // visible magenta on parse fail
    }
    let r = u8::from_str_radix(&stripped[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&stripped[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&stripped[4..6], 16).unwrap_or(255);
    [
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
        1.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use wisp::application::AppConfig;

    fn make_app() -> Application {
        pollster::block_on(Application::new(AppConfig::default())).expect("app init")
    }

    #[test]
    fn hex_parser_round_trips_known_color() {
        let c = hex_to_rgba("#fe640b");
        assert!((c[0] - f32::from(0xfe_u8) / 255.0).abs() < 1e-6);
        assert!((c[1] - f32::from(0x64_u8) / 255.0).abs() < 1e-6);
        assert!((c[2] - f32::from(0x0b_u8) / 255.0).abs() < 1e-6);
        assert!((c[3] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn hex_parser_handles_missing_hash() {
        assert!((hex_to_rgba("ffffff")[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn engmanager_defaults_have_5_distinct_stops() {
        let m = PaletteRampMaterial::engmanager_404();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "Stops are in [0, 1]; scaled-by-255 fits u8 by construction."
        )]
        let mut bits: Vec<u32> = m
            .stops
            .iter()
            .map(|s| {
                u32::from_le_bytes([
                    (s[0] * 255.0) as u8,
                    (s[1] * 255.0) as u8,
                    (s[2] * 255.0) as u8,
                    255,
                ])
            })
            .collect();
        bits.sort_unstable();
        bits.dedup();
        assert_eq!(bits.len(), 5);
    }

    #[test]
    fn material_renderer_builds() {
        let app = make_app();
        let renderer = MaterialRenderer::new(&app);
        assert_eq!(renderer.cached_pipeline_count(), 0);
    }

    #[test]
    fn draw_one_caches_pipeline_per_key() {
        let app = make_app();
        let mut renderer = MaterialRenderer::new(&app);
        let camera = Camera3D::perspective(45.0, 1.0, 0.1, 100.0);
        let mesh = Mesh3D::pyramid(1.34, 1.25);
        let material = PaletteRampMaterial::engmanager_404();

        let color_tex = make_offscreen(&app, 64, 64, 1, wgpu::TextureFormat::Rgba8UnormSrgb);
        let color_view = color_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_tex = make_depth(&app, 64, 64, 1);
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

        for _ in 0..3 {
            let mut encoder =
                app.device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("test"),
                    });
            app.device().push_error_scope(wgpu::ErrorFilter::Validation);
            renderer.draw_one(
                &app,
                &mut encoder,
                &color_view,
                &depth_view,
                &camera,
                &material,
                &mesh,
                Mat4::IDENTITY,
                [1.0, 1.0, 1.0, 1.0],
                wgpu::Color::TRANSPARENT,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                1,
            );
            app.queue().submit(std::iter::once(encoder.finish()));
            let err = pollster::block_on(app.device().pop_error_scope());
            assert!(err.is_none(), "validation: {err:?}");
        }
        // 3 draws of the same material at the same (format, msaa)
        // → exactly 1 cached pipeline.
        assert_eq!(renderer.cached_pipeline_count(), 1);
    }

    fn make_offscreen(
        app: &Application,
        w: u32,
        h: u32,
        msaa: u32,
        fmt: wgpu::TextureFormat,
    ) -> wgpu::Texture {
        app.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("test::offscreen_color"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: msaa,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    fn make_depth(app: &Application, w: u32, h: u32, msaa: u32) -> wgpu::Texture {
        app.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("test::offscreen_depth"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: msaa,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    #[test]
    fn palette_uniform_size_matches_wgsl_layout() {
        // 5 × vec4<f32> (80 bytes) + 1 × vec4<f32> (16 bytes) = 96.
        assert_eq!(std::mem::size_of::<PaletteUniform>(), 96);
        assert_eq!(std::mem::align_of::<PaletteUniform>(), 16);
    }
}
