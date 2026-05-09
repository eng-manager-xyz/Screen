//! `Filter` trait + built-in filters.
//!
//! Filters operate on `RenderTexture`s. Each filter declares how many passes
//! it needs; the orchestrator on `Renderer::apply_filter` allocates a scratch
//! `RenderTexture` when needed and ping-pongs.
//!
//! Built-ins:
//! - [`blur::BlurFilter`] — separable Gaussian (M0.16)
//! - `drop_shadow::DropShadowFilter` — alpha extract + blur + composite (M0.17)
//! - `motion_blur::MotionBlurFilter` — velocity-driven directional blur (M0.18)
//! - `color_matrix::ColorMatrixFilter` — 4×5 RGBA matrix (M0.18)

pub mod blur;
pub mod color_matrix;
pub mod drop_shadow;
pub mod motion_blur;

pub use blur::BlurFilter;
pub use color_matrix::ColorMatrixFilter;
pub use drop_shadow::DropShadowFilter;
pub use motion_blur::MotionBlurFilter;

use crate::application::Application;
use crate::texture::render_texture::RenderTexture;

/// Per-filter render context — provides the wgpu device / queue / encoder
/// needed to enqueue draw work for a filter pass.
pub struct FilterContext<'a> {
    pub app: &'a Application,
    pub encoder: &'a mut wgpu::CommandEncoder,
}

/// A renderable post-process filter applied to a `RenderTexture`.
pub trait Filter: Send + Sync {
    /// Number of passes (`>= 1`). Multi-pass filters ping-pong between two
    /// render targets across the orchestrator.
    fn passes(&self) -> u32;

    /// Render one pass: read from `input`, write to `output`. The orchestrator
    /// invokes this once per pass index `[0, passes())`.
    fn render_pass(
        &self,
        ctx: &mut FilterContext<'_>,
        input: &RenderTexture,
        output: &RenderTexture,
        pass: u32,
    );
}
