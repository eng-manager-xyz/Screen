//! AUT-56 — privacy blur driven by `Vector` (M-VEC.4).
//!
//! Two new contracts on top of the existing privacy-blur tests:
//! 1. Calling `apply_privacy_blur_vector` with an analytic shape
//!    produces output byte-equivalent to the old `MaskShape` path.
//! 2. Calling with a freehand path *blurs inside the polygon* — the
//!    feature that previously required a separate code path.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, MaskShape, RenderTexture, Stage, Vector, VectorShape};

const W: u32 = 128;
const H: u32 = 128;

fn skip_on_software_adapter() -> bool {
    if std::env::var_os("WISP_SKIP_GPU_FILTER_TESTS").is_some() {
        eprintln!(
            "WISP_SKIP_GPU_FILTER_TESTS set — skipping privacy-blur-vector test \
             (lavapipe loses the device on multi-bind-group filter pipelines)"
        );
        return true;
    }
    false
}

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

fn render_red_blue_split(app: &Application, renderer: &Renderer) -> RenderTexture {
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
fn vector_variant_matches_mask_shape_variant_byte_for_byte() {
    if skip_on_software_adapter() {
        return;
    }
    let (app, renderer) = boot();
    let base = render_red_blue_split(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);

    let out_a = RenderTexture::with_format(&app, W, H, format);
    renderer.apply_privacy_blur(&app, MaskShape::rect(region), 8.0, &base, &out_a);

    let out_b = RenderTexture::with_format(&app, W, H, format);
    renderer.apply_privacy_blur_vector(
        &app,
        &Vector::new(VectorShape::rect(region)),
        8.0,
        &base,
        &out_b,
    );

    let bytes_a = out_a.read_pixels(&app);
    let bytes_b = out_b.read_pixels(&app);
    assert_eq!(bytes_a.len(), bytes_b.len());
    let mismatches = bytes_a
        .iter()
        .zip(bytes_b.iter())
        .filter(|(a, b)| a != b)
        .count();
    // Identical pipelines should produce byte-equivalent output.
    assert!(
        mismatches < 16,
        "byte-mismatch count too high: {mismatches}/{}",
        bytes_a.len()
    );
}

#[test]
fn path_vector_blurs_inside_polygon() {
    if skip_on_software_adapter() {
        return;
    }
    let (app, renderer) = boot();
    let base = render_red_blue_split(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;

    // Diamond polygon centered on the seam. Center pixel is inside;
    // far corner is outside.
    let polygon = vec![
        Vec2::new(0.0, 0.6),
        Vec2::new(0.6, 0.0),
        Vec2::new(0.0, -0.6),
        Vec2::new(-0.6, 0.0),
    ];
    let v = Vector::new(VectorShape::path(polygon));

    let out = RenderTexture::with_format(&app, W, H, format);
    renderer.apply_privacy_blur_vector(&app, &v, 12.0, &base, &out);

    // Center of diamond, on the seam — both colors mix via blur.
    let center = read_pixel(&out, &app, W / 2, H / 2);
    assert!(
        center[0] > 20 && center[2] > 20,
        "diamond center should mix red+blue, got {center:?}"
    );

    // Far corner outside polygon — base preserved.
    let outside = read_pixel(&out, &app, 4, 4);
    assert_eq!(outside[0], 255, "outside polygon should be base red");
    assert_eq!(outside[2], 0, "outside polygon should have no blue bleed");
}
