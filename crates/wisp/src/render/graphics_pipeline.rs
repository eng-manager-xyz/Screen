//! Graphics primitive pipeline — M0.12 (rect + rounded rect via shared SDF shader).
//!
//! All graphics primitives across all `Graphics` nodes batch into a single
//! draw call when the scene fits in one instance buffer.

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec4};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::scene::graphics::{Fill, Primitive};
use crate::scene::{Node, Stage};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct GraphicsInstance {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub half_extents: [f32; 2],
    pub radius: f32,
    pub _padding: f32,
}

const ATTR_LAYOUT: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
    0 => Float32x4,
    1 => Float32x4,
    2 => Float32x4,
    3 => Float32x4,
    4 => Float32x4,
    5 => Float32x2,
    6 => Float32,
];

pub(crate) struct GraphicsPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl GraphicsPipeline {
    pub(crate) fn new(app: &Application, output_format: wgpu::TextureFormat) -> Self {
        let device = app.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp::graphics_solid"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/graphics_solid.wgsl").into(),
            ),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wisp::graphics pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wisp::graphics pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("main_vs"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GraphicsInstance>() as wgpu::BufferAddress,
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

        Self { pipeline }
    }

    /// Walk `stage` in pre-order and emit one draw call per visible primitive
    /// across all `Graphics` nodes.
    ///
    /// All primitives share the same pipeline + bind groups, so they batch
    /// into a single draw call regardless of count.
    pub(crate) fn draw_stage(
        &self,
        app: &Application,
        pass: &mut wgpu::RenderPass<'_>,
        stage: &Stage,
    ) -> (u32, u32) {
        let instances = collect_instances(stage);
        if instances.is_empty() {
            return (0, 0);
        }
        let count = u32::try_from(instances.len()).expect("instance count fits in u32");
        let buffer = app
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("wisp::graphics instances"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            });

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..6, 0..count);
        (1, count)
    }
}

fn collect_instances(stage: &Stage) -> Vec<GraphicsInstance> {
    let mut out = Vec::new();
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

        if let Node::Graphics(graphics) = node {
            for primitive in &graphics.primitives {
                out.push(instance_for_primitive(primitive, world, container.alpha));
            }
        }

        for child in container.children().rev().collect::<Vec<_>>() {
            stack.push((child, world));
        }
    }
    out
}

fn instance_for_primitive(p: &Primitive, world: Mat4, parent_alpha: f32) -> GraphicsInstance {
    match p {
        Primitive::RoundedRect { rect, radius, fill } => {
            let (color_arr, alpha) = fill_color(*fill, parent_alpha);
            // Place the unit-quad-scaled-by-half_extents at the rect's center
            // in primitive-local space, then world transform.
            let half = glam::Vec2::new(rect.size.x * 0.5, rect.size.y * 0.5);
            let center = rect.min + half;
            let placement = Mat4::from_translation(glam::Vec3::new(center.x, center.y, 0.0));
            let model = world * placement;
            GraphicsInstance {
                model: model.to_cols_array_2d(),
                color: [
                    color_arr[0],
                    color_arr[1],
                    color_arr[2],
                    color_arr[3] * alpha,
                ],
                half_extents: [half.x, half.y],
                radius: *radius,
                _padding: 0.0,
            }
        }
    }
}

fn fill_color(fill: Fill, parent_alpha: f32) -> ([f32; 4], f32) {
    match fill {
        Fill::Solid(c) => ([c.r, c.g, c.b, c.a], parent_alpha),
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
