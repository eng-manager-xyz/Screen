//! `wisp-3d` — real-3D rendering on top of the same wgpu device
//! that `wisp` uses for its 2D scene graph.
//!
//! ## Why a separate crate
//!
//! `wisp` is a Pixi-equivalent 2D library. Its render-stage contract
//! collects nodes in scene-graph order, batches per pipeline type, and
//! deliberately has **no depth buffer** — the back-to-front order
//! comes from `scene::Stage` insertion order plus the filter chain's
//! deterministic last-wins semantics. Adding a Z buffer would break
//! the filter / mask contract and the "two filters compose
//! independently of registration order" invariant.
//!
//! 3D scenes don't fit that model. A `PerspectiveCamera` needs view +
//! projection matrices. An indexed mesh needs per-vertex normals. A
//! spinning pyramid needs Z-test so its back face doesn't paint over
//! its front face. `wisp-3d` introduces those primitives in a sibling
//! crate so the 2D contract stays clean.
//!
//! ## Public API
//!
//! - [`Camera3D`] — perspective camera (view + projection matrices).
//! - [`Mesh3D`] — indexed positions + normals + a `pyramid()`
//!   constructor matching the engmanager.xyz 404 layout.
//!
//! ## Stage of work
//!
//! W3D.0 — crate skeleton. W3D.1..W3D.9 land subsequently per the
//! Linear `wisp-3d` project (AUT-292..AUT-301).

#![doc(html_root_url = "https://docs.rs/wisp-3d/0.1.0")]

pub use wisp::application::Application;

pub mod camera;
pub mod edges;
pub mod material;
pub mod mesh;
pub mod render;
pub mod sprite;
pub use camera::{Camera3D, ViewProj};
pub use edges::{Edge3D, EdgesMesh, LineColor, WireframePipeline};
pub use material::{Material3D, MaterialRenderer, PaletteRampMaterial, PaletteUniform};
pub use mesh::{Mesh3D, Vertex3D};
pub use render::{DEPTH_FORMAT, ModelUniform, Render3DPass};
pub use sprite::{Sprite3D, SpritePipeline};

// W3D.1+ modules land in the next commits on this branch. Keep the
// module tree empty for W3D.0 so the gate has something concrete to
// check ("the crate compiles, links to wisp, and exposes the
// Application re-export").

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_reexport_is_wisp_application() {
        // Anchor test for W3D.0: the public `wisp_3d::Application`
        // alias resolves to `wisp::application::Application`. Catches
        // accidental shadowing or moved re-exports.
        fn _accepts_wisp_app(_: Application) {}
        fn _accepts_wisp_native(_: wisp::application::Application) {
            // The two type paths must be identical.
        }
        // (Compile-time anchor only — instantiating Application
        // requires a wgpu device, out of scope for W3D.0.)
    }
}
