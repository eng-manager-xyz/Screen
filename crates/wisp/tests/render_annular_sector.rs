//! AUT-224 / M-VEC.20 — `Graphics::draw_arc` + `draw_annular_sector`
//! SDF primitive rendering tests.
//!
//! Validates the three shapes a chart consumer needs:
//! * full disc (`r_inner = 0`, full angular span) — pie chart baseline,
//! * 90° wedge (partial span) — pie slice / sunburst segment,
//! * donut band (`r_inner > 0`, full span) — gauge / donut hole.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "viewport dims and pixel coords are bounded by W=128, H=128 — \
              well below the f32 precision boundary; pixel coords stay non-negative."
)]

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, RenderTexture, Stage};

const W: u32 = 128;
const H: u32 = 128;

fn boot() -> Application {
    block_on(Application::new(AppConfig::default())).expect("init wisp")
}

fn renderer(app: &Application, rt: &RenderTexture) -> Renderer {
    Renderer::new(app, rt.format()).expect("renderer")
}

fn read_pixel(bytes: &[u8], x: u32, y: u32) -> [u8; 4] {
    let stride = (W * 4) as usize;
    let i = (y as usize) * stride + (x as usize) * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

/// Convert NDC `[-1, 1]` to pixel coords. `+Y` in NDC is up; pixel
/// `+Y` is down — so y flips.
fn ndc_to_px(x: f32, y: f32) -> (u32, u32) {
    let px = (f32::midpoint(x, 1.0) * W as f32).clamp(0.0, (W - 1) as f32);
    let py = ((1.0 - y) * 0.5 * H as f32).clamp(0.0, (H - 1) as f32);
    (px as u32, py as u32)
}

#[test]
fn full_disc_fills_centre() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = renderer(&app, &rt);

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(255, 0, 0, 255)));
    g.draw_annular_sector(glam::Vec2::ZERO, 0.0, 0.5, 0.0, std::f32::consts::TAU);
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    let bytes = rt.read_pixels(&app);

    // Centre pixel = inside disc → red.
    let (cx, cy) = ndc_to_px(0.0, 0.0);
    let centre = read_pixel(&bytes, cx, cy);
    assert!(
        centre[0] > 200 && centre[1] < 30 && centre[2] < 30,
        "centre of full disc should be opaque red, got {centre:?}"
    );

    // Outside the disc → background (black).
    let (ox, oy) = ndc_to_px(0.85, 0.85);
    let outside = read_pixel(&bytes, ox, oy);
    assert!(
        outside[0] < 30,
        "outside disc should be near-black, got {outside:?}"
    );
}

#[test]
fn quarter_wedge_fills_only_its_quadrant() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = renderer(&app, &rt);

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(0, 255, 0, 255)));
    // Wedge from 0 to π/2 (upper-right in NDC since +Y up).
    g.draw_annular_sector(glam::Vec2::ZERO, 0.0, 0.6, 0.0, std::f32::consts::FRAC_PI_2);
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    let bytes = rt.read_pixels(&app);

    // Pixel inside the wedge (upper-right of disc, ~45° from +x).
    let (ix, iy) = ndc_to_px(0.25, 0.25);
    let inside = read_pixel(&bytes, ix, iy);
    assert!(
        inside[1] > 200 && inside[0] < 30,
        "upper-right wedge interior should be green, got {inside:?}"
    );

    // Pixel inside the disc radius but OUTSIDE the wedge (~225°
    // direction, lower-left): should be background.
    let (lx, ly) = ndc_to_px(-0.25, -0.25);
    let lower_left = read_pixel(&bytes, lx, ly);
    assert!(
        lower_left[1] < 30,
        "lower-left (outside wedge) should be near-black, got {lower_left:?}"
    );
}

#[test]
fn donut_band_has_a_hole_in_the_middle() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = renderer(&app, &rt);

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(0, 0, 255, 255)));
    // Donut: ring from r=0.35 to r=0.55, full circle.
    g.draw_annular_sector(glam::Vec2::ZERO, 0.35, 0.55, 0.0, std::f32::consts::TAU);
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    let bytes = rt.read_pixels(&app);

    // Centre → in the hole, should be background.
    let (cx, cy) = ndc_to_px(0.0, 0.0);
    let centre = read_pixel(&bytes, cx, cy);
    assert!(
        centre[2] < 30,
        "donut centre (in hole) should be near-black, got {centre:?}"
    );

    // Mid-band (r ≈ 0.45) → in the ring, should be blue.
    let (bx, by) = ndc_to_px(0.45, 0.0);
    let band = read_pixel(&bytes, bx, by);
    assert!(
        band[2] > 200 && band[0] < 30,
        "donut mid-band should be opaque blue, got {band:?}"
    );

    // Outside outer radius → background.
    let (ox, oy) = ndc_to_px(0.8, 0.0);
    let outside = read_pixel(&bytes, ox, oy);
    assert!(
        outside[2] < 30,
        "outside donut outer radius should be near-black, got {outside:?}"
    );
}

#[test]
fn draw_arc_emits_thin_stroked_curve() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = renderer(&app, &rt);

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(255, 200, 0, 255)));
    // Thin stroked arc, radius 0.5, π/2-wide span centred on +x.
    g.draw_arc(
        glam::Vec2::ZERO,
        0.5,
        -std::f32::consts::FRAC_PI_4,
        std::f32::consts::FRAC_PI_4,
        0.05,
    );
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    let bytes = rt.read_pixels(&app);

    // On the arc centerline at angle = 0: (0.5, 0).
    let (ax, ay) = ndc_to_px(0.5, 0.0);
    let on_arc = read_pixel(&bytes, ax, ay);
    assert!(
        on_arc[0] > 200 && on_arc[1] > 150 && on_arc[2] < 50,
        "centerline of arc should be opaque amber, got {on_arc:?}"
    );

    // Just inside the band radius (r ≈ 0.35) → background.
    let (ix, iy) = ndc_to_px(0.35, 0.0);
    let inside = read_pixel(&bytes, ix, iy);
    assert!(
        inside[0] < 30 && inside[1] < 30,
        "inside band radius should be near-black, got {inside:?}"
    );

    // Discriminator pixel: well outside the angular span (top of
    // circle at angle π/2) → background.
    let (tx, ty) = ndc_to_px(0.0, 0.5);
    let top = read_pixel(&bytes, tx, ty);
    assert!(
        top[0] < 30 && top[1] < 30,
        "outside angular span (top of circle) should be near-black, got {top:?}"
    );
}
