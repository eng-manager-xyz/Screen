//! AUT-20 — rectangle privacy blur. Verifies the composition
//! primitive: `base` rendered as-is outside `region`, blurred
//! version of `base` rendered inside `region`.
//!
//! Strategy: build a base image with a sharp red/blue half-and-half
//! split. After privacy blur over a rect that straddles the seam,
//! sample three pixels:
//!
//! 1. Inside the region, near the seam — pixel value should be a
//!    blend (not pure red, not pure blue).
//! 2. Inside the region, far from the seam — pixel value should still
//!    bleed toward the other half (blur is broad).
//! 3. Outside the region — pixel must match the original `base`.

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

/// Render the base scene: left half red, right half blue.
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
fn outside_region_matches_base_exactly() {
    let (app, renderer) = boot();
    let base = render_base(&app, &renderer);

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    renderer.apply_privacy_blur(&app, MaskShape::rect(region), 8.0, &base, &output);

    // Top-left pixel (NDC ~ -1, +1) is far outside the region — should
    // match the base's red.
    let outside = read_pixel(&output, &app, 4, 4);
    assert_eq!(outside[0], 255, "outside region should be pure red");
    assert_eq!(outside[2], 0, "outside region must not have blue bleed");
}

#[test]
fn inside_region_picks_up_neighbor_via_blur() {
    let (app, renderer) = boot();
    let base = render_base(&app, &renderer);

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    renderer.apply_privacy_blur(&app, MaskShape::rect(region), 12.0, &base, &output);

    // Near the seam (NDC x ≈ 0), inside the region — should have BOTH
    // red AND blue components from the blur kernel.
    let near_seam = read_pixel(&output, &app, W / 2, H / 2);
    assert!(
        near_seam[0] > 20 && near_seam[2] > 20,
        "near-seam pixel should mix red+blue, got {near_seam:?}"
    );
}

#[test]
fn deep_inside_region_still_shows_blur_falloff() {
    let (app, renderer) = boot();
    let base = render_base(&app, &renderer);

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    renderer.apply_privacy_blur(&app, MaskShape::rect(region), 16.0, &base, &output);

    // NDC (-0.4, 0) — inside region but on the red side. Even with
    // strong blur, red should still dominate (blur is not infinite).
    let red_side_inside = read_pixel(&output, &app, 38, H / 2);
    assert!(
        red_side_inside[0] > red_side_inside[2],
        "red side should still favor red after blur: {red_side_inside:?}"
    );
}
