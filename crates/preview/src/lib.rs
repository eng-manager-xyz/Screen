//! `preview` — native winit + wgpu sibling window that wisp renders into.
//!
//! The `[[bin]]` target (`src/main.rs`) opens a real OS window via winit,
//! hands its wgpu surface to [`wisp::application::Application::from_wgpu`],
//! and pumps the [`playback::Player`] into it. The library half exposes the
//! pure helpers (math, headless render harness) so `[[example]]` and tests
//! can exercise the same render path without a window.

use glam::Vec2;

/// Letterbox/pillarbox scale that fits a `video_w × video_h` source into a
/// `surface_w × surface_h` viewport, expressed as a [`Vec2`] for the wisp
/// sprite's transform `scale`.
///
/// The fully-covered axis returns `2.0` (the full NDC `[-1, +1]` range applied
/// to a default unit-quad sprite); the bound axis returns the proportional
/// fraction of `2.0`.
///
/// Returns `Vec2::splat(1.0)` when any dimension is zero — guard for edge
/// cases (window minimized, decoder reports a malformed stream) without
/// risking `NaN` in the transform.
#[must_use]
pub fn aspect_fit_scale(surface_w: u32, surface_h: u32, video_w: u32, video_h: u32) -> Vec2 {
    if surface_w == 0 || surface_h == 0 || video_w == 0 || video_h == 0 {
        return Vec2::splat(1.0);
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "surface and video dimensions fit comfortably in f32"
    )]
    let surface_aspect = surface_w as f32 / surface_h as f32;
    #[allow(
        clippy::cast_precision_loss,
        reason = "surface and video dimensions fit comfortably in f32"
    )]
    let video_aspect = video_w as f32 / video_h as f32;
    if video_aspect > surface_aspect {
        Vec2::new(2.0, 2.0 * surface_aspect / video_aspect)
    } else {
        Vec2::new(2.0 * video_aspect / surface_aspect, 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::aspect_fit_scale;
    use glam::Vec2;

    /// Equality with a tolerance — `f32` aspect-ratio math has small rounding noise.
    fn approx(a: Vec2, b: Vec2) {
        assert!(
            (a.x - b.x).abs() < 1e-4 && (a.y - b.y).abs() < 1e-4,
            "{a:?} != {b:?}"
        );
    }

    #[test]
    fn matching_aspect_fills_full_ndc() {
        approx(aspect_fit_scale(1920, 1080, 1920, 1080), Vec2::splat(2.0));
        approx(aspect_fit_scale(1000, 1000, 512, 512), Vec2::splat(2.0));
    }

    #[test]
    fn wider_video_letterboxes_top_and_bottom() {
        // 16:9 video into a square surface — full width, shorter height.
        let scale = aspect_fit_scale(1000, 1000, 1920, 1080);
        approx(scale, Vec2::new(2.0, 2.0 * (1.0_f32 / (16.0 / 9.0))));
        assert!(scale.y < scale.x, "letterbox: y < x");
    }

    #[test]
    fn taller_video_pillarboxes_left_and_right() {
        // Portrait 9:16 video into a square surface — full height, narrower width.
        let scale = aspect_fit_scale(1000, 1000, 1080, 1920);
        approx(scale, Vec2::new(2.0 * (9.0 / 16.0), 2.0));
        assert!(scale.x < scale.y, "pillarbox: x < y");
    }

    #[test]
    fn zero_dimensions_return_identity() {
        approx(aspect_fit_scale(0, 1080, 1920, 1080), Vec2::splat(1.0));
        approx(aspect_fit_scale(1920, 0, 1920, 1080), Vec2::splat(1.0));
        approx(aspect_fit_scale(1920, 1080, 0, 1080), Vec2::splat(1.0));
        approx(aspect_fit_scale(1920, 1080, 1920, 0), Vec2::splat(1.0));
    }
}
