//! Generate an alpha-mask `RenderTexture` from a [`MaskShape`]
//! (M-DYN.1 / AUT-43).
//!
//! The output texture stores coverage as `(m, m, m, m)`: same value
//! in RGB and alpha so consumers can either alpha-multiply (sample
//! `.a`) or display as a grayscale silhouette. The mask is decoupled
//! from foreground sampling — composition happens in a downstream
//! pipeline.
//!
//! Architectural rationale (`AUT-43`): this primitive owns *only*
//! coverage. Privacy blur, redaction, and spotlight will compose with
//! these textures via separate paths. The cache (`AUT-44`) layers on
//! top to avoid regenerating identical masks every frame.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::scene::clip::MaskShape;
use crate::texture::render_texture::RenderTexture;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaskTextureUniforms {
    center: [f32; 2],
    half_extents: [f32; 2],
    radius: f32,
    aa: f32,
    invert: f32,
    shape_kind: f32,
}

pub(crate) struct MaskTexturePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl MaskTexturePipeline {
    pub(crate) fn new(app: &Application, output_format: wgpu::TextureFormat) -> Self {
        let device = app.device();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp::mask_texture bg layout"),
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
            label: Some("wisp::mask_texture pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp::mask_texture shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/mask_texture.wgsl").into(),
            ),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wisp::mask_texture pipeline"),
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

    /// Generate a mask texture for `shape` at `(w, h)` and return it.
    pub(crate) fn generate(
        &self,
        app: &Application,
        shape: MaskShape,
        w: u32,
        h: u32,
        output_format: wgpu::TextureFormat,
    ) -> RenderTexture {
        let rt = RenderTexture::with_format(app, w, h, output_format);
        self.render_into(app, shape, false, &rt);
        rt
    }

    /// Same as [`Self::generate`] but writes into a caller-owned RT
    /// (used by the cache so it can recycle textures).
    pub(crate) fn render_into(
        &self,
        app: &Application,
        shape: MaskShape,
        invert: bool,
        output: &RenderTexture,
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
                let r = radius.max(0.0);
                (center.x, center.y, r, r, r, 0.0)
            }
            MaskShape::Ellipse {
                center,
                half_extents,
            } => {
                let hx = half_extents.x.max(0.0);
                let hy = half_extents.y.max(0.0);
                (center.x, center.y, hx, hy, 0.0, 1.0)
            }
        };

        let w_f = f32::from(u16::try_from(output.width().min(u32::from(u16::MAX))).unwrap_or(1));
        let h_f = f32::from(u16::try_from(output.height().min(u32::from(u16::MAX))).unwrap_or(1));
        let aa = 2.0 / w_f.min(h_f).max(1.0);

        let uniforms = MaskTextureUniforms {
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
                label: Some("wisp::mask_texture uniforms"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bg = app.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp::mask_texture bg"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp::mask_texture encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp::mask_texture pass"),
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
