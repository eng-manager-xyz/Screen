//! Bitmap text pipeline — M0.15.
//!
//! One instance per glyph. All glyphs from all `Text` nodes that share a font
//! atlas batch into a single draw call. (Different fonts → different atlases →
//! one batch per font, in the order encountered during scene traversal.)

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::scene::text::{Font, Text};
use crate::scene::{Node, Stage};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct TextInstance {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub uv_rect: [f32; 4],
}

const ATTR_LAYOUT: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
    0 => Float32x4,
    1 => Float32x4,
    2 => Float32x4,
    3 => Float32x4,
    4 => Float32x4,
    5 => Float32x4,
];

pub(crate) struct TextPipeline {
    pipeline: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
}

impl TextPipeline {
    pub(crate) fn new(app: &Application, output_format: wgpu::TextureFormat) -> Self {
        let device = app.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp::text"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/text.wgsl").into()),
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp::text atlas layout"),
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
            label: Some("wisp::text pipeline layout"),
            bind_group_layouts: &[&texture_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wisp::text pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("main_vs"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TextInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &ATTR_LAYOUT,
                }],
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
            texture_layout,
        }
    }

    /// Walk `stage`, batch glyph instances per font atlas, draw.
    ///
    /// Returns `(draw_calls, glyph_count)`. Each font atlas costs one batch.
    pub(crate) fn draw_stage(
        &self,
        app: &Application,
        pass: &mut wgpu::RenderPass<'_>,
        stage: &Stage,
    ) -> (u32, u32) {
        let batches = collect_batches(stage);
        let mut draw_calls = 0u32;
        let mut glyphs = 0u32;

        for batch in &batches {
            if batch.instances.is_empty() {
                continue;
            }
            let count = u32::try_from(batch.instances.len()).expect("glyph count fits in u32");
            glyphs = glyphs.saturating_add(count);
            let buffer = app
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("wisp::text instances"),
                    contents: bytemuck::cast_slice(&batch.instances),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            let bg = app.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("wisp::text atlas bg"),
                layout: &self.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(batch.atlas.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(batch.atlas.sampler()),
                    },
                ],
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..6, 0..count);
            draw_calls += 1;
        }

        (draw_calls, glyphs)
    }
}

struct Batch {
    atlas: crate::texture::Texture,
    instances: Vec<TextInstance>,
}

fn collect_batches(stage: &Stage) -> Vec<Batch> {
    let mut grouped: HashMap<usize, (crate::texture::Texture, Vec<TextInstance>)> = HashMap::new();
    let mut order: Vec<usize> = Vec::new();

    let mut stack: Vec<(crate::scene::NodeId, Mat4)> = vec![(stage.root(), Mat4::IDENTITY)];
    while let Some((id, parent_world)) = stack.pop() {
        let Some(node) = stage.get(id) else {
            continue;
        };
        let container = node.container();
        if !container.visible {
            continue;
        }
        let local = mat3_to_mat4(container.transform.to_mat3());
        let world = parent_world * local;

        if let Node::Text(text) = node {
            push_text_glyphs(text, world, container.alpha, &mut grouped, &mut order);
        }

        for child in container.children().rev().collect::<Vec<_>>() {
            stack.push((child, world));
        }
    }

    order
        .into_iter()
        .filter_map(|key| {
            grouped
                .remove(&key)
                .map(|(atlas, instances)| Batch { atlas, instances })
        })
        .collect()
}

fn push_text_glyphs(
    text: &Text,
    world: Mat4,
    parent_alpha: f32,
    grouped: &mut HashMap<usize, (crate::texture::Texture, Vec<TextInstance>)>,
    order: &mut Vec<usize>,
) {
    let key = text.font.atlas().id();
    let atlas = text.font.atlas().clone();
    let entry = grouped.entry(key).or_insert_with(|| {
        order.push(key);
        (atlas, Vec::new())
    });

    let cell_pixels = f32_from_u32(text.font.cell_pixels());
    let glyph_size = text.cell_size * cell_pixels;
    let advance = glyph_size;
    let line_height = glyph_size * 1.25; // leading

    let mut cursor_x = 0.0f32;
    let mut cursor_y = 0.0f32;

    for c in text.content.chars() {
        if c == '\n' {
            cursor_x = 0.0;
            cursor_y -= line_height;
            continue;
        }

        let Some(glyph) = font_glyph(&text.font, c) else {
            cursor_x += advance;
            continue;
        };

        // Place a unit-quad ([-1,+1]^2) at the glyph's center, scaled to glyph size.
        let center_x = cursor_x + glyph_size * 0.5;
        let center_y = cursor_y - glyph_size * 0.5;
        let model = world
            * Mat4::from_translation(Vec3::new(center_x, center_y, 0.0))
            * Mat4::from_scale(Vec3::new(glyph_size * 0.5, glyph_size * 0.5, 1.0));

        entry.1.push(TextInstance {
            model: model.to_cols_array_2d(),
            color: [
                text.color.r,
                text.color.g,
                text.color.b,
                text.color.a * parent_alpha,
            ],
            uv_rect: [glyph.u_min, glyph.v_min, glyph.u_max, glyph.v_max],
        });

        cursor_x += advance;
    }
}

fn font_glyph(font: &Font, c: char) -> Option<crate::scene::text::GlyphMetrics> {
    font.glyph(c)
}

fn f32_from_u32(v: u32) -> f32 {
    // Cell pixel counts are tiny (8 today). Lossless within u23.
    #[allow(
        clippy::cast_precision_loss,
        reason = "atlas cell pixels fit easily in f32 mantissa (8 today, max 1024)"
    )]
    {
        v as f32
    }
}

fn mat3_to_mat4(m: Mat3) -> Mat4 {
    Mat4::from_cols(
        Vec4::new(m.x_axis.x, m.x_axis.y, 0.0, 0.0),
        Vec4::new(m.y_axis.x, m.y_axis.y, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(m.z_axis.x, m.z_axis.y, 0.0, 1.0),
    )
}
