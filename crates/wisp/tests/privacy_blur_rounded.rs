//! AUT-21 — rounded-rectangle privacy blur. Generalizes the AUT-20
//! rect primitive to a `MaskShape::RoundedRect`.
//!
//! Strategy: same red/blue split as the rect tests, but the privacy
//! region has a corner radius. Sample three contract pixels:
//!
//! 1. Outside the bounding rect entirely — must equal `base` exactly.
//! 2. Inside the bounding rect but *outside* the rounded corner —
//!    must also equal `base` (the SDF carved the corner away).
//! 3. Center of the rounded rect, on the seam — must mix R+B (blur
//!    actually ran inside the rounded region).
//!
//! The bounding-rect-but-corner test is the new contract AUT-21 adds
//! over AUT-20: the rounded shape must be a strict subset of the
//! bounding rect.

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

fn render_base(app: &Application, renderer: &Renderer) -> RenderTexture {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let base = RenderTexture::with_format(app, W, H, format);
    let mut stage = Stage::new();

    let mut left = Graphics::new();
    left.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
    left.draw_rect(Rect::new(-1.0, -1.0, 1.0, 2.0));
    let _ = stage.add_child(stage.root(), left);

    let mut right = Graphics::new();
    right.fill(Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)));
    right.draw_rect(Rect::new(0.0, -1.0, 1.0, 2.0));
    let _ = stage.add_child(stage.root(), right);

    let _ = renderer.render_stage(app, base.view(), Color::BLACK, &stage);
    base
}

#[test]
fn rounded_outside_bounding_rect_matches_base() {
    let (app, renderer) = boot();
    let base = render_base(&app, &renderer);

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    let radius = 0.2;
    renderer.apply_privacy_blur(
        &app,
        MaskShape::rounded_rect(region, radius),
        8.0,
        &base,
        &output,
    );

    // Top-left pixel — way outside the region, on the red side.
    let outside = read_pixel(&output, &app, 4, 4);
    assert_eq!(outside[0], 255, "outside region should be pure red");
    assert_eq!(outside[2], 0, "outside region must not have blue bleed");
}

#[test]
fn rounded_corner_carved_away_matches_base() {
    let (app, renderer) = boot();
    let base = render_base(&app, &renderer);

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    let radius = 0.3;
    renderer.apply_privacy_blur(
        &app,
        MaskShape::rounded_rect(region, radius),
        12.0,
        &base,
        &output,
    );

    // The top-left of the bounding rect (NDC -0.5, +0.5) is well
    // outside the rounded shape — the SDF has carved that corner away.
    // Distance from corner (-0.5, +0.5) to the rounded-rect's
    // round-corner center (-0.2, +0.2): sqrt(0.09 + 0.09) ~ 0.42 > 0.3.
    // So this pixel must read pure red (the base color there) — the
    // privacy blur did NOT touch it.
    //
    // NDC (-0.5, +0.5) → (32, 32) at 128x128 (NDC y +0.5 → row near
    // top, since base render flips y in the standard way; bias inwards
    // a few pixels to escape the AA band).
    let corner = read_pixel(&output, &app, 36, 36);
    assert_eq!(
        corner[0], 255,
        "rounded-corner cutout should still be base red, got {corner:?}"
    );
    assert_eq!(
        corner[2], 0,
        "rounded-corner cutout should have no blue bleed, got {corner:?}"
    );
}

#[test]
fn rounded_center_seam_picks_up_neighbor_via_blur() {
    let (app, renderer) = boot();
    let base = render_base(&app, &renderer);

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    let radius = 0.2;
    renderer.apply_privacy_blur(
        &app,
        MaskShape::rounded_rect(region, radius),
        12.0,
        &base,
        &output,
    );

    // Center of the region, on the seam — well inside the rounded
    // shape (distance from the seam to nearest rounded edge >> 0).
    // Should mix red and blue from the blur kernel.
    let center = read_pixel(&output, &app, W / 2, H / 2);
    assert!(
        center[0] > 20 && center[2] > 20,
        "center-seam pixel should mix red+blue via blur, got {center:?}"
    );
}
