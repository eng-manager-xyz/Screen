//! `Mesh3D` — indexed triangle mesh with per-vertex positions +
//! normals (W3D.2 / AUT-294).
//!
//! ## Data model
//!
//! Three parallel buffers: `positions`, `normals`, `indices`. Indices
//! reference `positions[i]` + `normals[i]` (no per-attribute index —
//! matches wgpu's vertex-attribute model where every attribute reads
//! from the same `gl_VertexIndex`).
//!
//! For flat-shaded meshes like the pyramid we duplicate vertices per
//! face so each face's normal is independent (the apex vertex appears
//! 4 times — once per side face — so each appearance can carry a
//! different normal). The trade-off is more vertex memory; the win is
//! sharp dihedrals without a geometry shader.
//!
//! ## Pyramid constructor
//!
//! [`Mesh3D::pyramid`] produces the engmanager.xyz 404 layout: square
//! base centred at `y = -1.05`, apex at `(0, apex_y, 0)`, base half-
//! width `base_half`. 18 positions / 18 indices / 6 triangles
//! (4 side faces + 2 base triangles).
//!
//! ## Compute normals
//!
//! [`Mesh3D::compute_vertex_normals`] writes flat per-face normals.
//! For shared-vertex meshes (no duplication) you'd want averaged
//! per-vertex normals — that's a separate constructor; this method
//! is the flat-shaded path matching THREE.js's default for non-
//! indexed `BufferGeometry::computeVertexNormals`.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// One mesh vertex shipped to the GPU. Position + normal.
///
/// `#[repr(C)]` so the field order matches the WGSL vertex-attribute
/// declarations exactly; layout matches
/// [`Mesh3D::wgpu_attributes`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Vertex3D {
    /// World/local position. Multiplied by the model matrix in the
    /// vertex shader.
    pub position: [f32; 3],
    /// Surface normal. Transformed by the normal matrix in the vertex
    /// shader for lighting calculations.
    pub normal: [f32; 3],
}

/// Indexed triangle mesh.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh3D {
    /// Per-vertex positions. Vertex `i` has position `positions[i]`
    /// and normal `normals[i]`.
    pub positions: Vec<[f32; 3]>,
    /// Per-vertex normals. Same length as `positions`. Computed via
    /// [`Self::compute_vertex_normals`] if you don't author them
    /// directly.
    pub normals: Vec<[f32; 3]>,
    /// Triangle indices into `positions` / `normals`. Length must be
    /// a multiple of 3.
    pub indices: Vec<u32>,
}

impl Mesh3D {
    /// Compute flat per-face normals.
    ///
    /// Iterates triangles in `indices`, computes the face normal once
    /// per triangle, and writes it into each of the triangle's three
    /// vertex slots. This produces sharp dihedrals (matching
    /// THREE.js's default behaviour for non-indexed geometry) at the
    /// cost of requiring per-face vertex duplication in
    /// `positions` — which `Self::pyramid` already does.
    ///
    /// If a vertex is referenced by multiple triangles with
    /// different face normals, the last triangle wins (use
    /// per-face-duplicated positions to avoid that).
    pub fn compute_vertex_normals(&mut self) {
        // Ensure the normal buffer has the right length.
        self.normals = vec![[0.0; 3]; self.positions.len()];
        for tri in self.indices.chunks_exact(3) {
            // Safe via chunks_exact: every chunk has length 3.
            let ia = tri[0] as usize;
            let ib = tri[1] as usize;
            let ic = tri[2] as usize;
            let a = Vec3::from(self.positions[ia]);
            let b = Vec3::from(self.positions[ib]);
            let c = Vec3::from(self.positions[ic]);
            let normal = (b - a).cross(c - a).normalize_or_zero();
            let arr = [normal.x, normal.y, normal.z];
            self.normals[ia] = arr;
            self.normals[ib] = arr;
            self.normals[ic] = arr;
        }
    }

    /// Combined `Vertex3D` buffer ready to upload as
    /// `wgpu::VertexBufferLayout`. Use [`Self::wgpu_attributes`] for
    /// the matching attribute decl.
    #[must_use]
    pub fn vertex_buffer(&self) -> Vec<Vertex3D> {
        self.positions
            .iter()
            .zip(self.normals.iter())
            .map(|(p, n)| Vertex3D {
                position: *p,
                normal: *n,
            })
            .collect()
    }

    /// wgpu vertex-attribute layout matching [`Vertex3D`]:
    /// `@location(0) position: vec3<f32>`,
    /// `@location(1) normal: vec3<f32>`.
    #[must_use]
    pub fn wgpu_attributes() -> [wgpu::VertexAttribute; 2] {
        wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
        ]
    }

    /// `VertexBufferLayout` for a `Vertex3D` buffer.
    #[must_use]
    pub fn wgpu_vertex_buffer_layout(
        attrs: &[wgpu::VertexAttribute; 2],
    ) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex3D>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: attrs,
        }
    }

    /// Build the engmanager.xyz 404 pyramid:
    /// - apex at `(0, apex_y, 0)`
    /// - square base centred at `y = -1.05`, half-extent `base_half`
    ///   on x and z
    /// - 4 side faces + 2 base triangles
    /// - 18 vertices (3 per triangle, duplicated per face so each
    ///   face has its own normal)
    /// - 18 indices = `0..18`
    ///
    /// Matches the THREE.js layout in `not-found.js::makePyramidGeometry`
    /// with `apex_y = 1.34`, `base_half = 1.25`, base at `y = -1.05`.
    #[must_use]
    pub fn pyramid(apex_y: f32, base_half: f32) -> Self {
        let base_y = -1.05_f32;
        let apex = [0.0, apex_y, 0.0];
        let nw = [-base_half, base_y, -base_half];
        let ne = [base_half, base_y, -base_half];
        let se = [base_half, base_y, base_half];
        let sw = [-base_half, base_y, base_half];

        // Six triangles, in the same winding order as the THREE
        // version: side faces apex-to-base-CCW from above, base
        // triangles CW from above (face down so they're invisible
        // from outside the pyramid).
        let positions: Vec<[f32; 3]> = vec![
            apex, sw, se, // front face
            apex, se, ne, // right face
            apex, ne, nw, // back face
            apex, nw, sw, // left face
            sw, nw, ne, // base tri 1
            sw, ne, se, // base tri 2
        ];
        let indices: Vec<u32> = (0..18).collect();
        let mut mesh = Self {
            positions,
            normals: Vec::new(),
            indices,
        };
        mesh.compute_vertex_normals();
        mesh
    }

    /// Triangle count.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Helper: bucket a normal into a 1e-3-precision key so `HashSet`
    /// can dedupe near-equal directions.
    fn key(n: [f32; 3]) -> (i32, i32, i32) {
        let scale = 1000.0;
        #[allow(
            clippy::cast_possible_truncation,
            reason = "normal components in [-1, 1]; scaled value fits i32 comfortably"
        )]
        let f = |v: f32| (v * scale).round() as i32;
        (f(n[0]), f(n[1]), f(n[2]))
    }

    #[test]
    fn pyramid_has_18_positions_and_18_indices() {
        let m = Mesh3D::pyramid(1.34, 1.25);
        assert_eq!(m.positions.len(), 18);
        assert_eq!(m.normals.len(), 18);
        assert_eq!(m.indices.len(), 18);
        assert_eq!(m.triangle_count(), 6);
        // Indices are 0..18 in order — each triangle owns its
        // vertices, no sharing.
        for (i, idx) in m.indices.iter().enumerate() {
            assert_eq!(*idx as usize, i);
        }
    }

    #[test]
    fn pyramid_has_5_unique_face_normals() {
        // 4 side faces + 1 base face (the two base triangles share a
        // normal because they're coplanar) = 5 unique directions.
        let m = Mesh3D::pyramid(1.34, 1.25);
        let unique: HashSet<_> = m.normals.iter().map(|n| key(*n)).collect();
        assert_eq!(
            unique.len(),
            5,
            "expected 5 unique face normals (4 sides + 1 base), got {unique:?}"
        );
    }

    #[test]
    fn pyramid_base_normal_points_down() {
        // Triangles 4 and 5 (vertex indices 12..18) are the base.
        // Cross product of the wound vertices points -Y from the
        // outside.
        let m = Mesh3D::pyramid(1.34, 1.25);
        for v in 12..18 {
            let n = m.normals[v];
            assert!(
                (n[1] - -1.0).abs() < 1e-4,
                "base normal at vert {v} should point -Y, got {n:?}"
            );
        }
    }

    #[test]
    fn pyramid_apex_is_topmost_vertex() {
        let m = Mesh3D::pyramid(1.34, 1.25);
        let max_y = m
            .positions
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((max_y - 1.34).abs() < 1e-6);
        // The apex appears exactly 4 times (one per side face).
        let apex_count = m
            .positions
            .iter()
            .filter(|p| (p[1] - 1.34).abs() < 1e-6)
            .count();
        assert_eq!(apex_count, 4);
    }

    #[test]
    fn compute_vertex_normals_on_unit_triangle_returns_z_up() {
        let mut m = Mesh3D {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: Vec::new(),
            indices: vec![0, 1, 2],
        };
        m.compute_vertex_normals();
        for n in &m.normals {
            assert!((n[0]).abs() < 1e-6);
            assert!((n[1]).abs() < 1e-6);
            assert!((n[2] - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn vertex_buffer_pairs_positions_with_normals() {
        // f32 array equality via clippy's `float_cmp` lint blocks
        // bare `assert_eq!` on `[f32; 3]`. The values here are
        // copied unchanged from the source buffers (no arithmetic),
        // so bitwise equality IS the correct invariant — we just
        // need to spell it out.
        fn arr_eq(a: [f32; 3], b: [f32; 3]) -> bool {
            a[0].to_bits() == b[0].to_bits()
                && a[1].to_bits() == b[1].to_bits()
                && a[2].to_bits() == b[2].to_bits()
        }
        let m = Mesh3D::pyramid(1.34, 1.25);
        let buf = m.vertex_buffer();
        assert_eq!(buf.len(), m.positions.len());
        for (i, v) in buf.iter().enumerate() {
            assert!(arr_eq(v.position, m.positions[i]));
            assert!(arr_eq(v.normal, m.normals[i]));
        }
    }

    #[test]
    fn vertex3d_size_matches_wgsl_vertex_layout() {
        // 6 × f32 = 24 bytes. attributes use Float32x3 + Float32x3,
        // total stride 24.
        assert_eq!(std::mem::size_of::<Vertex3D>(), 24);
    }
}
