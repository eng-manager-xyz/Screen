//! AUT-28 — highlight / spotlight mask. Inside the shape: base shows
//! through unchanged. Outside the shape: base is partially replaced
//! by `dim_color`, so a mid-gray dim overlay produces a darkened
//! variant of the surrounding content.
//!
//! Three contracts:
//! 1. Pixel inside the shape == base bit-exactly.
//! 2. Pixel outside the shape is darker (lower per-channel) than base
//!    when `dim_color` is opaque black.
//! 3. `RoundedRect` spotlight: pixel inside the bounding rect but
//!    outside the rounded corner is *darkened* (treated as outside
//!    the focus shape).

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

fn render_red_base(app: &Application, renderer: &Renderer) -> RenderTexture {
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
fn inside_shape_matches_base_exactly() {
    let (app, renderer) = boot();
    let base = render_red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    renderer.apply_spotlight(
        &app,
        MaskShape::rect(region),
        Color::rgba(0.0, 0.0, 0.0, 0.7),
        &base,
        &output,
    );

    // Center of the shape — base must show through unchanged.
    let inside = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(inside[0], 255, "inside spotlight should be base red");
    assert_eq!(inside[1], 0, "no green leak");
    assert_eq!(inside[2], 0, "no blue leak");
}

#[test]
fn outside_shape_is_darkened_by_overlay() {
    let (app, renderer) = boot();
    let base = render_red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    // Opaque black with alpha 0.7 → pixel red goes from 255 to ~76.
    renderer.apply_spotlight(
        &app,
        MaskShape::rect(region),
        Color::rgba(0.0, 0.0, 0.0, 0.7),
        &base,
        &output,
    );

    // Top-left pixel (NDC ~ -1, +1) — far outside region.
    let outside = read_pixel(&output, &app, 4, 4);
    assert!(
        outside[0] < 200,
        "outside should be visibly darkened, got R={}",
        outside[0]
    );
    assert!(
        outside[0] > 0,
        "but should NOT be pure black — base red still mixes in, got R={}",
        outside[0]
    );
}

#[test]
fn rounded_corner_treated_as_outside() {
    let (app, renderer) = boot();
    let base = render_red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    let radius = 0.3;
    renderer.apply_spotlight(
        &app,
        MaskShape::rounded_rect(region, radius),
        Color::rgba(0.0, 0.0, 0.0, 0.85),
        &base,
        &output,
    );

    // Inside the bounding rect but outside the rounded corner —
    // should be DARKENED (this corner was carved away from the
    // spotlight, so it's "outside" the focus shape).
    let corner = read_pixel(&output, &app, 36, 36);
    assert!(
        corner[0] < 100,
        "rounded-corner cutout should be darkened (outside shape), got R={}",
        corner[0]
    );

    // Center of the rounded shape — base shows through unchanged.
    let center = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(
        center[0], 255,
        "rounded-shape center should be base red, got {center:?}"
    );
}
