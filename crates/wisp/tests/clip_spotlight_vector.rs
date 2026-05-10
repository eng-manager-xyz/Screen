//! AUT-58 — clip + spotlight driven by `Vector` (M-VEC.6).

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, RenderTexture, Stage, Vector, VectorShape};

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
fn clip_path_vector_passes_inside_polygon() {
    let (app, renderer) = boot();
    let fg = render_white_foreground(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);

    let diamond = Vector::new(VectorShape::path(vec![
        Vec2::new(0.0, 0.6),
        Vec2::new(0.6, 0.0),
        Vec2::new(0.0, -0.6),
        Vec2::new(-0.6, 0.0),
    ]));
    renderer.apply_clip_vector(&app, &diamond, &fg, &output);

    let center = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(center[0], 255, "center R = white pass-through");
    assert_eq!(center[3], 255, "center alpha = opaque");

    let outside = read_pixel(&output, &app, 4, 4);
    assert_eq!(outside[3], 0, "outside polygon = alpha 0");
}

#[test]
fn spotlight_path_vector_dims_outside_polygon() {
    let (app, renderer) = boot();
    let base = render_red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);

    let diamond = Vector::new(VectorShape::path(vec![
        Vec2::new(0.0, 0.6),
        Vec2::new(0.6, 0.0),
        Vec2::new(0.0, -0.6),
        Vec2::new(-0.6, 0.0),
    ]));
    renderer.apply_spotlight_vector(
        &app,
        &diamond,
        Color::rgba(0.0, 0.0, 0.0, 0.7),
        &base,
        &output,
    );

    // Center inside diamond — base red preserved.
    let inside = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(inside[0], 255, "inside diamond should be base red");

    // Far corner outside polygon — darkened.
    let outside = read_pixel(&output, &app, 4, 4);
    assert!(
        outside[0] < 200,
        "outside polygon should be dimmed, got R={}",
        outside[0]
    );
}
