//! Instanced sprite pipeline — M0.9.
//!
//! Per-instance attributes: 4×4 model matrix (4 × `Float32x4`), tint
//! (`Float32x4`), anchor (`Float32x2`). Six vertices per instance form a
//! unit-quad in `[0, 1]²`; the vertex shader subtracts `anchor` then applies
//! the model.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec4};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::blend::BlendMode;
use crate::color::Color;
use crate::render::blend_pipeline::BlendPipelineMap;
use crate::scene::{Node, Sprite, Stage};
use crate::texture::Texture;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct SpriteInstance {
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

pub(crate) struct SpritePipeline {
    pipelines: BlendPipelineMap,
    texture_layout: wgpu::BindGroupLayout,
}

impl SpritePipeline {
    pub(crate) fn new(app: &Application, output_format: wgpu::TextureFormat) -> Self {
        let device = app.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp::sprite"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/sprite.wgsl").into()),
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp::sprite texture layout"),
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
            label: Some("wisp::sprite pipeline layout"),
            bind_group_layouts: &[&texture_layout],
            push_constant_ranges: &[],
        });

        // Build one pipeline per native blend mode — eight in total.
        // The vertex/fragment/layout setup is shared; only the
        // `BlendState` differs.
        let pipelines = BlendPipelineMap::new(|blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("wisp::sprite pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("main_vs"),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
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

    /// Traverse `stage` in pre-order, accumulate sprite instances grouped by
    /// `(texture id, blend mode)`, then draw one batch per group.
    ///
    /// Returns the number of draw calls emitted and total sprites rendered.
    pub(crate) fn draw_stage(
        &self,
        app: &Application,
        pass: &mut wgpu::RenderPass<'_>,
        stage: &Stage,
    ) -> (u32, u32) {
        let batches = collect_batches(stage);
        let mut draw_calls = 0u32;
        let mut sprites_drawn = 0u32;

        for batch in &batches {
            let count = u32::try_from(batch.instances.len()).expect("instance count fits in u32");
            sprites_drawn = sprites_drawn.saturating_add(count);
            let buffer = app
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("wisp::sprite instances"),
                    contents: bytemuck::cast_slice(&batch.instances),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            let bg = app.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("wisp::sprite texture bg"),
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

        (draw_calls, sprites_drawn)
    }
}

struct Batch {
    texture: Texture,
    blend_mode: BlendMode,
    instances: Vec<SpriteInstance>,
}

fn collect_batches(stage: &Stage) -> Vec<Batch> {
    type Key = (usize, BlendMode);
    let mut grouped: HashMap<Key, (Texture, BlendMode, Vec<SpriteInstance>)> = HashMap::new();
    let mut order: Vec<Key> = Vec::new();

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

        if let Node::Sprite(sprite) = node {
            push_sprite_instance(sprite, world, &mut grouped, &mut order);
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
                .map(|(texture, blend_mode, instances)| Batch {
                    texture,
                    blend_mode,
                    instances,
                })
        })
        .collect()
}

fn push_sprite_instance(
    sprite: &Sprite,
    world: Mat4,
    grouped: &mut HashMap<(usize, BlendMode), (Texture, BlendMode, Vec<SpriteInstance>)>,
    order: &mut Vec<(usize, BlendMode)>,
) {
    let key = (sprite.texture.id(), sprite.container.blend_mode);
    let alpha_tint = with_premultiplied_alpha(sprite.tint, sprite.container.alpha);
    let entry = grouped.entry(key).or_insert_with(|| {
        order.push(key);
        (
            sprite.texture.clone(),
            sprite.container.blend_mode,
            Vec::new(),
        )
    });
    entry.2.push(SpriteInstance {
        model: world.to_cols_array_2d(),
        tint: [alpha_tint.r, alpha_tint.g, alpha_tint.b, alpha_tint.a],
        anchor: [sprite.anchor.x, sprite.anchor.y],
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

fn mat3_to_mat4(m: Mat3) -> Mat4 {
    Mat4::from_cols(
        Vec4::new(m.x_axis.x, m.x_axis.y, 0.0, 0.0),
        Vec4::new(m.y_axis.x, m.y_axis.y, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(m.z_axis.x, m.z_axis.y, 0.0, 1.0),
    )
}
