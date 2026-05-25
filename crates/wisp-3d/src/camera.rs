//! `Camera3D` — perspective camera with view + projection matrices
//! (W3D.1 / AUT-293).
//!
//! ## Conventions
//!
//! - Right-handed (matches glam's `look_at_rh` / `perspective_rh_gl`).
//! - wgpu NDC depth range is `[0, 1]` (not `[-1, 1]` like OpenGL).
//!   `glam::Mat4::perspective_rh_gl` produces `[-1, 1]` clip-space Z
//!   — wgpu accepts this but wastes half the depth precision. We use
//!   `perspective_rh` (which produces `[0, 1]`) so the depth buffer
//!   gets full precision out of the box.
//! - `fov_y` stored as radians; the `perspective(fov_deg, …)`
//!   constructor takes degrees to match THREE.js's call shape
//!   (`PerspectiveCamera(38, …)`).
//!
//! ## GPU-side
//!
//! [`ViewProj`] is the uniform-buffer-object struct uploaded to
//! WGSL. It includes view, projection, and combined `view_proj`
//! (precomputed to save a multiply per vertex) plus the camera world
//! position (handy for fragment-side specular / rim terms). The
//! layout is `#[repr(C, align(16))]` per CLAUDE.md "WGSL ↔ Rust
//! uniform layout" — every field is naturally 16-byte aligned, so
//! no manual padding is needed today, but the explicit `align(16)`
//! makes the requirement load-bearing if a future field changes.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// Perspective camera. Mirrors THREE.js's `PerspectiveCamera` shape
/// (fov in degrees + aspect + near + far) plus a position / target /
/// up triple for the view matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera3D {
    /// Vertical field of view, **radians**. Use [`Camera3D::perspective`]
    /// to construct from degrees.
    pub fov_y: f32,
    /// Aspect ratio (width / height). Update via [`Camera3D::update_aspect`]
    /// on resize.
    pub aspect: f32,
    /// Near clip plane (must be > 0 for a valid projection).
    pub near: f32,
    /// Far clip plane (must be > `near`).
    pub far: f32,
    /// Camera position in world space.
    pub position: Vec3,
    /// Point the camera looks at.
    pub target: Vec3,
    /// World-space up direction (typically `Vec3::Y`).
    pub up: Vec3,
}

impl Camera3D {
    /// Build a perspective camera from degrees (matches THREE.js's
    /// `new PerspectiveCamera(fov_deg, aspect, near, far)` shape).
    ///
    /// Position defaults to `(0, 0, 5)` looking at the origin with
    /// `Vec3::Y` up — override via the public fields after construction.
    #[must_use]
    pub fn perspective(fov_deg: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            fov_y: fov_deg.to_radians(),
            aspect,
            near,
            far,
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
        }
    }

    /// View matrix (world → camera). Right-handed `look_at`.
    #[must_use]
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Projection matrix (camera → wgpu clip space, depth `[0, 1]`).
    #[must_use]
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far)
    }

    /// Combined `projection × view`, computed once for the UBO so the
    /// vertex shader doesn't multiply per-vertex.
    #[must_use]
    pub fn view_proj(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Update the aspect ratio. Call on window resize. Doesn't move
    /// the camera or change FOV; only the projection matrix shifts.
    pub fn update_aspect(&mut self, width: u32, height: u32) {
        let h = height.max(1);
        // Per CLAUDE.md "Cast hygiene" — u32 → f32 is precision-lossy
        // for very large values, but window dimensions are bounded
        // well below 2^23.
        #[allow(
            clippy::cast_precision_loss,
            reason = "viewport dimensions in pixels are bounded < 2^23; f32 is exact"
        )]
        let aspect = (width as f32) / (h as f32);
        self.aspect = aspect.max(0.01);
    }

    /// Build the GPU-side uniform snapshot.
    #[must_use]
    pub fn view_proj_uniform(&self) -> ViewProj {
        ViewProj {
            view: self.view_matrix().to_cols_array_2d(),
            proj: self.projection_matrix().to_cols_array_2d(),
            view_proj: self.view_proj().to_cols_array_2d(),
            camera_pos: [self.position.x, self.position.y, self.position.z, 1.0],
        }
    }
}

/// GPU-side camera uniform.
///
/// `#[repr(C, align(16))]` because WGSL `mat4x4<f32>` is 16-byte
/// aligned and `vec4<f32>` is 16-byte aligned. Every field here is
/// naturally a multiple of 16; the explicit `align(16)` documents
/// the invariant. See CLAUDE.md "WGSL ↔ Rust uniform layout".
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ViewProj {
    /// World → camera.
    pub view: [[f32; 4]; 4],
    /// Camera → clip space.
    pub proj: [[f32; 4]; 4],
    /// `proj * view`, precomputed.
    pub view_proj: [[f32; 4]; 4],
    /// Camera world position, padded to `vec4` with `w = 1.0` for the
    /// WGSL `vec4<f32>` layout. Useful in fragment shaders that need
    /// world-space viewing direction (specular, rim, etc.).
    pub camera_pos: [f32; 4],
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn view_proj_is_invertible_for_unit_camera() {
        // A camera at (0, 0, 5) looking at origin with a standard FOV
        // must produce an invertible view_proj. Singular matrices are
        // a sign of degenerate near/far or zero aspect.
        let cam = Camera3D::perspective(45.0, 16.0 / 9.0, 0.1, 100.0);
        let m = cam.view_proj();
        let det = m.determinant();
        assert!(
            det.abs() > 1e-6,
            "view_proj is singular (det={det}); inputs were invalid"
        );
        // And its inverse round-trips.
        let inv = m.inverse();
        let identity = m * inv;
        let id = Mat4::IDENTITY;
        for c in 0..4 {
            for r in 0..4 {
                assert!(
                    approx_eq(identity.col(c)[r], id.col(c)[r], 1e-4),
                    "view_proj * inv != identity at ({c},{r})"
                );
            }
        }
    }

    #[test]
    fn update_aspect_does_not_move_camera() {
        let mut cam = Camera3D::perspective(38.0, 1.0, 0.1, 100.0);
        cam.position = Vec3::new(0.0, 0.28, 6.2); // 404 page values.
        let pos_before = cam.position;
        let target_before = cam.target;
        cam.update_aspect(1920, 1080);
        assert_eq!(cam.position, pos_before);
        assert_eq!(cam.target, target_before);
        assert!(
            approx_eq(cam.aspect, 1920.0 / 1080.0, 1e-6),
            "aspect not set: {}",
            cam.aspect
        );
    }

    #[test]
    fn update_aspect_handles_zero_height_safely() {
        // Division by zero would NaN the projection; clamping height
        // to >=1 keeps the matrix finite.
        let mut cam = Camera3D::perspective(45.0, 1.0, 0.1, 100.0);
        cam.update_aspect(1920, 0);
        assert!(cam.aspect.is_finite());
        assert!(cam.aspect > 0.0);
    }

    #[test]
    fn lookat_at_origin_from_z_axis_uses_negative_z_view_dir() {
        // Camera at (0, 0, 5) looking at (0, 0, 0): the view direction
        // is -Z in world space. After applying the view matrix, the
        // origin lands at (0, 0, -5) in camera space (camera looks
        // down its own -Z).
        let cam = Camera3D::perspective(45.0, 1.0, 0.1, 100.0);
        let view = cam.view_matrix();
        let origin_in_view = view * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
        assert!(approx_eq(origin_in_view.x, 0.0, 1e-6));
        assert!(approx_eq(origin_in_view.y, 0.0, 1e-6));
        assert!(approx_eq(origin_in_view.z, -5.0, 1e-6));
    }

    #[test]
    fn view_proj_uniform_round_trips_matrices() {
        let cam = Camera3D::perspective(38.0, 16.0 / 9.0, 0.1, 100.0);
        let u = cam.view_proj_uniform();
        let view = Mat4::from_cols_array_2d(&u.view);
        let proj = Mat4::from_cols_array_2d(&u.proj);
        let vp = Mat4::from_cols_array_2d(&u.view_proj);
        assert!(view.abs_diff_eq(cam.view_matrix(), 1e-6));
        assert!(proj.abs_diff_eq(cam.projection_matrix(), 1e-6));
        assert!(vp.abs_diff_eq(cam.view_proj(), 1e-6));
        assert!(approx_eq(u.camera_pos[3], 1.0, 1e-9));
    }

    #[test]
    fn view_proj_uniform_size_matches_wgsl_layout() {
        // 3 × mat4x4<f32> (192 bytes) + 1 × vec4<f32> (16 bytes) = 208.
        assert_eq!(std::mem::size_of::<ViewProj>(), 208);
        assert_eq!(std::mem::align_of::<ViewProj>(), 16);
    }
}
