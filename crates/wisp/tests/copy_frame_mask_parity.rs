//! AUT-33 — copy-current-frame respects masks.
//!
//! `RenderTexture::read_pixels` is the renderer-level surface that
//! the future copy-frame button will sit on top of. These tests
//! lock in: the bytes you read back from a masked render *are* the
//! masked bytes — there's no separate "preview-only" mask path that
//! could be bypassed when a frame is copied.
//!
//! Strategy: render a known-content base + mask primitive, read
//! pixels, assert that pixels INSIDE the mask region show the
//! masked content (blur / redaction / spotlight effect) and pixels
//! OUTSIDE the region show the base unchanged.

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, MaskShape, RenderTexture, Stage};

const W: u32 = 96;
const H: u32 = 96;

fn skip_on_software_adapter() -> bool {
    if std::env::var_os("WISP_SKIP_GPU_FILTER_TESTS").is_some() {
        eprintln!(
            "WISP_SKIP_GPU_FILTER_TESTS set — skipping copy-frame parity test \
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

#[test]
fn copy_frame_after_redaction_shows_redacted_pixels() {
    let (app, renderer) = boot();
    let base = red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let shape = MaskShape::rect(Rect::new(-0.4, -0.4, 0.8, 0.8));
    let redact = Color::rgba_u8(40, 40, 200, 255);
    renderer.apply_solid_redaction(&app, shape, redact, &base, &output);

    // The "copy frame" path is just read_pixels.
    let inside = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(inside[0], 40, "copy: redacted R");
    assert_eq!(inside[2], 200, "copy: redacted B");

    let outside = read_pixel(&output, &app, 4, 4);
    assert_eq!(outside[0], 255, "copy: base red preserved outside");
}

#[test]
fn copy_frame_after_spotlight_shows_dimmed_outside() {
    let (app, renderer) = boot();
    let base = red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let output = RenderTexture::with_format(&app, W, H, format);
    let shape = MaskShape::rounded_rect(Rect::new(-0.4, -0.4, 0.8, 0.8), 0.1);
    renderer.apply_spotlight(&app, shape, Color::rgba(0.0, 0.0, 0.0, 0.7), &base, &output);

    let inside = read_pixel(&output, &app, W / 2, H / 2);
    assert_eq!(inside[0], 255, "copy: inside-spotlight base preserved");

    let outside = read_pixel(&output, &app, 4, 4);
    assert!(
        outside[0] < 200,
        "copy: outside-spotlight darkened, got R={}",
        outside[0]
    );
}

#[test]
fn copy_frame_after_privacy_blur_shows_blurred_pixels() {
    if skip_on_software_adapter() {
        return;
    }
    let (app, renderer) = boot();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    // Red/blue base, blur the seam region.
    let base = RenderTexture::with_format(&app, W, H, format);
    {
        let mut stage = Stage::new();
        let mut left = Graphics::new();
        left.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
        left.draw_rect(Rect::new(-1.0, -1.0, 1.0, 2.0));
        let _ = stage.add_child(stage.root(), left);
        let mut right = Graphics::new();
        right.fill(Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)));
        right.draw_rect(Rect::new(0.0, -1.0, 1.0, 2.0));
        let _ = stage.add_child(stage.root(), right);
        let _ = renderer.render_stage(&app, base.view(), Color::BLACK, &stage);
    }

    let output = RenderTexture::with_format(&app, W, H, format);
    let shape = MaskShape::rect(Rect::new(-0.5, -0.5, 1.0, 1.0));
    renderer.apply_privacy_blur(&app, shape, 12.0, &base, &output);

    // Center on the seam, inside the privacy region — both colors
    // mix via the blur kernel after the copy.
    let center = read_pixel(&output, &app, W / 2, H / 2);
    assert!(
        center[0] > 20 && center[2] > 20,
        "copy: blur kernel mixed both colors, got {center:?}"
    );

    // Outside region — base preserved.
    let outside = read_pixel(&output, &app, 2, 2);
    assert_eq!(outside[0], 255, "copy: outside-mask base preserved");
}
