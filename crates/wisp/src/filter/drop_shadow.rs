//! `DropShadowFilter` — alpha extract → blur → composite under source.
//!
//! Four logical passes implemented inside a single `Filter::render_pass`
//! invocation (we manage our own scratch render targets — `scratch_a` and
//! `scratch_b`):
//!   1. Extract the source alpha, offset by `offset` and tinted with `color`,
//!      into the first scratch.
//!   2. Horizontal Gaussian blur on the first scratch into the second.
//!   3. Vertical Gaussian blur back into the first scratch (now the blurred
//!      shadow).
//!   4. Composite source over the blurred shadow into `output`.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::color::Color;
use crate::filter::{Filter, FilterContext};
use crate::texture::render_texture::RenderTexture;

/// Drop-shadow post-process.
#[derive(Debug, Clone, Copy)]
pub struct DropShadowFilter {
    /// Texel offset of the shadow relative to the source.
    pub offset: glam::Vec2,
    /// Gaussian blur radius applied to the shadow.
    pub blur: f32,
    /// Shadow color (RGB tints the shadow; alpha multiplies the source alpha).
    pub color: Color,
}

impl Default for DropShadowFilter {
    fn default() -> Self {
        Self {
            offset: glam::Vec2::new(4.0, 4.0),
            blur: 6.0,
            color: Color::rgba(0.0, 0.0, 0.0, 0.5),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ExtractUniforms {
    offset: [f32; 2],
    _pad: [f32; 2],
    color: [f32; 4],
}

impl Filter for DropShadowFilter {
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
        let app = ctx.app;
        let format = output.format();
        let scratch_a = RenderTexture::with_format(app, input.width(), input.height(), format);
        let scratch_b = RenderTexture::with_format(app, input.width(), input.height(), format);

        run_extract(app, ctx.encoder, input, &scratch_a, self.offset, self.color);
        super::blur::run_blur_pass(
            app,
            ctx.encoder,
            &scratch_a,
            &scratch_b,
            [1.0, 0.0],
            self.blur,
        );
        super::blur::run_blur_pass(
            app,
            ctx.encoder,
            &scratch_b,
            &scratch_a,
            [0.0, 1.0],
            self.blur,
        );
        run_composite(app, ctx.encoder, &scratch_a, input, output);
    }
}

fn run_extract(
    app: &Application,
    encoder: &mut wgpu::CommandEncoder,
    input: &RenderTexture,
    output: &RenderTexture,
    offset: glam::Vec2,
    color: Color,
) {
    let device = app.device();
    let cache = extract_pipeline(app, output.format());

    let uniforms = ExtractUniforms {
        offset: [offset.x, offset.y],
        _pad: [0.0, 0.0],
        color: [color.r, color.g, color.b, color.a],
    };
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("wisp::ds_extract uniforms"),
        contents: bytemuck::bytes_of(&uniforms),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wisp::ds_extract uniform bg"),
        layout: &cache.uniform_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });
    let texture_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wisp::ds_extract texture bg"),
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
        label: Some("wisp::ds_extract pass"),
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

fn run_composite(
    app: &Application,
    encoder: &mut wgpu::CommandEncoder,
    shadow: &RenderTexture,
    source: &RenderTexture,
    output: &RenderTexture,
) {
    let device = app.device();
    let cache = composite_pipeline(app, output.format());

    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wisp::ds_composite bg"),
        layout: &cache.bind_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(shadow.view()),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(source.view()),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(source.sampler()),
            },
        ],
    });

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("wisp::ds_composite pass"),
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
    pass.set_bind_group(0, &bg, &[]);
    pass.draw(0..3, 0..1);
}

struct ExtractCache {
    pipeline: wgpu::RenderPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
}

fn extract_pipeline(app: &Application, format: wgpu::TextureFormat) -> ExtractCache {
    let device = app.device();
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("wisp::ds_extract"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../shaders/filter_drop_shadow_extract.wgsl").into(),
        ),
    });

    let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("wisp::ds_extract uniform layout"),
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
        label: Some("wisp::ds_extract texture layout"),
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
        label: Some("wisp::ds_extract pipeline layout"),
        bind_group_layouts: &[&uniform_layout, &texture_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("wisp::ds_extract pipeline"),
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

    ExtractCache {
        pipeline,
        uniform_layout,
        texture_layout,
    }
}

struct CompositeCache {
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
}

fn composite_pipeline(app: &Application, format: wgpu::TextureFormat) -> CompositeCache {
    let device = app.device();
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("wisp::ds_composite"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../shaders/filter_drop_shadow_composite.wgsl").into(),
        ),
    });

    let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("wisp::ds_composite layout"),
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
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("wisp::ds_composite pipeline layout"),
        bind_group_layouts: &[&bind_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("wisp::ds_composite pipeline"),
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

    CompositeCache {
        pipeline,
        bind_layout,
    }
}
