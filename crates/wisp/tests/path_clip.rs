//! AUT-35 — freehand path mask. Validates `apply_path_clip` and
//! `apply_solid_redaction_path` against a non-rect, non-elliptical
//! polygon (a five-pointed star — the canonical "shape that no SDF
//! can express").
//!
//! Three contracts:
//! 1. A pixel inside the star polygon passes the foreground through.
//! 2. A pixel inside the polygon's bounding rect but in one of the
//!    "concave gaps" between the points reads as the clear color
//!    (proves the winding-number test handles concave shapes).
//! 3. The path mask works inside `apply_solid_redaction_path` — same
//!    composition as the SDF redaction primitive.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, RenderTexture, Stage};

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

/// Five-pointed star polygon centered at origin. Outer radius 0.7,
/// inner radius 0.3. Concave — proves the winding-number test handles
/// non-convex shapes.
fn star_polygon() -> Vec<Vec2> {
    let mut pts = Vec::with_capacity(10);
    for i in 0i16..10 {
        let r = if i % 2 == 0 { 0.7 } else { 0.3 };
        // Step around the circle, offset by -π/2 so a point lands at
        // the top.
        let theta = f32::from(i) * std::f32::consts::PI / 5.0 - std::f32::consts::FRAC_PI_2;
        pts.push(Vec2::new(theta.cos() * r, theta.sin() * r));
    }
    pts
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
fn star_center_passes_through() {
    let (app, renderer) = boot();
    let fg = render_white_foreground(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let star = star_polygon();
    renderer.apply_path_clip(&app, &star, &fg, &output);

    // Center of the star — inside.
    let center = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(center[0], 255, "star center should pass white through");
    assert_eq!(center[3], 255, "star center should be opaque");
}

#[test]
fn star_concave_gap_is_carved_away() {
    let (app, renderer) = boot();
    let fg = render_white_foreground(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let star = star_polygon();
    renderer.apply_path_clip(&app, &star, &fg, &output);

    // (NDC -0.65, +0.55) is inside the bounding box [-0.7, +0.7]² but
    // outside the star — it's in the upper-left "between two arms"
    // gap. Distance from origin = sqrt(0.4225 + 0.3025) = 0.85, which
    // is beyond outer radius 0.7, so well outside.
    //
    // NDC (-0.65, +0.55) → x=((-0.65+1)/2)*128 = 22, y=((1-0.55)/2)*128 ≈ 28.
    let gap = read_pixel(&output, &app, 22, 28);
    assert_eq!(
        gap[3], 0,
        "concave-gap pixel must be alpha 0 (point-in-polygon false), got {gap:?}"
    );
}

#[test]
fn path_redaction_center_is_color() {
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
    let star = star_polygon();
    let redact = Color::rgba_u8(60, 0, 200, 255);
    renderer.apply_solid_redaction_path(&app, &star, redact, &base, &output);

    // Center of star → exactly the redaction color.
    let inside = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(inside[0], 60, "center R should be redaction color");
    assert_eq!(inside[1], 0, "center G should be 0");
    assert_eq!(inside[2], 200, "center B should be redaction color");

    // Top-left corner outside everything → base red.
    let outside = read_pixel(&output, &app, 4, 4);
    assert_eq!(outside[0], 255, "outside polygon should be base red");
}
