//! Path clip pipeline — rasterize a closed-polygon mask for a
//! `foreground` RT (M-MASK / AUT-35 freehand path mask).
//!
//! Uses a uniform-buffered point list (up to 32 vertices for V1) and
//! a fragment-shader winding-number test (Jordan curve theorem). No
//! tessellation required, no SDF — just the classic crossings-test
//! point-in-polygon at every pixel. Works for any simple polygon
//! (convex or concave); doesn't handle self-intersecting paths
//! cleanly, but those aren't on the freehand-mask UX path.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::texture::render_texture::RenderTexture;

/// Maximum points in a path mask. Mirrors `MAX_POINTS` in
/// `path_clip.wgsl`.
pub(crate) const MAX_PATH_POINTS: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PathClipUniforms {
    count: u32,
    invert: u32,
    _pad: [u32; 2],
    /// Each point stored as a `vec4` so the layout matches WGSL's
    /// `array<vec4<f32>, N>` (stricter alignment than `vec2`). Only
    /// `.xy` is read by the shader; `.zw` are unused padding.
    points: [[f32; 4]; MAX_PATH_POINTS],
}

pub(crate) struct PathClipPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl PathClipPipeline {
    pub(crate) fn new(app: &Application, output_format: wgpu::TextureFormat) -> Self {
        let device = app.device();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp::path_clip bg layout"),
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
            label: Some("wisp::path_clip pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp::path_clip shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/path_clip.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wisp::path_clip pipeline"),
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
            label: Some("wisp::path_clip sampler"),
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

    /// Sample `foreground` and write `foreground × point_in_polygon`
    /// into `output`. `points` is the closed polygon in NDC; up to
    /// `MAX_PATH_POINTS` entries are honored, the rest are ignored.
    pub(crate) fn apply(
        &self,
        app: &Application,
        points: &[glam::Vec2],
        invert: bool,
        foreground: &RenderTexture,
        output: &RenderTexture,
    ) {
        let mut padded = [[0.0_f32; 4]; MAX_PATH_POINTS];
        let count = points.len().min(MAX_PATH_POINTS);
        for (slot, p) in padded.iter_mut().zip(points.iter()).take(count) {
            *slot = [p.x, p.y, 0.0, 0.0];
        }
        let count_u32 = u32::try_from(count).unwrap_or(0);

        let uniforms = PathClipUniforms {
            count: count_u32,
            invert: u32::from(invert),
            _pad: [0, 0],
            points: padded,
        };
        let buffer = app
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("wisp::path_clip uniforms"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bg = app.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp::path_clip bg"),
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
                label: Some("wisp::path_clip encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp::path_clip pass"),
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
