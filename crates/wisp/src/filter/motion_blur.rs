//! `MotionBlurFilter` — directional Gaussian blur driven by a velocity vector.
//!
//! Reuses [`crate::filter::blur::run_blur_pass`] with an arbitrary direction
//! instead of the axis-aligned `(1,0)` / `(0,1)` used by [`BlurFilter`].

use crate::filter::{Filter, FilterContext};
use crate::texture::render_texture::RenderTexture;

/// Velocity-driven directional blur.
///
/// `velocity` is in primitive-local pixels per frame; the kernel size scales
/// with `velocity.length() / peak_velocity_pps` clamped at `max_kernel_px`.
/// Constants `peak = 1400` and `max = 14` are lifted from `OpenScreen` v1.4
/// `zoomTransform.ts` defaults — they feel right for cursor zoom motion.
#[derive(Debug, Clone, Copy)]
pub struct MotionBlurFilter {
    /// Direction + magnitude, in primitive-local pixels per frame.
    pub velocity: glam::Vec2,
    /// Reference speed at which the kernel reaches `max_kernel_px`.
    pub peak_velocity_pps: f32,
    /// Upper bound on per-tap displacement.
    pub max_kernel_px: f32,
}

impl Default for MotionBlurFilter {
    fn default() -> Self {
        Self {
            velocity: glam::Vec2::ZERO,
            peak_velocity_pps: 1400.0,
            max_kernel_px: 14.0,
        }
    }
}

impl MotionBlurFilter {
    fn kernel_pixels(&self) -> f32 {
        let speed = self.velocity.length();
        (speed / self.peak_velocity_pps).clamp(0.0, 1.0) * self.max_kernel_px
    }

    fn unit_direction(&self) -> [f32; 2] {
        if self.velocity.length_squared() < 1e-6 {
            [1.0, 0.0]
        } else {
            let n = self.velocity.normalize();
            [n.x, n.y]
        }
    }
}

impl Filter for MotionBlurFilter {
    fn passes(&self) -> u32 {
        1
    }

    fn render_pass(
        &self,
        ctx: &mut FilterContext<'_>,
        input: &RenderTexture,
        output: &RenderTexture,
        _pass: u32,
    ) {
        let radius = self.kernel_pixels();
        if radius < 0.05 {
            // Below threshold — fall back to a copy with a 0-radius blur.
            super::blur::run_blur_pass(ctx.app, ctx.encoder, input, output, [1.0, 0.0], 0.0);
            return;
        }
        super::blur::run_blur_pass(
            ctx.app,
            ctx.encoder,
            input,
            output,
            self.unit_direction(),
            radius,
        );
    }
}
