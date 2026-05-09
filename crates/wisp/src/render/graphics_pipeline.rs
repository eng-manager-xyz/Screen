//! Graphics primitive pipeline — M0.12 (rect/rounded rect), M0.13 (ellipse / line / stroke).
//!
//! All graphics primitives across all `Graphics` nodes batch into a single
//! draw call. Stroked primitives emit a second outline instance. Lines are
//! rendered as rotated thin rects.

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec2, Vec4};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::color::Color;
use crate::scene::graphics::{Fill, Primitive, Stroke};
use crate::scene::{Node, Stage};

const KIND_RECT: u32 = 0;
const KIND_ELLIPSE: u32 = 1;
const MODE_FILL: u32 = 0;
const MODE_OUTLINE: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct GraphicsInstance {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub half_extents: [f32; 2],
    pub radius: f32,
    pub stroke_width: f32,
    pub kind: u32,
    pub mode: u32,
}

const ATTR_LAYOUT: [wgpu::VertexAttribute; 10] = wgpu::vertex_attr_array![
    0 => Float32x4,
    1 => Float32x4,
    2 => Float32x4,
    3 => Float32x4,
    4 => Float32x4,
    5 => Float32x2,
    6 => Float32,
    7 => Float32,
    8 => Uint32,
    9 => Uint32,
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

    /// Walk `stage` in pre-order and emit one instance per visible primitive
    /// (plus an outline instance per stroked primitive). One draw call total.
    ///
    /// Returns `(draw_calls, primitives_drawn)` where `primitives_drawn`
    /// counts logical primitives (a stroked rect counts as 1, not 2).
    pub(crate) fn draw_stage(
        &self,
        app: &Application,
        pass: &mut wgpu::RenderPass<'_>,
        stage: &Stage,
    ) -> (u32, u32) {
        let (instances, logical_count) = collect_instances(stage);
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
        (1, logical_count)
    }
}

fn collect_instances(stage: &Stage) -> (Vec<GraphicsInstance>, u32) {
    let mut out = Vec::new();
    let mut logical = 0u32;
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
                logical = logical.saturating_add(1);
                emit_primitive(primitive, world, container.alpha, &mut out);
            }
        }

        for child in container.children().rev().collect::<Vec<_>>() {
            stack.push((child, world));
        }
    }
    (out, logical)
}

fn emit_primitive(p: &Primitive, world: Mat4, parent_alpha: f32, out: &mut Vec<GraphicsInstance>) {
    match *p {
        Primitive::RoundedRect {
            rect,
            radius,
            fill,
            stroke,
        } => {
            let half = Vec2::new(rect.size.x * 0.5, rect.size.y * 0.5);
            let center = rect.min + half;
            let model = world * Mat4::from_translation(glam::Vec3::new(center.x, center.y, 0.0));
            out.push(rect_instance(
                model,
                half,
                radius,
                fill,
                parent_alpha,
                MODE_FILL,
                0.0,
            ));
            if let Some(s) = stroke {
                out.push(rect_instance(
                    model,
                    half,
                    radius,
                    Fill::Solid(s.color),
                    parent_alpha,
                    MODE_OUTLINE,
                    s.width,
                ));
            }
        }
        Primitive::Ellipse {
            center,
            radii,
            fill,
            stroke,
        } => {
            let model = world * Mat4::from_translation(glam::Vec3::new(center.x, center.y, 0.0));
            out.push(ellipse_instance(
                model,
                radii,
                fill,
                parent_alpha,
                MODE_FILL,
                0.0,
            ));
            if let Some(s) = stroke {
                out.push(ellipse_instance(
                    model,
                    radii,
                    Fill::Solid(s.color),
                    parent_alpha,
                    MODE_OUTLINE,
                    s.width,
                ));
            }
        }
        Primitive::Line {
            from,
            to,
            width,
            fill,
        } => {
            let delta = to - from;
            let length = delta.length();
            if length < f32::EPSILON {
                return;
            }
            let angle = delta.y.atan2(delta.x);
            let center = (from + to) * 0.5;
            let translate = Mat4::from_translation(glam::Vec3::new(center.x, center.y, 0.0));
            let rotate = Mat4::from_rotation_z(angle);
            let model = world * translate * rotate;
            let half = Vec2::new(length * 0.5, width * 0.5);
            out.push(rect_instance(
                model,
                half,
                0.0,
                fill,
                parent_alpha,
                MODE_FILL,
                0.0,
            ));
        }
    }
}

fn rect_instance(
    model: Mat4,
    half: Vec2,
    radius: f32,
    fill: Fill,
    parent_alpha: f32,
    mode: u32,
    stroke_width: f32,
) -> GraphicsInstance {
    let color = resolve_fill(fill, parent_alpha);
    GraphicsInstance {
        model: model.to_cols_array_2d(),
        color,
        half_extents: [half.x, half.y],
        radius,
        stroke_width,
        kind: KIND_RECT,
        mode,
    }
}

fn ellipse_instance(
    model: Mat4,
    radii: Vec2,
    fill: Fill,
    parent_alpha: f32,
    mode: u32,
    stroke_width: f32,
) -> GraphicsInstance {
    let color = resolve_fill(fill, parent_alpha);
    GraphicsInstance {
        model: model.to_cols_array_2d(),
        color,
        half_extents: [radii.x, radii.y],
        radius: 0.0,
        stroke_width,
        kind: KIND_ELLIPSE,
        mode,
    }
}

fn resolve_fill(fill: Fill, parent_alpha: f32) -> [f32; 4] {
    match fill {
        Fill::Solid(c) => apply_alpha(c, parent_alpha),
    }
}

fn apply_alpha(c: Color, parent_alpha: f32) -> [f32; 4] {
    [c.r, c.g, c.b, c.a * parent_alpha]
}

fn mat3_to_mat4(m: Mat3) -> Mat4 {
    Mat4::from_cols(
        Vec4::new(m.x_axis.x, m.x_axis.y, 0.0, 0.0),
        Vec4::new(m.y_axis.x, m.y_axis.y, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(m.z_axis.x, m.z_axis.y, 0.0, 1.0),
    )
}

// `Stroke` is consumed via `*p` deref; this re-export keeps the imports clean.
#[allow(dead_code, reason = "re-exported to keep collect_instances readable")]
const _: Option<Stroke> = None;
