//! `Sprite3D` — unlit alpha-blended primitives placed in 3D space
//! (W3D.6 / AUT-298).
//!
//! ## Why "sprite" (not "mesh")
//!
//! `Sprite3D` is the wisp-3d equivalent of THREE's `MeshBasicMaterial`:
//! no lighting, no shading, just a color/tint. The 404 page uses it
//! for the eye composition (glow ellipse + iris ring + pupil) and
//! the base-glow circle — geometry that should sit AT a depth (so
//! the camera occludes it correctly if you spin around) but should
//! NOT occlude what's behind it (so the eye doesn't punch a black
//! hole through the pyramid).
//!
//! That's the "alpha occlusion gotcha" — the classic 3D-rendering
//! trap. Translucent geometry must be drawn with
//! `depth_test: LessEqual` + `depth_write: false` or it
//! corrupts the depth buffer for everything drawn after it.
//! [`SpritePipeline::new`] hardwires this state.
//!
//! ## Geometry constructors
//!
//! [`Sprite3D::circle`], [`Sprite3D::ring`], [`Sprite3D::quad`] each
//! return a [`Mesh3D`] — `Sprite3D` reuses the mesh vertex layout so
//! the underlying buffer machinery is shared. Sprites carry dummy
//! `Vec3::Z` normals; the unlit shader discards them.

use glam::Mat4;
use wisp::application::Application;

use crate::camera::{Camera3D, ViewProj};
use crate::mesh::{Mesh3D, Vertex3D};
use crate::render::{DEPTH_FORMAT, ModelUniform};

/// Free-function namespace for sprite geometry constructors. Returns
/// `Mesh3D` so callers can hand the result to either
/// [`SpritePipeline::draw_one`] (alpha-blended, depth-write-off) or
/// the regular [`crate::Render3DPass`] (opaque + lit).
pub struct Sprite3D;

impl Sprite3D {
    /// Filled disc — apex-fan triangulation around the centre.
    /// `radius > 0`, `segments >= 3` (clamped to 3).
    #[must_use]
    pub fn circle(radius: f32, segments: u32) -> Mesh3D {
        let n = segments.max(3) as usize;
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n + 1);
        // Centre vertex.
        positions.push([0.0, 0.0, 0.0]);
        // Rim vertices.
        for i in 0..n {
            #[allow(
                clippy::cast_precision_loss,
                reason = "segments is bounded by u16 in practice; f32 mantissa sufficient"
            )]
            let theta = std::f32::consts::TAU * (i as f32) / (n as f32);
            positions.push([radius * theta.cos(), radius * theta.sin(), 0.0]);
        }
        let mut indices: Vec<u32> = Vec::with_capacity(n * 3);
        for i in 0..n {
            let a = 0_u32;
            #[allow(
                clippy::cast_possible_truncation,
                reason = "segments is bounded by u16 in practice; cast safe"
            )]
            let b = 1 + i as u32;
            #[allow(
                clippy::cast_possible_truncation,
                reason = "segments is bounded by u16 in practice; cast safe"
            )]
            let c = 1 + ((i + 1) % n) as u32;
            indices.extend_from_slice(&[a, b, c]);
        }
        let normals = vec![[0.0_f32, 0.0, 1.0]; positions.len()];
        Mesh3D {
            positions,
            normals,
            indices,
        }
    }

    /// Annulus / ring (filled donut shape). `inner < outer`,
    /// `segments >= 3`. Triangulates as a strip of quads (2 tris
    /// each) around the perimeter.
    #[must_use]
    pub fn ring(inner: f32, outer: f32, segments: u32) -> Mesh3D {
        let n = segments.max(3) as usize;
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n * 2);
        for i in 0..n {
            #[allow(
                clippy::cast_precision_loss,
                reason = "segments bounded by u16 in practice"
            )]
            let theta = std::f32::consts::TAU * (i as f32) / (n as f32);
            let c = theta.cos();
            let s = theta.sin();
            positions.push([inner * c, inner * s, 0.0]);
            positions.push([outer * c, outer * s, 0.0]);
        }
        let mut indices: Vec<u32> = Vec::with_capacity(n * 6);
        for i in 0..n {
            let next = (i + 1) % n;
            #[allow(
                clippy::cast_possible_truncation,
                reason = "segments bounded by u16 in practice"
            )]
            let i0 = (i * 2) as u32; // inner @ i
            #[allow(
                clippy::cast_possible_truncation,
                reason = "segments bounded by u16 in practice"
            )]
            let o0 = (i * 2 + 1) as u32; // outer @ i
            #[allow(
                clippy::cast_possible_truncation,
                reason = "segments bounded by u16 in practice"
            )]
            let i1 = (next * 2) as u32;
            #[allow(
                clippy::cast_possible_truncation,
                reason = "segments bounded by u16 in practice"
            )]
            let o1 = (next * 2 + 1) as u32;
            // Two CCW triangles per quad.
            indices.extend_from_slice(&[i0, o0, o1, i0, o1, i1]);
        }
        let normals = vec![[0.0_f32, 0.0, 1.0]; positions.len()];
        Mesh3D {
            positions,
            normals,
            indices,
        }
    }

    /// Axis-aligned rectangle centred at the origin in the XY plane.
    #[must_use]
    pub fn quad(width: f32, height: f32) -> Mesh3D {
        let hw = width * 0.5;
        let hh = height * 0.5;
        let positions = vec![
            [-hw, -hh, 0.0],
            [hw, -hh, 0.0],
            [hw, hh, 0.0],
            [-hw, hh, 0.0],
        ];
        let indices = vec![0, 1, 2, 0, 2, 3];
        let normals = vec![[0.0_f32, 0.0, 1.0]; positions.len()];
        Mesh3D {
            positions,
            normals,
            indices,
        }
    }
}

/// Unlit-sprite pipeline: alpha-blend ON, depth-test ON, depth-write
/// OFF.
pub struct SpritePipeline {
    pipeline: wgpu::RenderPipeline,
    model_layout: wgpu::BindGroupLayout,
    view_proj_buffer: wgpu::Buffer,
    view_proj_bg: wgpu::BindGroup,
}

impl SpritePipeline {
    /// Construct. Format + MSAA must match the underlying mesh pass
    /// so they can share color + depth attachments.
    #[must_use]
    #[allow(
        clippy::too_many_lines,
        reason = "Single linear pipeline-setup function; splitting would scatter the bind-group-layout/buffer/pipeline triple across helpers with no reuse."
    )]
    pub fn new(app: &Application, output_format: wgpu::TextureFormat, msaa_samples: u32) -> Self {
        let device = app.device();
        let view_proj_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp_3d::sprite::view_proj_layout"),
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
            label: Some("wisp_3d::sprite::model_layout"),
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
            label: Some("wisp_3d::sprite::view_proj_buffer"),
            size: std::mem::size_of::<ViewProj>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_proj_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp_3d::sprite::view_proj_bg"),
            layout: &view_proj_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: view_proj_buffer.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp_3d::sprite_unlit"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sprite_unlit.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wisp_3d::sprite::pipeline_layout"),
            bind_group_layouts: &[&view_proj_layout, &model_layout],
            push_constant_ranges: &[],
        });
        let attrs = Mesh3D::wgpu_attributes();
        let vbo_layout = Mesh3D::wgpu_vertex_buffer_layout(&attrs);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wisp_3d::sprite_pipeline"),
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
                // Sprites are typically thin / single-sided; we
                // intentionally don't cull so the user can place a
                // sprite anywhere in 3D space and see it from either
                // side without surprises.
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                // ★ The alpha-occlusion gotcha. Sprites participate in
                // depth-TEST (so opaque geometry occludes them) but do
                // NOT write depth (so they don't punch holes in the
                // depth buffer for whatever's drawn after them).
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
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
        let _ = view_proj_layout;
        Self {
            pipeline,
            model_layout,
            view_proj_buffer,
            view_proj_bg,
        }
    }

    /// Encode a single sprite draw to a standalone render pass.
    /// Caller owns color + depth attachments.
    #[allow(
        clippy::too_many_arguments,
        reason = "Render-call surface intentionally exposes every wgpu attachment / state input; wrapping in a builder would just defer the same set of fields."
    )]
    pub fn draw_one(
        &mut self,
        app: &Application,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        camera: &Camera3D,
        mesh: &Mesh3D,
        model: Mat4,
        tint: [f32; 4],
    ) {
        if mesh.indices.is_empty() {
            return;
        }
        let device = app.device();
        let vp = camera.view_proj_uniform();
        app.queue()
            .write_buffer(&self.view_proj_buffer, 0, bytemuck::bytes_of(&vp));

        let vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp_3d::sprite::vbuf"),
            size: (mesh.vertex_buffer().len() * std::mem::size_of::<Vertex3D>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        app.queue()
            .write_buffer(&vbuf, 0, bytemuck::cast_slice(&mesh.vertex_buffer()));
        let ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp_3d::sprite::ibuf"),
            size: (mesh.indices.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        app.queue()
            .write_buffer(&ibuf, 0, bytemuck::cast_slice(&mesh.indices));

        let m = ModelUniform::new(model, tint);
        let mbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp_3d::sprite::model_buf"),
            size: std::mem::size_of::<ModelUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        app.queue().write_buffer(&mbuf, 0, bytemuck::bytes_of(&m));
        let mbg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp_3d::sprite::model_bg"),
            layout: &self.model_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: mbuf.as_entire_binding(),
            }],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wisp_3d::sprite::pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.view_proj_bg, &[]);
        pass.set_bind_group(1, &mbg, &[]);
        pass.set_vertex_buffer(0, vbuf.slice(..));
        pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "indices.len() bounded by realistic mesh sizes"
        )]
        let idx_count = mesh.indices.len() as u32;
        pass.draw_indexed(0..idx_count, 0, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wisp::application::AppConfig;

    fn make_app() -> Application {
        pollster::block_on(Application::new(AppConfig::default())).expect("app init")
    }

    #[test]
    fn circle_vertex_count_is_segments_plus_center() {
        let m = Sprite3D::circle(1.0, 32);
        assert_eq!(m.positions.len(), 33);
        // 32 triangles, 3 indices each.
        assert_eq!(m.indices.len(), 96);
        assert_eq!(m.triangle_count(), 32);
    }

    #[test]
    fn circle_clamps_segments_to_minimum_three() {
        let m = Sprite3D::circle(1.0, 1);
        assert_eq!(m.positions.len(), 4); // 1 center + 3 rim
        assert_eq!(m.triangle_count(), 3);
    }

    #[test]
    fn ring_vertex_count_is_2x_segments() {
        let m = Sprite3D::ring(0.5, 1.0, 48);
        assert_eq!(m.positions.len(), 96);
        // 48 quads × 2 tris × 3 indices.
        assert_eq!(m.indices.len(), 48 * 6);
        assert_eq!(m.triangle_count(), 96);
    }

    #[test]
    fn quad_has_4_vertices_and_2_triangles() {
        let m = Sprite3D::quad(2.0, 1.0);
        assert_eq!(m.positions.len(), 4);
        assert_eq!(m.indices.len(), 6);
        assert_eq!(m.triangle_count(), 2);
        // 1×2 quad spans [-1, 1] × [-0.5, 0.5].
        let half = 0.5_f32;
        assert!(m.positions.iter().any(|p| (p[0] - -1.0).abs() < 1e-6));
        assert!(m.positions.iter().any(|p| (p[0] - 1.0).abs() < 1e-6));
        assert!(m.positions.iter().any(|p| (p[1] - half).abs() < 1e-6));
    }

    #[test]
    fn sprite_pipeline_builds() {
        let app = make_app();
        let _p = SpritePipeline::new(&app, wgpu::TextureFormat::Rgba8UnormSrgb, 1);
    }

    #[test]
    fn sprite_draw_emits_no_validation_errors() {
        let app = make_app();
        let mut p = SpritePipeline::new(&app, wgpu::TextureFormat::Rgba8UnormSrgb, 1);
        let color_tex = app.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("test::offscreen_color"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color_view = color_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_tex = app.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("test::offscreen_depth"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
        // Clear color + depth first so the sprite pass's
        // `LoadOp::Load` reads valid data.
        let mut encoder0 = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("test::clear"),
            });
        {
            let _ = encoder0.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("test::clear_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        app.queue().submit(std::iter::once(encoder0.finish()));

        let camera = Camera3D::perspective(45.0, 1.0, 0.1, 100.0);
        let ring = Sprite3D::ring(0.3, 0.5, 32);
        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("test::sprite"),
            });
        app.device().push_error_scope(wgpu::ErrorFilter::Validation);
        p.draw_one(
            &app,
            &mut encoder,
            &color_view,
            &depth_view,
            &camera,
            &ring,
            Mat4::IDENTITY,
            [1.0, 0.5, 0.0, 0.8],
        );
        app.queue().submit(std::iter::once(encoder.finish()));
        let err = pollster::block_on(app.device().pop_error_scope());
        assert!(err.is_none(), "validation: {err:?}");
    }
}
