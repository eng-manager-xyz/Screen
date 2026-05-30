//! AUT-225 / M-VEC.21 — `Graphics::draw_polygon` (convex, fan-triangulated)
//! rendering tests.
//!
//! Validates the three shapes a chart consumer needs from a convex polygon
//! primitive:
//! * **square** — minimal 4-vertex polygon, sanity check for the triangle
//!   pipeline.
//! * **regular pentagon** — non-rect convex shape; centre fills, exterior
//!   is background.
//! * **trapezoid** — funnel-area-style asymmetric quad.

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

fn read_pixel(bytes: &[u8], x: u32, y: u32) -> [u8; 4] {
    let stride = (W * 4) as usize;
    let i = (y as usize) * stride + (x as usize) * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

fn ndc_to_px(x: f32, y: f32) -> (u32, u32) {
    let px = (f32::midpoint(x, 1.0) * W as f32).clamp(0.0, (W - 1) as f32);
    let py = ((1.0 - y) * 0.5 * H as f32).clamp(0.0, (H - 1) as f32);
    (px as u32, py as u32)
}

#[test]
fn square_polygon_fills_interior() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(255, 0, 0, 255)));
    // CCW square, NDC [-0.3, +0.3] in both axes.
    g.draw_polygon(&[
        glam::Vec2::new(-0.3, -0.3),
        glam::Vec2::new(0.3, -0.3),
        glam::Vec2::new(0.3, 0.3),
        glam::Vec2::new(-0.3, 0.3),
    ]);
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    let bytes = rt.read_pixels(&app);

    let (cx, cy) = ndc_to_px(0.0, 0.0);
    let centre = read_pixel(&bytes, cx, cy);
    assert_eq!(
        centre,
        [255, 0, 0, 255],
        "centre of square should be opaque red, got {centre:?}"
    );

    let (ox, oy) = ndc_to_px(0.7, 0.7);
    let outside = read_pixel(&bytes, ox, oy);
    assert!(
        outside[0] < 10,
        "exterior should be black background, got {outside:?}"
    );
}

#[test]
fn pentagon_polygon_fills_interior() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(0, 200, 100, 255)));
    // Regular pentagon centred at origin, CCW from top.
    let r = 0.5_f32;
    let mut vertices = Vec::with_capacity(5);
    for i in 0..5 {
        let theta = std::f32::consts::FRAC_PI_2 + (i as f32) * std::f32::consts::TAU / 5.0;
        vertices.push(glam::Vec2::new(r * theta.cos(), r * theta.sin()));
    }
    g.draw_polygon(&vertices);
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    let bytes = rt.read_pixels(&app);

    let (cx, cy) = ndc_to_px(0.0, 0.0);
    let centre = read_pixel(&bytes, cx, cy);
    assert_eq!(
        centre,
        [0, 200, 100, 255],
        "pentagon centre should be opaque green, got {centre:?}"
    );

    // Far outside any pentagon vertex → background.
    let (ox, oy) = ndc_to_px(0.85, 0.85);
    let outside = read_pixel(&bytes, ox, oy);
    assert!(
        outside[1] < 10,
        "pentagon exterior should be black, got {outside:?}"
    );
}

#[test]
fn trapezoid_polygon_renders_funnel_shape() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(120, 100, 220, 255)));
    // Inverted trapezoid — funnel-area style: wide at top, narrow at
    // bottom. CCW order starting from bottom-left.
    g.draw_polygon(&[
        glam::Vec2::new(-0.2, -0.5),
        glam::Vec2::new(0.2, -0.5),
        glam::Vec2::new(0.6, 0.5),
        glam::Vec2::new(-0.6, 0.5),
    ]);
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    let bytes = rt.read_pixels(&app);

    // Centre should be inside.
    let (cx, cy) = ndc_to_px(0.0, 0.0);
    let centre = read_pixel(&bytes, cx, cy);
    assert_eq!(
        centre,
        [120, 100, 220, 255],
        "trapezoid centre should be opaque purple, got {centre:?}"
    );

    // Lower-left corner pixel (outside the narrow bottom edge).
    let (lx, ly) = ndc_to_px(-0.4, -0.4);
    let lower_left = read_pixel(&bytes, lx, ly);
    assert!(
        lower_left[2] < 30,
        "lower-left outside narrow bottom should be black, got {lower_left:?}"
    );
}
