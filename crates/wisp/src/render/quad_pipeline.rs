//! Textured-quad pipeline.
//!
//! M0.6 introduces this. M0.9 evolves it into the sprite batcher (instancing
//! and shared uniform buffer per batch).

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::color::Color;
use crate::texture::Texture;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct QuadUniforms {
    model: [[f32; 4]; 4],
    tint: [f32; 4],
}

pub(crate) struct QuadPipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
}

impl QuadPipeline {
    pub(crate) fn new(app: &Application, output_format: wgpu::TextureFormat) -> Self {
        let device = app.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp::quad"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/quad.wgsl").into()),
        });

        let uniforms_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp::quad uniforms layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp::quad texture layout"),
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wisp::quad pipeline layout"),
            bind_group_layouts: &[&uniforms_layout, &texture_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wisp::quad pipeline"),
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
            uniforms_layout,
            texture_layout,
        }
    }

    /// Draw a single textured quad. Allocates a uniform buffer + 2 bind groups
    /// per call — fine for M0.6 (one quad per frame). M0.9 batches.
    pub(crate) fn draw(
        &self,
        app: &Application,
        pass: &mut wgpu::RenderPass<'_>,
        texture: &Texture,
        model: Mat4,
        tint: Color,
    ) {
        let device = app.device();
        let uniforms = QuadUniforms {
            model: model.to_cols_array_2d(),
            tint: [tint.r, tint.g, tint.b, tint.a],
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wisp::quad uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let uniforms_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp::quad uniforms bg"),
            layout: &self.uniforms_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        let texture_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp::quad texture bg"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(texture.sampler()),
                },
            ],
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &uniforms_bg, &[]);
        pass.set_bind_group(1, &texture_bg, &[]);
        pass.draw(0..6, 0..1);
    }
}
