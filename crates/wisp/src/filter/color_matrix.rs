//! `ColorMatrixFilter` — 4×5 RGBA matrix multiplication.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::filter::{Filter, FilterContext};
use crate::texture::render_texture::RenderTexture;

/// Apply a 4×5 color transform: `out = M · [r, g, b, a, 1]`.
///
/// Stored as four 4-vectors of multiplications and a 4-vector of constants —
/// shader-friendly layout. Use named constructors for common operations.
#[derive(Debug, Clone, Copy)]
pub struct ColorMatrixFilter {
    /// Multipliers: rows for [R, G, B, A] each indexed [r→, g→, b→, a→].
    pub multipliers: [[f32; 4]; 4],
    /// Per-channel constant adds (the 5th column of the 4×5 matrix).
    pub constants: [f32; 4],
}

impl Default for ColorMatrixFilter {
    fn default() -> Self {
        Self::identity()
    }
}

impl ColorMatrixFilter {
    /// Identity transform — output equals input.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            multipliers: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            constants: [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Grayscale via Rec.709 luminance weights.
    #[must_use]
    pub const fn grayscale() -> Self {
        let r = 0.2126;
        let g = 0.7152;
        let b = 0.0722;
        Self {
            multipliers: [
                [r, g, b, 0.0],
                [r, g, b, 0.0],
                [r, g, b, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            constants: [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Brightness multiplier (1.0 = no change).
    #[must_use]
    pub const fn brightness(scale: f32) -> Self {
        Self {
            multipliers: [
                [scale, 0.0, 0.0, 0.0],
                [0.0, scale, 0.0, 0.0],
                [0.0, 0.0, scale, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            constants: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ColorMatrixUniforms {
    row_r: [f32; 4],
    row_g: [f32; 4],
    row_b: [f32; 4],
    row_a: [f32; 4],
    constants: [f32; 4],
}

impl Filter for ColorMatrixFilter {
    fn passes(&self) -> u32 {
        1
    }

    fn render_pass(
        &self,
        ctx: &mut FilterContext<'_>,
        input: &RenderTexture,
        output: &RenderTexture,
        _pass: u32,
    ) {
        run_color_matrix_pass(ctx.app, ctx.encoder, input, output, self);
    }
}

fn run_color_matrix_pass(
    app: &Application,
    encoder: &mut wgpu::CommandEncoder,
    input: &RenderTexture,
    output: &RenderTexture,
    filter: &ColorMatrixFilter,
) {
    let device = app.device();
    let cache = pipeline(app, output.format());

    let uniforms = ColorMatrixUniforms {
        row_r: filter.multipliers[0],
        row_g: filter.multipliers[1],
        row_b: filter.multipliers[2],
        row_a: filter.multipliers[3],
        constants: filter.constants,
    };
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("wisp::color_matrix uniforms"),
        contents: bytemuck::bytes_of(&uniforms),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wisp::color_matrix uniform bg"),
        layout: &cache.uniform_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });
    let texture_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wisp::color_matrix texture bg"),
        layout: &cache.texture_layout,
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
        label: Some("wisp::color_matrix pass"),
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
    pass.set_pipeline(&cache.pipeline);
    pass.set_bind_group(0, &uniform_bg, &[]);
    pass.set_bind_group(1, &texture_bg, &[]);
    pass.draw(0..3, 0..1);
}

struct ColorMatrixCache {
    pipeline: wgpu::RenderPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
}

fn pipeline(app: &Application, format: wgpu::TextureFormat) -> ColorMatrixCache {
    let device = app.device();
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("wisp::color_matrix"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../shaders/filter_color_matrix.wgsl").into(),
        ),
    });

    let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("wisp::color_matrix uniform layout"),
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
        label: Some("wisp::color_matrix texture layout"),
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

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("wisp::color_matrix pipeline layout"),
        bind_group_layouts: &[&uniform_layout, &texture_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("wisp::color_matrix pipeline"),
        layout: Some(&layout),
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

    ColorMatrixCache {
        pipeline,
        uniform_layout,
        texture_layout,
    }
}
