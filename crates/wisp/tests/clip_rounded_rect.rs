//! Pixel-readback tests for the M-MASK.1 rounded-rect clip foundation
//! ([`Container::clip = MaskShape::RoundedRect { … }`](wisp::Container)).
//!
//! Strategy: render a solid graphic over a black clear, with a clip
//! mask applied. Assert that center-pixel = graphic color (mask alpha
//! = 1) and far-corner pixel = black clear (mask alpha = 0). The
//! anti-alias band between is exercised at a tolerance check on a
//! pixel just inside vs. just outside the rounded edge.

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Container, Fill, Graphics, MaskShape, Node, RenderTexture, Stage};

const W: u32 = 128;
const H: u32 = 128;

fn boot() -> (Application, Renderer, RenderTexture) {
    let app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("app");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("renderer");
    let target = RenderTexture::with_format(&app, W, H, format);
    (app, renderer, target)
}

fn read_pixel(rt: &RenderTexture, app: &Application, x: u32, y: u32) -> [u8; 4] {
    let bytes = rt.read_pixels(app);
    let stride = (W as usize) * 4;
    let i = (y as usize) * stride + (x as usize) * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

fn assert_close(actual: [u8; 4], expected: [u8; 4], tol: i32, ctx: &str) {
    for ch in 0..4 {
        let diff = i32::from(actual[ch]) - i32::from(expected[ch]);
        assert!(
            diff.abs() <= tol,
            "{ctx} ch{ch}: got {actual:?}, expected ~{expected:?} (diff {diff})"
        );
    }
}

/// Build a stage with a single full-NDC red graphics inside a clipped
/// container. The clip is `rect` with `radius` corner radius.
fn build_clipped_stage(rect: Rect, radius: f32) -> Stage {
    let mut stage = Stage::new();
    let mut clip_container = Container::new();
    clip_container.clip = Some(MaskShape::RoundedRect { rect, radius });
    let clip_node = stage.add_child(stage.root(), Node::Container(clip_container));

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
    g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(clip_node.expect("clip node id"), g);
    stage
}

#[test]
fn center_pixel_is_inside_the_clip() {
    let (app, renderer, target) = boot();
    let stage = build_clipped_stage(Rect::new(-0.5, -0.5, 1.0, 1.0), 0.1);
    let _ = renderer.render_stage(&app, target.view(), Color::BLACK, &stage);

    // Center of the RT is inside the clip rect → red, fully opaque.
    let center = read_pixel(&target, &app, W / 2, H / 2);
    assert_close(center, [255, 0, 0, 255], 2, "clip center");
}

#[test]
fn far_corner_is_outside_the_clip() {
    let (app, renderer, target) = boot();
    let stage = build_clipped_stage(Rect::new(-0.5, -0.5, 1.0, 1.0), 0.1);
    let _ = renderer.render_stage(&app, target.view(), Color::BLACK, &stage);

    // Top-left pixel is far outside the clip rect → black clear shows.
    let corner = read_pixel(&target, &app, 2, 2);
    assert_close(corner, [0, 0, 0, 255], 2, "far corner");
}

#[test]
fn pixel_inside_rect_but_outside_corner_radius_is_clipped() {
    // Clip rect [-0.5, +0.5]² with radius 0.4 — the rounded shape
    // strictly excludes a disc-corner of the bounding rect.
    // NDC (0.45, 0.45) is inside the bounding rect but outside the
    // rounded shape (the corner is cut at radius 0.4 from the inset
    // corner anchor at (0.1, 0.1)).
    let (app, renderer, target) = boot();
    let stage = build_clipped_stage(Rect::new(-0.5, -0.5, 1.0, 1.0), 0.4);
    let _ = renderer.render_stage(&app, target.view(), Color::BLACK, &stage);

    // NDC (0.45, 0.45) — convert to pixel: (0.45 + 1)/2 * W = 0.725*W.
    // W is small (128) so the cast is exact in f32.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "constant-expr pixel coord, W is bounded at 128 well below 2^23"
    )]
    let px = (0.725_f32 * W as f32) as u32;
    let corner_inside_rect = read_pixel(&target, &app, px, px);
    assert_close(corner_inside_rect, [0, 0, 0, 255], 6, "rounded corner");
}

#[test]
fn no_clip_renders_normally_via_fast_path() {
    // A scene with NO clipped containers should hit the fast path —
    // pixel readback identical to pre-M-MASK.1 behavior.
    let (app, renderer, target) = boot();
    let mut stage = Stage::new();
    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba(0.0, 1.0, 0.0, 1.0)));
    g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, target.view(), Color::BLACK, &stage);
    let center = read_pixel(&target, &app, W / 2, H / 2);
    assert_close(center, [0, 255, 0, 255], 2, "fast-path no-clip");
}
