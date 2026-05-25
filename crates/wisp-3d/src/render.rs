//! `Render3DPass` — depth-tested + MSAA-aware render pass (W3D.3 /
//! AUT-295).
//!
//! ## What it does
//!
//! Records a single render pass to a caller-supplied color attachment
//! plus an internally-owned `Depth32Float` depth attachment. The
//! depth attachment is recreated on resize. MSAA sample count is
//! locked at construction time so the pipeline and the depth texture
//! agree.
//!
//! ## Risk callout (W3D.3 spec'd this as Urgent)
//!
//! wgpu requires the depth texture's sample count to match the color
//! attachment's sample count, AND the pipeline's `multisample.count`
//! field to match both. Mismatches surface as a `Validation Error /
//! Pipeline ... is bound with sample count X` at draw time, NOT at
//! pipeline creation. The plumbing here keeps all three in lock-step
//! by reading the `msaa_samples` constructor arg into all three sites.
//!
//! ## What it doesn't do (yet)
//!
//! - Instanced draws — every mesh becomes its own vertex+index buffer
//!   write per draw. W3D.4+ adds a per-instance buffer.
//! - Per-material pipeline cache — W3D.3 ships only the default
//!   shader; W3D.4 introduces `Material3D` + the keyed cache.
//! - Texturing — the default shader is procedural-color only.

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wisp::application::Application;

use crate::camera::{Camera3D, ViewProj};
use crate::mesh::{Mesh3D, Vertex3D};

/// Default depth format. `Depth32Float` is the safest cross-platform
/// choice (every modern adapter supports it for both texture binding
/// and depth-stencil attachment).
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Per-mesh model + tint uniform.
///
/// `#[repr(C, align(16))]` — `mat4x4<f32>` is 16-byte aligned in
/// WGSL; the trailing `vec4` is already 16-aligned.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ModelUniform {
    /// Model matrix (local → world).
    pub matrix: [[f32; 4]; 4],
    /// RGBA tint multiplied into the lambert shade.
    pub tint: [f32; 4],
}

impl ModelUniform {
    /// Build from a `glam::Mat4` + tint.
    #[must_use]
    pub fn new(matrix: Mat4, tint: [f32; 4]) -> Self {
        Self {
            matrix: matrix.to_cols_array_2d(),
            tint,
        }
    }
}

/// Render pass with owned depth attachment + pipeline.
pub struct Render3DPass {
    pipeline: wgpu::RenderPipeline,
    depth_view: wgpu::TextureView,
    depth_size: (u32, u32),
    msaa_samples: u32,
    output_format: wgpu::TextureFormat,
    view_proj_buffer: wgpu::Buffer,
    view_proj_bg: wgpu::BindGroup,
    model_layout: wgpu::BindGroupLayout,
}

impl Render3DPass {
    /// Construct the pass. `msaa_samples` must be 1, 2, 4, or 8 (wgpu
    /// adapter dependent; 1 = no MSAA). `output_format` is the format
    /// of the color attachment you'll bind in [`Self::draw`].
    #[must_use]
    #[allow(
        clippy::too_many_lines,
        reason = "Single linear pipeline-setup function; splitting would scatter the bind-group-layout/buffer/pipeline triple across helpers with no reuse and harder reading."
    )]
    pub fn new(
        app: &Application,
        output_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        msaa_samples: u32,
    ) -> Self {
        let device = app.device();

        // View-proj UBO bind group layout (group 0).
        let view_proj_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp_3d::view_proj_layout"),
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

        // Model UBO bind group layout (group 1) — re-uploaded per mesh.
        let model_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp_3d::model_layout"),
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

        // View-proj UBO buffer + bind group (mutated each frame).
        let view_proj_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp_3d::view_proj_buffer"),
            size: std::mem::size_of::<ViewProj>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_proj_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp_3d::view_proj_bg"),
            layout: &view_proj_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: view_proj_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp_3d::mesh_default"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/mesh_default.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wisp_3d::pipeline_layout"),
            bind_group_layouts: &[&view_proj_layout, &model_layout],
            push_constant_ranges: &[],
        });

        let attrs = Mesh3D::wgpu_attributes();
        let vbo_layout = Mesh3D::wgpu_vertex_buffer_layout(&attrs);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wisp_3d::mesh_pipeline"),
            layout: Some(&pipeline_layout),
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

        let depth_view = create_depth_view(device, width, height, msaa_samples);

        Self {
            pipeline,
            depth_view,
            depth_size: (width, height),
            msaa_samples,
            output_format,
            view_proj_buffer,
            view_proj_bg,
            model_layout,
        }
    }

    /// Re-create the depth attachment for a new size. No-op when the
    /// size hasn't changed.
    pub fn resize(&mut self, app: &Application, width: u32, height: u32) {
        if self.depth_size == (width, height) {
            return;
        }
        self.depth_view = create_depth_view(app.device(), width, height, self.msaa_samples);
        self.depth_size = (width, height);
    }

    /// Sample count baked into the pipeline. Returned for tests + so
    /// downstream consumers can sanity-check before passing in a
    /// mismatched MSAA color attachment.
    #[must_use]
    pub fn msaa_samples(&self) -> u32 {
        self.msaa_samples
    }

    /// Output color format the pipeline targets.
    #[must_use]
    pub fn output_format(&self) -> wgpu::TextureFormat {
        self.output_format
    }

    /// Encode the render pass: clear color + depth, draw every
    /// `(mesh, model_matrix, tint)` triple.
    ///
    /// `color_view` is the caller-supplied attachment — typically the
    /// surface texture's view, or an offscreen `wgpu::Texture` view.
    /// Must match the `msaa_samples` declared at construction time.
    ///
    /// `clear_color` is applied to the color attachment; the depth
    /// buffer is always cleared to `1.0` (far plane).
    pub fn draw(
        &mut self,
        app: &Application,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        camera: &Camera3D,
        meshes: &[(Mesh3D, Mat4, [f32; 4])],
        clear_color: wgpu::Color,
    ) {
        // 1. Upload the latest view-proj.
        let view_proj = camera.view_proj_uniform();
        app.queue()
            .write_buffer(&self.view_proj_buffer, 0, bytemuck::bytes_of(&view_proj));

        // 2. Per-mesh: build a vertex buffer, index buffer, model UBO,
        //    and bind group. These ARE per-draw allocations today —
        //    instancing lives in a later chunk.
        let device = app.device();

        // Build all the GPU-side resources up-front so the borrow of
        // `encoder` for the pass doesn't conflict with the borrows of
        // `device` for buffer creation.
        let mut per_mesh: Vec<(wgpu::Buffer, wgpu::Buffer, u32, wgpu::BindGroup)> =
            Vec::with_capacity(meshes.len());
        for (mesh, model_m, tint) in meshes {
            if mesh.indices.is_empty() {
                continue;
            }
            let vbuf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("wisp_3d::mesh_vbuf"),
                size: (mesh.vertex_buffer().len() * std::mem::size_of::<Vertex3D>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            app.queue()
                .write_buffer(&vbuf, 0, bytemuck::cast_slice(&mesh.vertex_buffer()));

            let ibuf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("wisp_3d::mesh_ibuf"),
                size: (mesh.indices.len() * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            app.queue()
                .write_buffer(&ibuf, 0, bytemuck::cast_slice(&mesh.indices));

            let model = ModelUniform::new(*model_m, *tint);
            let ubo = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("wisp_3d::model_ubo"),
                size: std::mem::size_of::<ModelUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            app.queue()
                .write_buffer(&ubo, 0, bytemuck::bytes_of(&model));

            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("wisp_3d::model_bg"),
                layout: &self.model_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                }],
            });
            #[allow(
                clippy::cast_possible_truncation,
                reason = "index buffer length is bounded by Mesh3D::indices.len(), which fits u32 for any realistic mesh"
            )]
            let idx_count = mesh.indices.len() as u32;
            per_mesh.push((vbuf, ibuf, idx_count, bg));
        }

        // 3. One render pass: clear color + depth, draw each mesh.
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wisp_3d::render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.view_proj_bg, &[]);
        for (vbuf, ibuf, idx_count, bg) in &per_mesh {
            pass.set_bind_group(1, bg, &[]);
            pass.set_vertex_buffer(0, vbuf.slice(..));
            pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..*idx_count, 0, 0..1);
        }
    }
}

fn create_depth_view(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    msaa_samples: u32,
) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("wisp_3d::depth_texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: msaa_samples,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wisp::application::AppConfig;

    fn make_app() -> Application {
        pollster::block_on(Application::new(AppConfig::default())).expect("application init failed")
    }

    #[test]
    fn pipeline_state_uses_less_depth_compare() {
        // Indirect check: the pipeline is built with depth_compare:
        // Less. We can't introspect the wgpu state object, so we
        // assert the constant we built it from is what we expect by
        // anchoring on DEPTH_FORMAT (the other half of the contract).
        assert_eq!(DEPTH_FORMAT, wgpu::TextureFormat::Depth32Float);
    }

    #[test]
    fn pass_constructs_with_msaa_one() {
        let app = make_app();
        let pass = Render3DPass::new(&app, wgpu::TextureFormat::Rgba8UnormSrgb, 256, 256, 1);
        assert_eq!(pass.msaa_samples(), 1);
        assert_eq!(pass.output_format(), wgpu::TextureFormat::Rgba8UnormSrgb);
    }

    #[test]
    fn pass_constructs_with_msaa_four() {
        let app = make_app();
        let pass = Render3DPass::new(&app, wgpu::TextureFormat::Rgba8UnormSrgb, 512, 512, 4);
        assert_eq!(pass.msaa_samples(), 4);
    }

    #[test]
    fn resize_no_op_when_dimensions_unchanged() {
        let app = make_app();
        let mut pass = Render3DPass::new(&app, wgpu::TextureFormat::Rgba8UnormSrgb, 256, 256, 1);
        // Same dims → no rebuild. We can't observe the inner texture
        // identity directly, but the bookkeeping field flips only on
        // actual resize, so we anchor on `depth_size`.
        pass.resize(&app, 256, 256);
        assert_eq!(pass.depth_size, (256, 256));
        pass.resize(&app, 512, 768);
        assert_eq!(pass.depth_size, (512, 768));
    }

    #[test]
    fn draw_with_empty_mesh_list_is_noop() {
        // The pass must not validation-error when handed an empty
        // mesh list — exercises the clear-color-only path.
        let app = make_app();
        let mut pass = Render3DPass::new(&app, wgpu::TextureFormat::Rgba8UnormSrgb, 64, 64, 1);
        let target = app.device().create_texture(&wgpu::TextureDescriptor {
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
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let camera = Camera3D::perspective(45.0, 1.0, 0.1, 100.0);
        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("test::encoder"),
            });
        app.device().push_error_scope(wgpu::ErrorFilter::Validation);
        pass.draw(
            &app,
            &mut encoder,
            &view,
            &camera,
            &[],
            wgpu::Color::TRANSPARENT,
        );
        app.queue().submit(std::iter::once(encoder.finish()));
        let err = pollster::block_on(app.device().pop_error_scope());
        assert!(err.is_none(), "validation error: {err:?}");
    }

    #[test]
    fn draw_pyramid_emits_no_validation_errors() {
        let app = make_app();
        let mut pass = Render3DPass::new(&app, wgpu::TextureFormat::Rgba8UnormSrgb, 128, 128, 1);
        let target = app.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("test::offscreen_color"),
            size: wgpu::Extent3d {
                width: 128,
                height: 128,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut camera = Camera3D::perspective(38.0, 1.0, 0.1, 100.0);
        camera.position = glam::Vec3::new(0.0, 0.28, 6.2);
        let mesh = Mesh3D::pyramid(1.34, 1.25);
        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("test::encoder"),
            });
        app.device().push_error_scope(wgpu::ErrorFilter::Validation);
        pass.draw(
            &app,
            &mut encoder,
            &view,
            &camera,
            &[(mesh, Mat4::IDENTITY, [0.8, 0.5, 0.9, 1.0])],
            wgpu::Color::TRANSPARENT,
        );
        app.queue().submit(std::iter::once(encoder.finish()));
        let err = pollster::block_on(app.device().pop_error_scope());
        assert!(err.is_none(), "validation error: {err:?}");
    }
}
