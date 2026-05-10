//! AUT-34 — ellipse mask. Adds `MaskShape::Ellipse` and validates the
//! anisotropic SDF branch in `clip.wgsl`.
//!
//! Three contracts:
//! 1. Center pixel of the ellipse passes through (inside SDF).
//! 2. A point inside the bounding rect but with `(x/a)^2 + (y/b)^2 > 1`
//!    is carved away (outside SDF). Pick a clearly-anisotropic
//!    ellipse so this differs from a circle's behavior.
//! 3. Ellipse plugs into `apply_solid_redaction` like the other
//!    shapes (proves enum dispatch works end-to-end).

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
fn ellipse_center_passes_through_clip() {
    let (app, renderer) = boot();
    let fg = render_white_foreground(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    // Wide ellipse — a=0.7, b=0.3.
    let ellipse = MaskShape::ellipse(glam::Vec2::ZERO, glam::Vec2::new(0.7, 0.3));
    renderer.apply_clip(&app, ellipse, &fg, &output);

    let center = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(center[0], 255, "ellipse center should pass white through");
    assert_eq!(center[3], 255, "ellipse center should be opaque");
}

#[test]
fn ellipse_inside_bounds_outside_curve_is_carved_away() {
    let (app, renderer) = boot();
    let fg = render_white_foreground(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    // Wide ellipse a=0.7, b=0.3. (NDC 0.0, +0.45) — b is 0.3 so y=0.45
    // is well above the top of the ellipse (|y/b| = 1.5). x is 0 so
    // (x/a)^2 = 0. (x/a)^2 + (y/b)^2 = 0 + 2.25 = 2.25 > 1 → outside
    // ellipse, but inside the [-0.7, +0.7] × [-0.3, +0.3] bounding
    // rect (whoops — actually y=0.45 is also outside the bounding
    // rect since b=0.3, so let's pick a point that's INSIDE the
    // bounds-from-corner-axis but outside the curve).
    //
    // Instead pick (0.6, 0.25): bounding rect is [-0.7, +0.7] ×
    // [-0.3, +0.3] so (0.6, 0.25) is inside. (0.6/0.7)^2 = 0.735;
    // (0.25/0.3)^2 = 0.694; sum = 1.43 > 1 → outside the ellipse.
    //
    // NDC (0.6, 0.25) → ((0.6+1)/2)*128 = 102, ((1-0.25)/2)*128 = 48.
    let ellipse = MaskShape::ellipse(glam::Vec2::ZERO, glam::Vec2::new(0.7, 0.3));
    renderer.apply_clip(&app, ellipse, &fg, &output);

    let outside_curve = read_pixel(&output, &app, 102, 48);
    assert_eq!(
        outside_curve[3], 0,
        "inside-bounds-outside-curve pixel must be alpha 0, got {outside_curve:?}"
    );
}

#[test]
fn ellipse_plugs_into_solid_redaction() {
    let (app, renderer) = boot();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let base = RenderTexture::with_format(&app, W, H, format);
    let mut stage = Stage::new();
    let mut bg = Graphics::new();
    bg.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), bg);
    let _ = renderer.render_stage(&app, base.view(), Color::BLACK, &stage);

    let output = RenderTexture::with_format(&app, W, H, format);
    let ellipse = MaskShape::ellipse(glam::Vec2::ZERO, glam::Vec2::new(0.6, 0.4));
    let redact = Color::rgba_u8(40, 40, 200, 255);
    renderer.apply_solid_redaction(&app, ellipse, redact, &base, &output);

    // Center inside the ellipse → redaction color.
    let inside = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(inside[2], 200, "center should be redaction blue");
    assert_eq!(inside[0], 40, "center should be redaction R");

    // Top-left → outside the ellipse (and outside the bounding rect)
    // → base red.
    let outside = read_pixel(&output, &app, 4, 4);
    assert_eq!(outside[0], 255, "outside ellipse should still be base red");
}
