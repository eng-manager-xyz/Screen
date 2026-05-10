//! AUT-59 / M-VEC.7 — vector-driven dim-outside accepting paths.
//!
//! `apply_dim_outside_vector(vector, strength, base, output)` is
//! the path-accepting companion to `apply_dim_outside_data`. The
//! `DimStrength` enum still controls outside opacity; the only
//! difference is the shape source.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{
    Color, DimOutside, DimStrength, Fill, Graphics, RenderTexture, Stage, Vector, VectorShape,
};

const W: u32 = 96;
const H: u32 = 96;

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

fn red_base(app: &Application, renderer: &Renderer) -> RenderTexture {
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

fn read_pixel(rt: &RenderTexture, app: &Application, x: u32, y: u32) -> [u8; 4] {
    let bytes = rt.read_pixels(app);
    let stride = (W as usize) * 4;
    let i = (y as usize) * stride + (x as usize) * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

#[test]
fn vector_dim_outside_matches_data_route_for_analytic_shape() {
    let (app, renderer) = boot();
    let base = red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let region = Rect::new(-0.4, -0.4, 0.8, 0.8);

    let out_data = RenderTexture::with_format(&app, W, H, format);
    let dim = DimOutside::rect(region).with_strength(DimStrength::Heavy);
    renderer.apply_dim_outside_data(&app, &dim, &base, &out_data);

    let out_vector = RenderTexture::with_format(&app, W, H, format);
    renderer.apply_dim_outside_vector(
        &app,
        &Vector::new(VectorShape::rect(region)),
        DimStrength::Heavy,
        &base,
        &out_vector,
    );

    assert_eq!(
        out_data.read_pixels(&app),
        out_vector.read_pixels(&app),
        "vector dim_outside must match data dim_outside for analytic shape"
    );
}

#[test]
fn vector_dim_outside_path_dims_around_polygon() {
    let (app, renderer) = boot();
    let base = red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);

    let diamond = Vector::new(VectorShape::path(vec![
        Vec2::new(0.0, 0.6),
        Vec2::new(0.6, 0.0),
        Vec2::new(0.0, -0.6),
        Vec2::new(-0.6, 0.0),
    ]));
    renderer.apply_dim_outside_vector(&app, &diamond, DimStrength::Medium, &base, &output);

    // Center inside diamond — base preserved.
    let inside = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(inside[0], 255, "inside diamond should be base red");

    // Corner outside diamond — dimmed by Medium (alpha 0.7).
    let outside = read_pixel(&output, &app, 4, 4);
    assert!(
        outside[0] < 180,
        "outside polygon should be dimmed (Medium ≈ 0.7 alpha black over red), got R={}",
        outside[0]
    );
}
