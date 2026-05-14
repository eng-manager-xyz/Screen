//! Graphics primitive pipeline.
//!
//! Evolution: M0.12 (rect/rounded rect) → M0.13 (ellipse / line / stroke) →
//! M0.14 (linear + radial gradient fills).
//!
//! All graphics primitives across all `Graphics` nodes batch into a single
//! draw call. Stroked primitives emit a second outline instance. Lines are
//! rendered as rotated thin rects.

use std::collections::{HashMap, HashSet};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2};
use wgpu::util::DeviceExt;

use crate::application::Application;
use crate::blend::BlendMode;
use crate::color::Color;
use crate::render::blend_pipeline::BlendPipelineMap;
use crate::render::scene_walk::walk_visible_subtree;
use crate::scene::graphics::{Fill, Primitive, Stroke};
use crate::scene::{Node, NodeId, Stage};

const KIND_RECT: u32 = 0;
const KIND_ELLIPSE: u32 = 1;
const KIND_ANNULAR_SECTOR: u32 = 2;
const MODE_FILL: u32 = 0;
const MODE_OUTLINE: u32 = 1;
const FILL_SOLID: u32 = 0;
const FILL_LINEAR: u32 = 1;
const FILL_RADIAL: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct GraphicsInstance {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub color_b: [f32; 4],
    pub half_extents: [f32; 2],
    pub radius: f32,
    pub stroke_width: f32,
    pub grad_a: [f32; 2],
    pub grad_b: [f32; 2],
    /// Packed `[kind, mode, fill_kind, _padding]`. Bundled into
    /// one Uint32x4 attribute so the subsequent `arc_data`
    /// Float32x4 falls on a 16-byte boundary — `vertex_attr_array!`
    /// computes offsets sequentially without honouring struct
    /// padding, so a packed quad here matches the `repr(C)`
    /// layout exactly.
    pub kind_pack: [u32; 4],
    /// `[r_inner, r_outer, mid_angle, half_angle]` for annular
    /// sectors. Zeroed (and ignored) for other primitive kinds.
    pub arc_data: [f32; 4],
}

const ATTR_LAYOUT: [wgpu::VertexAttribute; 13] = wgpu::vertex_attr_array![
    0  => Float32x4,
    1  => Float32x4,
    2  => Float32x4,
    3  => Float32x4,
    4  => Float32x4,
    5  => Float32x4,
    6  => Float32x2,
    7  => Float32,
    8  => Float32,
    9  => Float32x2,
    10 => Float32x2,
    11 => Uint32x4,
    12 => Float32x4,
];

/// One vertex of a polygon's fan triangulation. Position is in
/// clip space — the world matrix is baked CPU-side during the
/// scene walk, since polygon vertex counts are variable and don't
/// fit the per-instance model the SDF primitives use.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct PolygonVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

const POLYGON_ATTR_LAYOUT: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
    0 => Float32x2,
    1 => Float32x4,
];

pub(crate) struct GraphicsPipeline {
    pipelines: BlendPipelineMap,
    polygon_pipelines: BlendPipelineMap,
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

        let pipelines = BlendPipelineMap::new(|blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("wisp::graphics pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("main_vs"),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GraphicsInstance>()
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

        // Polygon (triangle-list) sub-pipeline. Separate WGSL +
        // VertexBufferLayout because polygons have variable vertex
        // counts and can't fit the instanced-quad model the SDF
        // path uses.
        let polygon_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp::graphics_polygon"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/graphics_polygon.wgsl").into(),
            ),
        });
        let polygon_pipelines = BlendPipelineMap::new(|blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("wisp::graphics polygon pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &polygon_shader,
                    entry_point: Some("main_vs"),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<PolygonVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &POLYGON_ATTR_LAYOUT,
                    }],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &polygon_shader,
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
            polygon_pipelines,
        }
    }

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
        let CollectedGraphics {
            instance_groups,
            polygon_groups,
            logical_count,
        } = collect_graphics(stage, start, exclude);
        let mut draw_calls = 0u32;

        // SDF instance pass (rect / rounded rect / ellipse / line /
        // annular sector). Issued first so polygons composite on
        // top in scene-tree order, matching the existing primitive
        // ordering semantics inside a single Graphics node.
        for (mode, instances) in instance_groups {
            if instances.is_empty() {
                continue;
            }
            let count = u32::try_from(instances.len()).expect("instance count fits in u32");
            let buffer = app
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("wisp::graphics instances"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            pass.set_pipeline(self.pipelines.get(mode));
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..6, 0..count);
            draw_calls += 1;
        }

        // Polygon triangle-list pass.
        for (mode, vertices) in polygon_groups {
            if vertices.is_empty() {
                continue;
            }
            let vertex_count =
                u32::try_from(vertices.len()).expect("polygon vertex count fits in u32");
            let buffer = app
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("wisp::graphics polygon vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            pass.set_pipeline(self.polygon_pipelines.get(mode));
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..vertex_count, 0..1);
            draw_calls += 1;
        }

        (draw_calls, logical_count)
    }
}

struct CollectedGraphics {
    instance_groups: Vec<(BlendMode, Vec<GraphicsInstance>)>,
    polygon_groups: Vec<(BlendMode, Vec<PolygonVertex>)>,
    logical_count: u32,
}

/// Walk the scene and bucket every primitive a `Graphics` node
/// emits, separated into:
/// * SDF instances (rect / rounded rect / ellipse / line / arc),
/// * polygon triangles (variable-vertex-count primitives that go
///   through the dedicated `graphics_polygon.wgsl` path).
///
/// Both buckets group by blend mode in encounter order so each
/// batch binds its matching pipeline.
fn collect_graphics(stage: &Stage, start: NodeId, exclude: &HashSet<NodeId>) -> CollectedGraphics {
    let mut instance_grouped: HashMap<BlendMode, Vec<GraphicsInstance>> = HashMap::new();
    let mut polygon_grouped: HashMap<BlendMode, Vec<PolygonVertex>> = HashMap::new();
    let mut instance_order: Vec<BlendMode> = Vec::new();
    let mut polygon_order: Vec<BlendMode> = Vec::new();
    let mut logical = 0u32;
    walk_visible_subtree(stage, start, exclude, |_id, node, world| {
        let container = node.container();
        if let Node::Graphics(graphics) = node {
            let mode = container.blend_mode;
            for primitive in &graphics.primitives {
                logical = logical.saturating_add(1);
                if matches!(primitive, Primitive::Polygon { .. }) {
                    let bucket = polygon_grouped.entry(mode).or_insert_with(|| {
                        polygon_order.push(mode);
                        Vec::new()
                    });
                    emit_polygon(primitive, world, container.alpha, bucket);
                } else {
                    let bucket = instance_grouped.entry(mode).or_insert_with(|| {
                        instance_order.push(mode);
                        Vec::new()
                    });
                    emit_primitive(primitive, world, container.alpha, bucket);
                }
            }
        }
    });
    let instance_groups = instance_order
        .into_iter()
        .filter_map(|m| instance_grouped.remove(&m).map(|v| (m, v)))
        .collect();
    let polygon_groups = polygon_order
        .into_iter()
        .filter_map(|m| polygon_grouped.remove(&m).map(|v| (m, v)))
        .collect();
    CollectedGraphics {
        instance_groups,
        polygon_groups,
        logical_count: logical,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "single dispatch table over Primitive variants — splitting per variant would add indirection without clarity"
)]
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
        Primitive::Polygon { .. } => {
            // Handled by `emit_polygon` — separate triangle-list
            // path. Reaching here would mean the dispatcher
            // misrouted; bail rather than emit a malformed
            // instance.
        }
        Primitive::AnnularSector {
            center,
            r_inner,
            r_outer,
            start_angle,
            end_angle,
            fill,
            stroke,
        } => {
            // Quad covers the AABB around the annular sector
            // centred at `center`. Outer-radius bound is the
            // simple safe choice (slightly oversized when the
            // angular span doesn't wrap a full half-circle, but
            // never undersized — no clipping).
            let r_inner = r_inner.max(0.0);
            let r_outer = r_outer.max(r_inner);
            let span = (end_angle - start_angle).clamp(0.0, std::f32::consts::TAU);
            let mid_angle = start_angle + span * 0.5;
            let half_angle = span * 0.5;
            let model = world * Mat4::from_translation(glam::Vec3::new(center.x, center.y, 0.0));
            let half = Vec2::new(r_outer, r_outer);
            out.push(annular_sector_instance(
                model,
                half,
                r_inner,
                r_outer,
                mid_angle,
                half_angle,
                fill,
                parent_alpha,
                MODE_FILL,
                0.0,
            ));
            if let Some(s) = stroke {
                out.push(annular_sector_instance(
                    model,
                    half,
                    r_inner,
                    r_outer,
                    mid_angle,
                    half_angle,
                    Fill::Solid(s.color),
                    parent_alpha,
                    MODE_OUTLINE,
                    s.width,
                ));
            }
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
    let resolved = resolve_fill(fill, parent_alpha);
    GraphicsInstance {
        model: model.to_cols_array_2d(),
        color: resolved.color,
        color_b: resolved.color_b,
        half_extents: [half.x, half.y],
        radius,
        stroke_width,
        grad_a: resolved.grad_a,
        grad_b: resolved.grad_b,
        kind_pack: [KIND_RECT, mode, resolved.fill_kind, 0],
        arc_data: [0.0; 4],
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
    let resolved = resolve_fill(fill, parent_alpha);
    GraphicsInstance {
        model: model.to_cols_array_2d(),
        color: resolved.color,
        color_b: resolved.color_b,
        half_extents: [radii.x, radii.y],
        radius: 0.0,
        stroke_width,
        grad_a: resolved.grad_a,
        grad_b: resolved.grad_b,
        kind_pack: [KIND_ELLIPSE, mode, resolved.fill_kind, 0],
        arc_data: [0.0; 4],
    }
}

/// Fan-triangulate a convex polygon and bake each vertex into
/// clip space via `world`. Non-convex polygons produce visible
/// overlap — that's the documented v1 limitation.
fn emit_polygon(p: &Primitive, world: Mat4, parent_alpha: f32, out: &mut Vec<PolygonVertex>) {
    let Primitive::Polygon { vertices, fill } = p else {
        return;
    };
    if vertices.len() < 3 {
        return;
    }
    let color = apply_alpha(
        match *fill {
            Fill::Solid(c) => c,
            // Gradients on polygons are deferred — flatten to the
            // first colour stop so the primitive still renders.
            Fill::LinearGradient { color_a, .. } | Fill::RadialGradient { color_a, .. } => color_a,
        },
        parent_alpha,
    );
    let to_clip = |v: Vec2| -> [f32; 2] {
        let clip = world * glam::Vec4::new(v.x, v.y, 0.0, 1.0);
        [clip.x, clip.y]
    };
    // Fan from vertex 0: (v0, v_i, v_{i+1}) for i in 1..n-1.
    let v0 = to_clip(vertices[0]);
    for i in 1..vertices.len() - 1 {
        let vi = to_clip(vertices[i]);
        let vj = to_clip(vertices[i + 1]);
        out.push(PolygonVertex {
            position: v0,
            color,
        });
        out.push(PolygonVertex {
            position: vi,
            color,
        });
        out.push(PolygonVertex {
            position: vj,
            color,
        });
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "annular sector emission inherits the same flat parameter style as rect/ellipse helpers"
)]
fn annular_sector_instance(
    model: Mat4,
    half: Vec2,
    r_inner: f32,
    r_outer: f32,
    mid_angle: f32,
    half_angle: f32,
    fill: Fill,
    parent_alpha: f32,
    mode: u32,
    stroke_width: f32,
) -> GraphicsInstance {
    let resolved = resolve_fill(fill, parent_alpha);
    GraphicsInstance {
        model: model.to_cols_array_2d(),
        color: resolved.color,
        color_b: resolved.color_b,
        half_extents: [half.x, half.y],
        radius: 0.0,
        stroke_width,
        grad_a: resolved.grad_a,
        grad_b: resolved.grad_b,
        kind_pack: [KIND_ANNULAR_SECTOR, mode, resolved.fill_kind, 0],
        arc_data: [r_inner, r_outer, mid_angle, half_angle],
    }
}

struct ResolvedFill {
    fill_kind: u32,
    color: [f32; 4],
    color_b: [f32; 4],
    grad_a: [f32; 2],
    grad_b: [f32; 2],
}

fn resolve_fill(fill: Fill, parent_alpha: f32) -> ResolvedFill {
    match fill {
        Fill::Solid(c) => {
            let arr = apply_alpha(c, parent_alpha);
            ResolvedFill {
                fill_kind: FILL_SOLID,
                color: arr,
                color_b: arr,
                grad_a: [0.0, 0.0],
                grad_b: [0.0, 0.0],
            }
        }
        Fill::LinearGradient {
            start,
            end,
            color_a,
            color_b,
        } => ResolvedFill {
            fill_kind: FILL_LINEAR,
            color: apply_alpha(color_a, parent_alpha),
            color_b: apply_alpha(color_b, parent_alpha),
            grad_a: [start.x, start.y],
            grad_b: [end.x, end.y],
        },
        Fill::RadialGradient {
            center,
            radius,
            color_a,
            color_b,
        } => ResolvedFill {
            fill_kind: FILL_RADIAL,
            color: apply_alpha(color_a, parent_alpha),
            color_b: apply_alpha(color_b, parent_alpha),
            grad_a: [center.x, center.y],
            grad_b: [radius, 0.0],
        },
    }
}

fn apply_alpha(c: Color, parent_alpha: f32) -> [f32; 4] {
    [c.r, c.g, c.b, c.a * parent_alpha]
}

#[allow(dead_code, reason = "re-exported to keep collect_instances readable")]
const _: Option<Stroke> = None;
