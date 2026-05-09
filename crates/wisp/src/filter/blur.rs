//! `BlurFilter` — two-pass separable Gaussian blur.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::filter::{Filter, FilterContext};
use crate::texture::render_texture::RenderTexture;

/// Separable Gaussian blur, 9-tap horizontal + 9-tap vertical.
///
/// `radius` is the texel-space spread (`1.0` ≈ subtle, `8.0` heavy). `passes`
/// returns 2 — the orchestrator runs us once with `pass=0` (horizontal) and
/// once with `pass=1` (vertical) using ping-pong render targets.
#[derive(Debug, Clone, Copy)]
pub struct BlurFilter {
    /// Texel-space spread per pass.
    pub radius: f32,
}

impl BlurFilter {
    /// Construct a blur with an explicit radius.
    #[must_use]
    pub const fn new(radius: f32) -> Self {
        Self { radius }
    }
}

impl Default for BlurFilter {
    fn default() -> Self {
        Self { radius: 4.0 }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BlurUniforms {
    direction: [f32; 2],
    radius: f32,
    _pad: f32,
}

impl Filter for BlurFilter {
    fn passes(&self) -> u32 {
        2
    }

    fn render_pass(
        &self,
        ctx: &mut FilterContext<'_>,
        input: &RenderTexture,
        output: &RenderTexture,
        pass: u32,
    ) {
        let direction = if pass == 0 {
            [1.0_f32, 0.0]
        } else {
            [0.0, 1.0]
        };
        run_blur_pass(ctx.app, ctx.encoder, input, output, direction, self.radius);
    }
}

pub(crate) fn run_blur_pass(
    app: &Application,
    encoder: &mut wgpu::CommandEncoder,
    input: &RenderTexture,
    output: &RenderTexture,
    direction: [f32; 2],
    radius: f32,
) {
    let device = app.device();
    let pipeline_cache = filter_pipeline(app, output.format());

    let uniforms = BlurUniforms {
        direction,
        radius,
        _pad: 0.0,
    };
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("wisp::blur uniforms"),
        contents: bytemuck::bytes_of(&uniforms),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wisp::blur uniform bg"),
        layout: &pipeline_cache.uniform_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let texture_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wisp::blur texture bg"),
        layout: &pipeline_cache.texture_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(input.view()),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(input.sampler()),
            },
        ],
    });

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("wisp::blur pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: output.view(),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    pass.set_pipeline(&pipeline_cache.pipeline);
    pass.set_bind_group(0, &uniform_bg, &[]);
    pass.set_bind_group(1, &texture_bg, &[]);
    pass.draw(0..3, 0..1);
}

/// Pipeline cache. M0.16 builds a fresh one each call; M1+ can promote it
/// to a `Renderer` field if perf demands.
struct BlurPipelineCache {
    pipeline: wgpu::RenderPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
}

fn filter_pipeline(app: &Application, format: wgpu::TextureFormat) -> BlurPipelineCache {
    let device = app.device();
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("wisp::filter_blur"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/filter_blur.wgsl").into()),
    });

    let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("wisp::blur uniforms layout"),
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
    let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("wisp::blur texture layout"),
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
        label: Some("wisp::blur pipeline layout"),
        bind_group_layouts: &[&uniform_layout, &texture_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("wisp::blur pipeline"),
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
                format,
                blend: None,
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

    BlurPipelineCache {
        pipeline,
        uniform_layout,
        texture_layout,
    }
}
