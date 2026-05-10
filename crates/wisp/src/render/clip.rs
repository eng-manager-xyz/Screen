//! Clip pipeline — apply a [`MaskShape`] to a foreground
//! `RenderTexture` and write the masked result.
//!
//! Used by the auto-dispatch path in
//! [`Renderer::render_stage`](crate::render::Renderer::render_stage)
//! when a container has a
//! [`Container::clip`](crate::scene::container::Container::clip) set:
//! the subtree is rendered into a foreground RT, this pipeline samples
//! the foreground and multiplies in the SDF-based mask alpha, and the
//! result is composited back onto the parent's destination.
//!
//! Today: only [`MaskShape::RoundedRect`]. Later issues add more shape
//! variants; the same pipeline (uniform-driven SDF) handles them by
//! switching the SDF function in the WGSL.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::scene::clip::MaskShape;
use crate::texture::render_texture::RenderTexture;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ClipUniforms {
    center: [f32; 2],
    half_extents: [f32; 2],
    radius: f32,
    aa: f32,
    invert: f32,
    shape_kind: f32,
}

pub(crate) struct ClipPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl ClipPipeline {
    pub(crate) fn new(app: &Application, output_format: wgpu::TextureFormat) -> Self {
        let device = app.device();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp::clip bg layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wisp::clip pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp::clip shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/clip.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wisp::clip pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("main_vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("main_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("wisp::clip sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
        }
    }

    /// Sample `foreground` and write the masked result into `output`.
    /// `output_dims` lets us compute a 1-pixel anti-alias band in the
    /// shader (`aa = 2/min(w, h)` in NDC units).
    pub(crate) fn apply(
        &self,
        app: &Application,
        shape: MaskShape,
        foreground: &RenderTexture,
        output: &RenderTexture,
    ) {
        self.apply_with_invert(app, shape, foreground, output, false);
    }

    fn apply_with_invert(
        &self,
        app: &Application,
        shape: MaskShape,
        foreground: &RenderTexture,
        output: &RenderTexture,
        invert: bool,
    ) {
        let (cx, cy, hx, hy, radius, shape_kind) = match shape {
            MaskShape::Rect { rect } => {
                let cx = rect.min.x + rect.size.x * 0.5;
                let cy = rect.min.y + rect.size.y * 0.5;
                let hx = (rect.size.x * 0.5).max(0.0);
                let hy = (rect.size.y * 0.5).max(0.0);
                (cx, cy, hx, hy, 0.0, 0.0)
            }
            MaskShape::RoundedRect { rect, radius } => {
                let cx = rect.min.x + rect.size.x * 0.5;
                let cy = rect.min.y + rect.size.y * 0.5;
                let hx = (rect.size.x * 0.5).max(0.0);
                let hy = (rect.size.y * 0.5).max(0.0);
                let r = radius.clamp(0.0, hx.min(hy));
                (cx, cy, hx, hy, r, 0.0)
            }
            MaskShape::Circle { center, radius } => {
                // Rounded-rect SDF degenerates to circle when
                // half_extents == radius == r. (The shader formula
                // becomes length(max(|p|, 0)) - r = length(p) - r.)
                let r = radius.max(0.0);
                (center.x, center.y, r, r, r, 0.0)
            }
            MaskShape::Ellipse {
                center,
                half_extents,
            } => {
                let hx = half_extents.x.max(0.0);
                let hy = half_extents.y.max(0.0);
                // Radius is unused by the ellipse SDF branch; pass 0.
                (center.x, center.y, hx, hy, 0.0, 1.0)
            }
        };

        let w_f = f32::from(u16::try_from(output.width().min(u32::from(u16::MAX))).unwrap_or(1));
        let h_f = f32::from(u16::try_from(output.height().min(u32::from(u16::MAX))).unwrap_or(1));
        let aa = 2.0 / w_f.min(h_f).max(1.0);

        let uniforms = ClipUniforms {
            center: [cx, cy],
            half_extents: [hx, hy],
            radius,
            aa,
            invert: if invert { 1.0 } else { 0.0 },
            shape_kind,
        };
        let buffer = app
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("wisp::clip uniforms"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bg = app.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp::clip bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(foreground.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp::clip encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp::clip pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.draw(0..3, 0..1);
        }
        app.queue().submit(std::iter::once(encoder.finish()));
    }
}
