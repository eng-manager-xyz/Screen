//! AUT-30 — circle mask. Validates `MaskShape::Circle` plugs into the
//! same `apply_clip` / `apply_privacy_blur` / `apply_solid_redaction`
//! machinery as the existing rect / rounded-rect shapes.
//!
//! Three contracts:
//! 1. Center pixel = inside circle = passes through.
//! 2. Pixel inside the bounding box but outside the circle radius =
//!    carved away by the SDF (alpha 0). For an `apply_clip` call this
//!    means the pixel reads as the clear color (transparent).
//! 3. The circle is usable inside `apply_solid_redaction` exactly
//!    like other shapes — center is the redaction color.

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, MaskShape, RenderTexture, Stage};

const W: u32 = 128;
const H: u32 = 128;

fn boot() -> (Application, Renderer) {
    let app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("app");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("renderer");
    (app, renderer)
}

fn read_pixel(rt: &RenderTexture, app: &Application, x: u32, y: u32) -> [u8; 4] {
    let bytes = rt.read_pixels(app);
    let stride = (W as usize) * 4;
    let i = (y as usize) * stride + (x as usize) * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

/// All-white foreground for clip tests (so we can tell pass-through
/// from the transparent clear color).
fn render_white_foreground(app: &Application, renderer: &Renderer) -> RenderTexture {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let fg = RenderTexture::with_format(app, W, H, format);
    let mut stage = Stage::new();
    let mut bg = Graphics::new();
    bg.fill(Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)));
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), bg);
    let _ = renderer.render_stage(app, fg.view(), Color::TRANSPARENT, &stage);
    fg
}

#[test]
fn circle_center_passes_through_clip() {
    let (app, renderer) = boot();
    let fg = render_white_foreground(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let circle = MaskShape::circle(glam::Vec2::ZERO, 0.5);
    renderer.apply_clip(&app, circle, &fg, &output);

    let center = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(center[0], 255, "circle center should pass white through");
    assert_eq!(center[3], 255, "circle center should be opaque");
}

#[test]
fn circle_corner_of_bounds_is_carved_away() {
    let (app, renderer) = boot();
    let fg = render_white_foreground(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let circle = MaskShape::circle(glam::Vec2::ZERO, 0.5);
    renderer.apply_clip(&app, circle, &fg, &output);

    // (NDC -0.45, +0.45) is inside the bounding square (extents
    // [-0.5, +0.5]²) but distance to center = sqrt(0.2025 + 0.2025) =
    // 0.636 > 0.5 — outside the circle radius. Should read clear
    // (transparent) — the SDF carved this away.
    //
    // NDC (-0.45, +0.45) → (35, 35) at 128² (NDC y +0.45 maps to row
    // (1 - 0.45)/2 * 128 ≈ 35).
    let corner = read_pixel(&output, &app, 35, 35);
    assert_eq!(
        corner[3], 0,
        "bounding-corner pixel must be alpha 0 (carved by SDF), got {corner:?}"
    );
}

#[test]
fn circle_works_inside_solid_redaction() {
    let (app, renderer) = boot();
    // Build an all-red base.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let base = RenderTexture::with_format(&app, W, H, format);
    let mut stage = Stage::new();
    let mut bg = Graphics::new();
    bg.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), bg);
    let _ = renderer.render_stage(&app, base.view(), Color::BLACK, &stage);

    let output = RenderTexture::with_format(&app, W, H, format);
    let circle = MaskShape::circle(glam::Vec2::ZERO, 0.5);
    let redact = Color::rgba_u8(0, 200, 0, 255);
    renderer.apply_solid_redaction(&app, circle, redact, &base, &output);

    // Center inside the circle = redaction color.
    let inside = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(inside[1], 200, "circle center should be redaction green");
    assert_eq!(inside[0], 0, "no red leak inside the circle");

    // Top-left corner = far outside the circle = base red.
    let outside = read_pixel(&output, &app, 4, 4);
    assert_eq!(
        outside[0], 255,
        "outside the circle should still be base red"
    );
}
