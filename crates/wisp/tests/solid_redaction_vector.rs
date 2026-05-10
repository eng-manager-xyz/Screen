//! AUT-57 — solid redaction driven by `Vector` (M-VEC.5).

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, MaskShape, RenderTexture, Stage, Vector, VectorShape};

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
fn redaction_vector_route_matches_mask_shape_route() {
    let (app, renderer) = boot();
    let base = render_red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    let color = Color::rgba_u8(20, 200, 60, 255);

    let out_a = RenderTexture::with_format(&app, W, H, format);
    renderer.apply_solid_redaction(&app, MaskShape::rect(region), color, &base, &out_a);

    let out_b = RenderTexture::with_format(&app, W, H, format);
    renderer.apply_solid_redaction_vector(
        &app,
        &Vector::new(VectorShape::rect(region)),
        color,
        &base,
        &out_b,
    );

    // Sample a few pixels — both routes should match closely.
    for (x, y) in [(W / 2, H / 2), (4, 4), (32, 32)] {
        let a = read_pixel(&out_a, &app, x, y);
        let b = read_pixel(&out_b, &app, x, y);
        for ch in 0..4 {
            assert!(
                a[ch].abs_diff(b[ch]) <= 1,
                "channel {ch} differs at ({x}, {y}): {a:?} vs {b:?}"
            );
        }
    }
}

#[test]
fn redaction_path_vector_fills_polygon_only() {
    let (app, renderer) = boot();
    let base = render_red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;

    // Diamond polygon.
    let polygon = vec![
        Vec2::new(0.0, 0.6),
        Vec2::new(0.6, 0.0),
        Vec2::new(0.0, -0.6),
        Vec2::new(-0.6, 0.0),
    ];
    let v = Vector::new(VectorShape::path(polygon));
    let color = Color::rgba_u8(10, 30, 200, 255);

    let out = RenderTexture::with_format(&app, W, H, format);
    renderer.apply_solid_redaction_vector(&app, &v, color, &base, &out);

    // Center inside diamond — exact redaction color.
    let inside = read_pixel(&out, &app, W / 2, H / 2);
    assert_eq!(inside[0], 10, "center R = redaction R");
    assert_eq!(inside[2], 200, "center B = redaction B");

    // Far corner outside diamond — base red.
    let outside = read_pixel(&out, &app, 4, 4);
    assert_eq!(outside[0], 255, "outside polygon = base red");
    assert_eq!(outside[2], 0, "outside polygon = no blue");
}
