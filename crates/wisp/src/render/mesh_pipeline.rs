//! Mesh pipeline — perspective-rotated textured quad (M0.19).

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec4};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::scene::{Node, Stage};
use crate::texture::Texture;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct MeshInstance {
    pub model: [[f32; 4]; 4],
    pub tint: [f32; 4],
    pub rotation_y: f32,
    pub persp_strength: f32,
    pub _pad: [f32; 2],
}

const ATTR_LAYOUT: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
    0 => Float32x4,
    1 => Float32x4,
    2 => Float32x4,
    3 => Float32x4,
    4 => Float32x4,
    5 => Float32,
    6 => Float32,
];

pub(crate) struct MeshPipeline {
    pipeline: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
}

impl MeshPipeline {
    pub(crate) fn new(app: &Application, output_format: wgpu::TextureFormat) -> Self {
        let device = app.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp::mesh_perspective"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/mesh_perspective.wgsl").into(),
            ),
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp::mesh texture layout"),
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
            label: Some("wisp::mesh pipeline layout"),
            bind_group_layouts: &[&texture_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wisp::mesh pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("main_vs"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshInstance>() as wgpu::BufferAddress,
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

    /// Walk `stage`, batch mesh instances per shared texture, draw.
    pub(crate) fn draw_stage(
        &self,
        app: &Application,
        pass: &mut wgpu::RenderPass<'_>,
        stage: &Stage,
    ) -> (u32, u32) {
        let batches = collect_batches(stage);
        let mut draw_calls = 0u32;
        let mut meshes = 0u32;

        for batch in &batches {
            if batch.instances.is_empty() {
                continue;
            }
            let count = u32::try_from(batch.instances.len()).expect("mesh count fits in u32");
            meshes = meshes.saturating_add(count);

            let buffer = app
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("wisp::mesh instances"),
                    contents: bytemuck::cast_slice(&batch.instances),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            let bg = app.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("wisp::mesh texture bg"),
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

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..6, 0..count);
            draw_calls += 1;
        }

        (draw_calls, meshes)
    }
}

struct Batch {
    texture: Texture,
    instances: Vec<MeshInstance>,
}

fn collect_batches(stage: &Stage) -> Vec<Batch> {
    let mut grouped: HashMap<usize, (Texture, Vec<MeshInstance>)> = HashMap::new();
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

        if let Node::Mesh(mesh) = node {
            let key = mesh.texture.id();
            let entry = grouped.entry(key).or_insert_with(|| {
                order.push(key);
                (mesh.texture.clone(), Vec::new())
            });
            entry.1.push(MeshInstance {
                model: world.to_cols_array_2d(),
                tint: [
                    mesh.tint.r,
                    mesh.tint.g,
                    mesh.tint.b,
                    mesh.tint.a * container.alpha,
                ],
                rotation_y: mesh.rotation_y,
                persp_strength: mesh.perspective_strength,
                _pad: [0.0, 0.0],
            });
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
                .map(|(texture, instances)| Batch { texture, instances })
        })
        .collect()
}

fn mat3_to_mat4(m: Mat3) -> Mat4 {
    Mat4::from_cols(
        Vec4::new(m.x_axis.x, m.x_axis.y, 0.0, 0.0),
        Vec4::new(m.y_axis.x, m.y_axis.y, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(m.z_axis.x, m.z_axis.y, 0.0, 1.0),
    )
}
