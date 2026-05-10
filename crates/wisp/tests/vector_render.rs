//! AUT-54 — render vector primitives to scene geometry.
//!
//! `Vector::add_to_stage` consumes a vector primitive (analytic
//! shape + fill / stroke / opacity / transform) and produces a
//! `Graphics` node in the stage. The existing graphics rasterizer
//! draws it.
//!
//! Three contracts:
//! 1. Filled rect — center pixel = the fill color.
//! 2. Filled circle — center pixel = fill color, far-corner pixel =
//!    clear (carved by the ellipse rasterizer).
//! 3. Opacity multiplies fill alpha — half-opacity solid red over
//!    black gives ~50% red, not full red.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, RenderTexture, Stage, Vector, VectorShape};

const W: u32 = 64;
const H: u32 = 64;

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

fn render_to_rt(app: &Application, renderer: &Renderer, stage: &Stage) -> RenderTexture {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let rt = RenderTexture::with_format(app, W, H, format);
    let _ = renderer.render_stage(app, rt.view(), Color::BLACK, stage);
    rt
}

fn read_pixel(rt: &RenderTexture, app: &Application, x: u32, y: u32) -> [u8; 4] {
    let bytes = rt.read_pixels(app);
    let stride = (W as usize) * 4;
    let i = (y as usize) * stride + (x as usize) * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

#[test]
fn filled_rect_renders_with_fill_color() {
    let (app, renderer) = boot();
    let mut stage = Stage::new();
    let v = Vector::new(VectorShape::rect(Rect::new(-0.5, -0.5, 1.0, 1.0)))
        .with_fill(Fill::Solid(Color::rgba_u8(200, 80, 40, 255)));
    let root = stage.root();
    let _ = v.add_to_stage(&mut stage, root);

    let rt = render_to_rt(&app, &renderer, &stage);
    let center = read_pixel(&rt, &app, W / 2, H / 2);
    assert_eq!(center[0], 200, "center R should be fill color");
    assert_eq!(center[1], 80, "center G should be fill color");
    assert_eq!(center[2], 40, "center B should be fill color");
}

#[test]
fn filled_circle_renders_round_silhouette() {
    let (app, renderer) = boot();
    let mut stage = Stage::new();
    let v = Vector::new(VectorShape::circle(Vec2::ZERO, 0.4))
        .with_fill(Fill::Solid(Color::rgba_u8(80, 200, 120, 255)));
    let root = stage.root();
    let _ = v.add_to_stage(&mut stage, root);

    let rt = render_to_rt(&app, &renderer, &stage);

    // Center inside circle = fill.
    let center = read_pixel(&rt, &app, W / 2, H / 2);
    assert_eq!(center[1], 200, "center G should be fill color");

    // Far corner = outside circle = clear (black).
    let corner = read_pixel(&rt, &app, 2, 2);
    assert_eq!(corner[1], 0, "far corner should be clear (black)");
}

#[test]
fn opacity_multiplies_fill_alpha() {
    let (app, renderer) = boot();
    let mut stage = Stage::new();
    let v = Vector::new(VectorShape::rect(Rect::new(-0.5, -0.5, 1.0, 1.0)))
        .with_fill(Fill::Solid(Color::rgba_u8(255, 0, 0, 255)))
        .with_opacity(0.5);
    let root = stage.root();
    let _ = v.add_to_stage(&mut stage, root);

    let rt = render_to_rt(&app, &renderer, &stage);
    // Solid red at alpha 0.5 over black with alpha-blending:
    // out.r = 255 * 0.5 + 0 * 0.5 = 127.5 → 127 or 128 (premul vs not).
    // Be lenient — verify red is mid-range, not full.
    let center = read_pixel(&rt, &app, W / 2, H / 2);
    assert!(
        (60..=180).contains(&center[0]),
        "half-opacity red should be mid-range, got R={}",
        center[0]
    );
}

#[test]
fn path_to_graphics_returns_none() {
    let v = Vector::new(VectorShape::path(vec![Vec2::ZERO, Vec2::new(0.5, 0.5)]));
    assert!(
        v.to_graphics().is_none(),
        "path → graphics should return None until M-VEC.10"
    );
}
