//! Generate an alpha-mask `RenderTexture` from a closed polygon
//! (M-DYN.1 / AUT-43, freehand-path variant).
//!
//! Sister pipeline to [`MaskTexturePipeline`](super::mask_texture::MaskTexturePipeline);
//! same uniform-buffered point-in-polygon as `PathClipPipeline` but
//! emits coverage directly instead of multiplying it into a sampled
//! foreground.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::texture::render_texture::RenderTexture;

const MAX_PATH_POINTS: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PathMaskTextureUniforms {
    count: u32,
    invert: u32,
    _pad: [u32; 2],
    points: [[f32; 4]; MAX_PATH_POINTS],
}

pub(crate) struct PathMaskTexturePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl PathMaskTexturePipeline {
    pub(crate) fn new(app: &Application, output_format: wgpu::TextureFormat) -> Self {
        let device = app.device();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp::path_mask_texture bg layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wisp::path_mask_texture pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp::path_mask_texture shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/path_mask_texture.wgsl").into(),
            ),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wisp::path_mask_texture pipeline"),
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

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    pub(crate) fn generate(
        &self,
        app: &Application,
        points: &[glam::Vec2],
        w: u32,
        h: u32,
        output_format: wgpu::TextureFormat,
    ) -> RenderTexture {
        let rt = RenderTexture::with_format(app, w, h, output_format);
        self.render_into(app, points, false, &rt);
        rt
    }

    pub(crate) fn render_into(
        &self,
        app: &Application,
        points: &[glam::Vec2],
        invert: bool,
        output: &RenderTexture,
    ) {
        let mut padded = [[0.0_f32; 4]; MAX_PATH_POINTS];
        let count = points.len().min(MAX_PATH_POINTS);
        for (slot, p) in padded.iter_mut().zip(points.iter()).take(count) {
            *slot = [p.x, p.y, 0.0, 0.0];
        }
        let count_u32 = u32::try_from(count).unwrap_or(0);

        let uniforms = PathMaskTextureUniforms {
            count: count_u32,
            invert: u32::from(invert),
            _pad: [0, 0],
            points: padded,
        };
        let buffer = app
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("wisp::path_mask_texture uniforms"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bg = app.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp::path_mask_texture bg"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp::path_mask_texture encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp::path_mask_texture pass"),
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
