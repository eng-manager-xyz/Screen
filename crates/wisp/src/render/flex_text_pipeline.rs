//! Late-pass textured-quad pipeline for [`FlexText`] scene nodes.
//!
//! Structurally identical to [`SpritePipeline`](super::sprite_pipeline)
//! — same shader, same instancing layout, same blend-mode cache —
//! but walks the scene for [`Node::FlexText`] instead of
//! [`Node::Sprite`] and is invoked by the renderer in the **fourth
//! render pass**, after [`TextPipeline`](super::text_pipeline). The
//! intent is for cosmic-text-rendered axis labels / legends / KPI
//! big numbers to paint on top of the chart's
//! [`Graphics`](crate::scene::Graphics) primitives instead of under
//! them.
//!
//! Kept as a separate file from `sprite_pipeline.rs` so the pass
//! order is self-documenting at the call site in
//! [`crate::render::Renderer::render_stage_fast`].

use std::collections::{HashMap, HashSet};

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::blend::BlendMode;
use crate::color::Color;
use crate::render::blend_pipeline::BlendPipelineMap;
use crate::render::scene_walk::walk_visible_subtree;
use crate::scene::{FlexText, Node, NodeId, Stage};
use crate::texture::Texture;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct FlexTextInstance {
    pub model: [[f32; 4]; 4],
    pub tint: [f32; 4],
    pub anchor: [f32; 2],
    pub _padding: [f32; 2],
}

const ATTR_LAYOUT: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
    0 => Float32x4,
    1 => Float32x4,
    2 => Float32x4,
    3 => Float32x4,
    4 => Float32x4,
    5 => Float32x2,
];

pub(crate) struct FlexTextPipeline {
    pipelines: BlendPipelineMap,
    texture_layout: wgpu::BindGroupLayout,
}

impl FlexTextPipeline {
    pub(crate) fn new(app: &Application, output_format: wgpu::TextureFormat) -> Self {
        let device = app.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp::flex_text"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/sprite.wgsl").into()),
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp::flex_text texture layout"),
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
            label: Some("wisp::flex_text pipeline layout"),
            bind_group_layouts: &[&texture_layout],
            push_constant_ranges: &[],
        });

        let pipelines = BlendPipelineMap::new(|blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("wisp::flex_text pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("main_vs"),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<FlexTextInstance>()
                            as wgpu::BufferAddress,
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

    /// Traverse `stage` in pre-order, accumulate flex-text instances
    /// grouped by `(texture id, blend mode)`, then draw one batch per
    /// group.
    ///
    /// Returns `(draw_calls, instances_drawn)`.
    pub(crate) fn draw_stage(
        &self,
        app: &Application,
        pass: &mut wgpu::RenderPass<'_>,
        stage: &Stage,
    ) -> (u32, u32) {
        self.draw_subtree(app, pass, stage, stage.root(), &HashSet::new())
    }

    /// Render only the subtree rooted at `start`. Nodes whose IDs
    /// are in `exclude` (and their descendants) are skipped.
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
        let mut nodes_drawn = 0u32;

        for batch in &batches {
            let count = u32::try_from(batch.instances.len()).expect("instance count fits in u32");
            nodes_drawn = nodes_drawn.saturating_add(count);
            let buffer = app
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("wisp::flex_text instances"),
                    contents: bytemuck::cast_slice(&batch.instances),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            let bg = app.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("wisp::flex_text texture bg"),
                layout: &self.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(batch.texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(batch.texture.sampler()),
                    },
                ],
            });

            pass.set_pipeline(self.pipelines.get(batch.blend_mode));
            pass.set_bind_group(0, &bg, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..6, 0..count);
            draw_calls += 1;
        }

        (draw_calls, nodes_drawn)
    }
}

struct Batch {
    texture: Texture,
    blend_mode: BlendMode,
    instances: Vec<FlexTextInstance>,
}

fn collect_batches(stage: &Stage, start: NodeId, exclude: &HashSet<NodeId>) -> Vec<Batch> {
    type Key = (usize, BlendMode);
    let mut grouped: HashMap<Key, (Texture, BlendMode, Vec<FlexTextInstance>)> = HashMap::new();
    let mut order: Vec<Key> = Vec::new();

    walk_visible_subtree(stage, start, exclude, |_id, node, world| {
        if let Node::FlexText(flex) = node {
            push_flex_text_instance(flex, world, &mut grouped, &mut order);
        }
    });

    order
        .into_iter()
        .filter_map(|key| {
            grouped
                .remove(&key)
                .map(|(texture, blend_mode, instances)| Batch {
                    texture,
                    blend_mode,
                    instances,
                })
        })
        .collect()
}

fn push_flex_text_instance(
    flex: &FlexText,
    world: Mat4,
    grouped: &mut HashMap<(usize, BlendMode), (Texture, BlendMode, Vec<FlexTextInstance>)>,
    order: &mut Vec<(usize, BlendMode)>,
) {
    let key = (flex.texture.id(), flex.container.blend_mode);
    let alpha_tint = with_premultiplied_alpha(flex.tint, flex.container.alpha);
    let entry = grouped.entry(key).or_insert_with(|| {
        order.push(key);
        (flex.texture.clone(), flex.container.blend_mode, Vec::new())
    });
    entry.2.push(FlexTextInstance {
        model: world.to_cols_array_2d(),
        tint: [alpha_tint.r, alpha_tint.g, alpha_tint.b, alpha_tint.a],
        anchor: [flex.anchor.x, flex.anchor.y],
        _padding: [0.0, 0.0],
    });
}

fn with_premultiplied_alpha(tint: Color, alpha: f32) -> Color {
    Color {
        r: tint.r,
        g: tint.g,
        b: tint.b,
        a: tint.a * alpha,
    }
}
