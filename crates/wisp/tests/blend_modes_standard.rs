//! Standard blend mode pixel-readback tests (Tier A + B).
//!
//! For each GPU-native [`BlendMode`], render a known-color backdrop +
//! known-color foreground via Graphics (full alpha, no premultiplication
//! complications), read back the center pixel, assert it matches the
//! expected blend math.
//!
//! Format: `Rgba8Unorm` (linear) so byte values map directly to
//! `Color::rgba` channels — the M0.11 lesson in CLAUDE.md.

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{BlendMode, Color, Fill, Graphics, RenderTexture, Stage};

const W: u32 = 32;
const H: u32 = 32;

fn render_with_blend(mode: BlendMode) -> [u8; 4] {
    let app = block_on(Application::new(AppConfig::default())).expect("app");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("renderer");
    let rt = RenderTexture::with_format(&app, W, H, format);

    // Backdrop: opaque red over the whole NDC.
    let mut bg = Graphics::new();
    bg.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));

    // Foreground: opaque blue over the whole NDC, with the test mode.
    let mut fg = Graphics::new();
    fg.fill(Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)));
    fg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    fg.container.blend_mode = mode;

    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), bg);
    let _ = stage.add_child(stage.root(), fg);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    let bytes = rt.read_pixels(&app);

    // Pull the center pixel.
    let cx = (W / 2) as usize;
    let cy = (H / 2) as usize;
    let stride = (W as usize) * 4;
    let i = cy * stride + cx * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

/// Tolerance per channel — small to catch real regressions but tolerant
/// of the exact GPU rounding (Apple Silicon Metal can disagree with a
/// reference math impl by 1 in the LSB).
const TOL: i32 = 2;

fn assert_close(actual: [u8; 4], expected: [u8; 4]) {
    for ch in 0..4 {
        let diff = i32::from(actual[ch]) - i32::from(expected[ch]);
        assert!(
            diff.abs() <= TOL,
            "channel {ch}: got {actual:?}, expected ~{expected:?} (diff {diff})"
        );
    }
}

// ─── Standard set (GPU blend equation only) ──────────────────────────

#[test]
fn normal_overwrites_with_foreground() {
    // src.a=1 + ALPHA_BLENDING → src wins.
    assert_close(render_with_blend(BlendMode::Normal), [0, 0, 255, 255]);
}

#[test]
fn multiply_darkens_to_zero() {
    // red * blue = 0 (no overlap).
    assert_close(render_with_blend(BlendMode::Multiply), [0, 0, 0, 255]);
}

#[test]
fn add_combines_to_magenta() {
    // red + blue = magenta.
    assert_close(render_with_blend(BlendMode::Add), [255, 0, 255, 255]);
}

#[test]
fn screen_lightens_to_magenta() {
    // 1 - (1-red)·(1-blue) = (1, 0, 1) at the union of disjoint channels.
    assert_close(render_with_blend(BlendMode::Screen), [255, 0, 255, 255]);
}

#[test]
fn subtract_yields_backdrop_minus_foreground() {
    // ReverseSubtract with src.a=1, dst_factor=One: dst - src.
    // red - blue = (1, 0, 0) clamped.
    assert_close(render_with_blend(BlendMode::Subtract), [255, 0, 0, 255]);
}

#[test]
fn min_picks_per_channel_minimum() {
    // min(red, blue) = (0, 0, 0).
    assert_close(render_with_blend(BlendMode::Min), [0, 0, 0, 255]);
}

#[test]
fn max_picks_per_channel_maximum() {
    // max(red, blue) = (1, 0, 1).
    assert_close(render_with_blend(BlendMode::Max), [255, 0, 255, 255]);
}

#[test]
fn erase_zeroes_destination_where_src_alpha() {
    // dst·(1 - src.a) with src.a=1 → all zeros, alpha included.
    assert_close(render_with_blend(BlendMode::Erase), [0, 0, 0, 0]);
}
