//! `EdgesMesh` + line-list wireframe pipeline (W3D.5 / AUT-297).
//!
//! Sharp-edge derivation matches `THREE.EdgesGeometry(geom, 8°)`:
//! for every triangle edge that's shared by exactly two triangles,
//! emit a line segment iff the angle between the two face normals
//! exceeds the `angle_threshold_deg` value. Boundary edges (used by
//! only one triangle) always emit.
//!
//! Line rendering uses `PrimitiveTopology::LineList`. On wgpu this
//! produces 1-device-pixel-wide hairlines on every backend; that
//! matches the 404 page's 1px wireframe at typical viewing distance.
//! Thick lines via screen-space-expanded quads are deferred to a
//! follow-up (filed alongside this ticket).
//!
//! Pipeline state notes:
//! - `depth_compare: LessEqual` so edges that coincide with the
//!   underlying mesh's depth still draw (otherwise z-fighting hides
//!   them).
//! - `depth_write_enabled: false` — wireframe doesn't occlude
//!   anything behind it.
//! - `cull_mode: None` — line segments are 1D, no front/back face.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wisp::application::Application;

use crate::camera::{Camera3D, ViewProj};
use crate::mesh::Mesh3D;
use crate::render::DEPTH_FORMAT;

/// Internal bucket: triangle indices that share a coordinate-bucketed
/// edge + the endpoint positions captured from the first contributor.
#[derive(Default)]
struct EdgeBucket {
    /// Triangle indices that own this edge.
    tris: Vec<usize>,
    /// Endpoints (positions) for the edge — captured from the first
    /// triangle that contributed.
    endpoints: Option<([f32; 3], [f32; 3])>,
}

/// One line segment (a pair of endpoints) for the wireframe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge3D {
    /// First endpoint.
    pub a: [f32; 3],
    /// Second endpoint.
    pub b: [f32; 3],
}

/// A list of line segments derived from a triangle mesh.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EdgesMesh {
    /// Segments in render order.
    pub segments: Vec<Edge3D>,
}

impl EdgesMesh {
    /// Build from a triangle mesh.
    ///
    /// For each triangle edge shared by exactly two triangles, emit
    /// a segment iff the angle between the two face normals is
    /// `>= angle_threshold_deg`. Boundary edges (only one triangle)
    /// always emit.
    ///
    /// Triangle face normals are recomputed here so the call doesn't
    /// depend on `Mesh3D::compute_vertex_normals` having been called.
    #[must_use]
    pub fn from_mesh(mesh: &Mesh3D, angle_threshold_deg: f32) -> Self {
        if mesh.indices.is_empty() {
            return Self::default();
        }
        let cos_threshold = angle_threshold_deg.to_radians().cos();

        let mut tri_normals: Vec<Vec3> = Vec::with_capacity(mesh.indices.len() / 3);
        for tri in mesh.indices.chunks_exact(3) {
            let a = Vec3::from(mesh.positions[tri[0] as usize]);
            let b = Vec3::from(mesh.positions[tri[1] as usize]);
            let c = Vec3::from(mesh.positions[tri[2] as usize]);
            tri_normals.push((b - a).cross(c - a).normalize_or_zero());
        }

        // Bucket edges by the canonical endpoint pair. Two edges with
        // the same coordinates (across different vertex indices —
        // common when faces are duplicated for flat shading) end up
        // in the same bucket.
        let mut buckets: HashMap<(EdgeKey, EdgeKey), EdgeBucket> = HashMap::new();
        for (tri_idx, tri) in mesh.indices.chunks_exact(3).enumerate() {
            let p = [
                mesh.positions[tri[0] as usize],
                mesh.positions[tri[1] as usize],
                mesh.positions[tri[2] as usize],
            ];
            for (i, j) in [(0_usize, 1_usize), (1, 2), (2, 0)] {
                let key_a = EdgeKey::from(p[i]);
                let key_b = EdgeKey::from(p[j]);
                let (lo, hi) = if key_a <= key_b {
                    (key_a, key_b)
                } else {
                    (key_b, key_a)
                };
                let bucket = buckets.entry((lo, hi)).or_default();
                bucket.tris.push(tri_idx);
                if bucket.endpoints.is_none() {
                    bucket.endpoints = Some((p[i], p[j]));
                }
            }
        }

        let mut segments: Vec<Edge3D> = Vec::new();
        for bucket in buckets.values() {
            let Some((ea, eb)) = bucket.endpoints else {
                continue;
            };
            let emit = match bucket.tris.as_slice() {
                [] => false,
                // Two-triangle interior edge — emit iff angle exceeds.
                [t0, t1] => {
                    let n0 = tri_normals[*t0];
                    let n1 = tri_normals[*t1];
                    // If both normals are non-zero unit vectors,
                    // angle exceeds threshold ⇔ dot < cos(threshold).
                    n0.dot(n1) < cos_threshold
                }
                // Boundary edges (1 triangle) AND non-manifold edges
                // (3+ triangles) both always emit — boundaries are
                // by definition sharp, non-manifold needs to be
                // visible to surface the issue.
                _ => true,
            };
            if emit {
                segments.push(Edge3D { a: ea, b: eb });
            }
        }

        // Stable order for snapshot testing.
        segments.sort_by_key(edge_order);

        Self { segments }
    }

    /// Number of segments — convenience for tests.
    #[must_use]
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// `true` when no segments.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Build the flat `[f32; 3]` vertex stream (2 verts per segment)
    /// ready to upload to a wgpu vertex buffer.
    #[must_use]
    pub fn vertex_buffer(&self) -> Vec<[f32; 3]> {
        let mut v = Vec::with_capacity(self.segments.len() * 2);
        for s in &self.segments {
            v.push(s.a);
            v.push(s.b);
        }
        v
    }
}

fn edge_order(e: &Edge3D) -> (EdgeKey, EdgeKey) {
    let a = EdgeKey::from(e.a);
    let b = EdgeKey::from(e.b);
    if a <= b { (a, b) } else { (b, a) }
}

/// Coordinate-bucketed key for deduping vertices across face-
/// duplicated meshes. 1e-4 precision is fine for our scale.
#[derive(Hash, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
struct EdgeKey(i32, i32, i32);

impl From<[f32; 3]> for EdgeKey {
    fn from(p: [f32; 3]) -> Self {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "position components are bounded by the scene's working units (a few thousand at most); fit i32"
        )]
        fn q(v: f32) -> i32 {
            (v * 10_000.0).round() as i32
        }
        Self(q(p[0]), q(p[1]), q(p[2]))
    }
}

// ─── GPU pipeline ──────────────────────────────────────────────────

/// Per-draw wireframe color uniform.
///
/// `#[repr(C, align(16))]` per CLAUDE.md "WGSL ↔ Rust uniform
/// layout" — single `vec4<f32>` is naturally 16-byte aligned.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct LineColor {
    /// RGBA. Alpha is honoured by the fragment shader; defaults to
    /// 0.82 for the 404 wireframe (matches THREE's
    /// `LineBasicMaterial({ transparent: true, opacity: 0.82 })`).
    pub color: [f32; 4],
}

/// Wireframe-drawing helper.
pub struct WireframePipeline {
    pipeline: wgpu::RenderPipeline,
    color_layout: wgpu::BindGroupLayout,
    view_proj_buffer: wgpu::Buffer,
    view_proj_bg: wgpu::BindGroup,
}

impl WireframePipeline {
    /// Construct with the same format / MSAA the underlying mesh pass
    /// uses (so they can share the depth attachment).
    #[must_use]
    #[allow(
        clippy::too_many_lines,
        reason = "Single linear pipeline-setup; splitting would scatter the bind-group-layout / buffer / pipeline triple across helpers with no reuse."
    )]
    pub fn new(app: &Application, output_format: wgpu::TextureFormat, msaa_samples: u32) -> Self {
        let device = app.device();

        let view_proj_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp_3d::wireframe::view_proj_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<ViewProj>() as u64),
                },
                count: None,
            }],
        });
        let color_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("wisp_3d::wireframe::color_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<LineColor>() as u64
                        ),
                    },
                    count: None,
                }],
            });

        let view_proj_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp_3d::wireframe::view_proj_buffer"),
            size: std::mem::size_of::<ViewProj>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_proj_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp_3d::wireframe::view_proj_bg"),
            layout: &view_proj_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: view_proj_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp_3d::wireframe_lines"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/wireframe_lines.wgsl").into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wisp_3d::wireframe::pipeline_layout"),
            bind_group_layouts: &[&view_proj_layout, &color_layout],
            push_constant_ranges: &[],
        });

        let vbo_layout = wgpu::VertexBufferLayout {
            array_stride: (std::mem::size_of::<[f32; 3]>()) as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wisp_3d::wireframe_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("main_vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[vbo_layout],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                // Always-pass: wireframe sits ON TOP of whatever's
                // drawn into the same color attachment, regardless
                // of depth. Two reasons:
                //
                // 1. Browser WebGPU REJECTS non-zero `depth_bias`
                //    on `LineList` topology — the bias trick that
                //    normally breaks z-fighting between coplanar
                //    edges and their mesh isn't available.
                //
                // 2. Even with a 1.002× outward geometric offset
                //    (see `wireframe_lines.wgsl::main_vs`), the
                //    `LessEqual` test still loses against the
                //    rasterised mesh's interpolated depth at
                //    interior line pixels — observed empirically:
                //    `LessEqual` produces no visible edges,
                //    `Always` produces clean outlines.
                //
                // Trade-off: the wireframe is visible THROUGH the
                // mesh from behind too. For the 404 pyramid use
                // case (rotating, looking at front faces) that's
                // visually identical to a depth-tested edge. If a
                // future consumer needs back-face hiding,
                // alternatives are: front-face cull on the
                // wireframe, or render edges per-face only.
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: msaa_samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("main_fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        let _ = view_proj_layout;
        Self {
            pipeline,
            color_layout,
            view_proj_buffer,
            view_proj_bg,
        }
    }

    /// Encode the wireframe draw INTO an existing render pass.
    /// Caller has already set up color + depth attachments and is
    /// inside the same `begin_render_pass` scope as the underlying
    /// mesh; we just bind + draw.
    #[allow(
        clippy::too_many_arguments,
        reason = "Render-call surface intentionally exposes every wgpu attachment / state input; wrapping in a builder would just defer the same set of fields."
    )]
    pub fn draw_into<'a>(
        &'a mut self,
        app: &Application,
        pass: &mut wgpu::RenderPass<'a>,
        camera: &Camera3D,
        edges: &EdgesMesh,
        color: LineColor,
        vbuf: &'a wgpu::Buffer,
        color_bg: &'a wgpu::BindGroup,
    ) {
        if edges.is_empty() {
            return;
        }
        let vp = camera.view_proj_uniform();
        app.queue()
            .write_buffer(&self.view_proj_buffer, 0, bytemuck::bytes_of(&vp));
        let _ = color; // color is uploaded by caller before constructing color_bg

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.view_proj_bg, &[]);
        pass.set_bind_group(1, color_bg, &[]);
        pass.set_vertex_buffer(0, vbuf.slice(..));
        #[allow(
            clippy::cast_possible_truncation,
            reason = "edges.segments.len() bounded by mesh size; 2 verts per segment fits u32 well below realistic mesh sizes"
        )]
        let vert_count = (edges.segments.len() * 2) as u32;
        pass.draw(0..vert_count, 0..1);
    }

    /// Allocate a per-draw `LineColor` UBO + bind group.
    /// Owned by the caller so they can keep it alive across the
    /// `begin_render_pass` borrow.
    #[must_use]
    pub fn build_color_resources(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color: LineColor,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp_3d::wireframe::color_buf"),
            size: std::mem::size_of::<LineColor>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buf, 0, bytemuck::bytes_of(&color));
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp_3d::wireframe::color_bg"),
            layout: &self.color_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buf.as_entire_binding(),
            }],
        });
        (buf, bg)
    }

    /// Upload the edge-segment vertex stream to a fresh wgpu buffer.
    #[must_use]
    pub fn build_vertex_buffer(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        edges: &EdgesMesh,
    ) -> wgpu::Buffer {
        let verts = edges.vertex_buffer();
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp_3d::wireframe::vbuf"),
            size: (verts.len() * std::mem::size_of::<[f32; 3]>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buf, 0, bytemuck::cast_slice(&verts));
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pyramid_at_8deg_has_8_edges() {
        // 4 apex-to-base edges (CCW around the square base) + 4
        // base perimeter edges = 8 sharp edges. The internal
        // diagonal of the square base is coplanar (180°) → not an
        // edge.
        let mesh = Mesh3D::pyramid(1.34, 1.25);
        let edges = EdgesMesh::from_mesh(&mesh, 8.0);
        assert_eq!(edges.len(), 8, "got {edges:?}");
    }

    #[test]
    fn pyramid_at_180deg_returns_empty() {
        // Only edges with face-normal angle >= 180° emit. Even a
        // pyramid's sharp dihedrals don't hit that, so result is
        // empty.
        let mesh = Mesh3D::pyramid(1.34, 1.25);
        let edges = EdgesMesh::from_mesh(&mesh, 180.0);
        assert!(edges.is_empty(), "got {edges:?}");
    }

    #[test]
    fn unit_triangle_has_three_boundary_edges() {
        // A standalone triangle: every edge is boundary (used by 1
        // triangle) → all 3 emit regardless of threshold.
        let mesh = Mesh3D {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            indices: vec![0, 1, 2],
        };
        for threshold in [0.0, 30.0, 90.0, 179.0] {
            let edges = EdgesMesh::from_mesh(&mesh, threshold);
            assert_eq!(edges.len(), 3, "threshold {threshold}");
        }
    }

    #[test]
    fn empty_mesh_returns_empty_edges() {
        let mesh = Mesh3D::default();
        let edges = EdgesMesh::from_mesh(&mesh, 8.0);
        assert!(edges.is_empty());
    }

    #[test]
    fn line_color_size_matches_wgsl_layout() {
        assert_eq!(std::mem::size_of::<LineColor>(), 16);
        assert_eq!(std::mem::align_of::<LineColor>(), 16);
    }
}
