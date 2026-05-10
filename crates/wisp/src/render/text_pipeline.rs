//! Bitmap text pipeline — M0.15.
//!
//! One instance per glyph. All glyphs from all `Text` nodes that share a font
//! atlas batch into a single draw call. (Different fonts → different atlases →
//! one batch per font, in the order encountered during scene traversal.)

use std::collections::{HashMap, HashSet};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::blend::BlendMode;
use crate::render::blend_pipeline::BlendPipelineMap;
use crate::render::scene_walk::walk_visible_subtree;
use crate::scene::text::{Font, Text};
use crate::scene::{Node, NodeId, Stage};

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
    pipelines: BlendPipelineMap,
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

        let pipelines = BlendPipelineMap::new(|blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        });

        Self {
            pipelines,
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
        self.draw_subtree(app, pass, stage, stage.root(), &HashSet::new())
    }

    /// Subtree variant — see `SpritePipeline::draw_subtree`.
    pub(crate) fn draw_subtree(
        &self,
        app: &Application,
        pass: &mut wgpu::RenderPass<'_>,
        stage: &Stage,
        start: NodeId,
        exclude: &HashSet<NodeId>,
    ) -> (u32, u32) {
        let batches = collect_batches(stage, start, exclude);
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

            pass.set_pipeline(self.pipelines.get(batch.blend_mode));
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
    blend_mode: BlendMode,
    instances: Vec<TextInstance>,
}

fn collect_batches(stage: &Stage, start: NodeId, exclude: &HashSet<NodeId>) -> Vec<Batch> {
    type Key = (usize, BlendMode);
    let mut grouped: HashMap<Key, (crate::texture::Texture, BlendMode, Vec<TextInstance>)> =
        HashMap::new();
    let mut order: Vec<Key> = Vec::new();

    walk_visible_subtree(stage, start, exclude, |_id, node, world| {
        let container = node.container();
        if let Node::Text(text) = node {
            push_text_glyphs(
                text,
                container.blend_mode,
                world,
                container.alpha,
                &mut grouped,
                &mut order,
            );
        }
    });

    order
        .into_iter()
        .filter_map(|key| {
            grouped
                .remove(&key)
                .map(|(atlas, blend_mode, instances)| Batch {
                    atlas,
                    blend_mode,
                    instances,
                })
        })
        .collect()
}

fn push_text_glyphs(
    text: &Text,
    blend_mode: BlendMode,
    world: Mat4,
    parent_alpha: f32,
    grouped: &mut HashMap<
        (usize, BlendMode),
        (crate::texture::Texture, BlendMode, Vec<TextInstance>),
    >,
    order: &mut Vec<(usize, BlendMode)>,
) {
    let key = (text.font.atlas().id(), blend_mode);
    let atlas = text.font.atlas().clone();
    let entry = grouped.entry(key).or_insert_with(|| {
        order.push(key);
        (atlas, blend_mode, Vec::new())
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

        entry.2.push(TextInstance {
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
