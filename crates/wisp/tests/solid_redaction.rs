//! AUT-23 — solid-color redaction. Companion to AUT-20/21/22 privacy
//! blur. The contract:
//!
//! 1. Pixels outside the shape match `base` bit-exactly.
//! 2. Pixels well inside the shape are exactly the redaction `color`.
//! 3. Both `MaskShape::Rect` and `MaskShape::RoundedRect` work — the
//!    same primitive handles both shapes (just like privacy blur).
//! 4. For a rounded shape, pixels in the bounding rect but outside the
//!    rounded corner show `base` (the SDF carves the corner cleanly).

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

/// All-red base scene.
fn render_base(app: &Application, renderer: &Renderer) -> RenderTexture {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let base = RenderTexture::with_format(app, W, H, format);
    let mut stage = Stage::new();
    let mut bg = Graphics::new();
    bg.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), bg);
    let _ = renderer.render_stage(app, base.view(), Color::BLACK, &stage);
    base
}

#[test]
fn rect_redaction_outside_matches_base() {
    let (app, renderer) = boot();
    let base = render_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    renderer.apply_solid_redaction(
        &app,
        MaskShape::rect(region),
        Color::rgba(0.0, 1.0, 0.0, 1.0),
        &base,
        &output,
    );

    // Top-left pixel — far outside the region.
    let outside = read_pixel(&output, &app, 4, 4);
    assert_eq!(outside[0], 255, "outside should still be base red");
    assert_eq!(outside[1], 0, "outside should have no green");
}

#[test]
fn rect_redaction_inside_is_exactly_color() {
    let (app, renderer) = boot();
    let base = render_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    let redact = Color::rgba_u8(7, 200, 64, 255);
    renderer.apply_solid_redaction(&app, MaskShape::rect(region), redact, &base, &output);

    // Center of the region — must be the exact redaction color.
    let inside = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(inside[0], 7, "R channel should be redaction color");
    assert_eq!(inside[1], 200, "G channel should be redaction color");
    assert_eq!(inside[2], 64, "B channel should be redaction color");
    assert_eq!(inside[3], 255, "alpha should be opaque");
}

#[test]
fn rounded_redaction_corner_carved_away() {
    let (app, renderer) = boot();
    let base = render_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    let radius = 0.3;
    renderer.apply_solid_redaction(
        &app,
        MaskShape::rounded_rect(region, radius),
        Color::rgba(0.0, 0.0, 0.0, 1.0),
        &base,
        &output,
    );

    // Same coordinate as AUT-21's rounded test — inside the bounding
    // rect, outside the rounded corner. Should still be base red, NOT
    // black.
    let corner = read_pixel(&output, &app, 36, 36);
    assert_eq!(
        corner[0], 255,
        "rounded-corner cutout should still be base red, got {corner:?}"
    );
    assert_eq!(
        corner[1], 0,
        "rounded-corner cutout should have no green/blue, got {corner:?}"
    );
}

#[test]
fn rounded_redaction_center_is_color() {
    let (app, renderer) = boot();
    let base = render_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    let radius = 0.2;
    let redact = Color::rgba_u8(20, 20, 20, 255);
    renderer.apply_solid_redaction(
        &app,
        MaskShape::rounded_rect(region, radius),
        redact,
        &base,
        &output,
    );

    let inside = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(inside[0], 20, "R should be redaction color");
    assert_eq!(inside[1], 20, "G should be redaction color");
    assert_eq!(inside[2], 20, "B should be redaction color");
}
