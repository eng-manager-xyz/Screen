//! AUT-22 — privacy blur strength is configurable. Two contracts:
//!
//! 1. `BlurStrength::Soft.radius_px() < Medium < Strong` — symbolic
//!    presets are monotonic in the underlying numeric radius.
//! 2. End-to-end: rendering the same scene with a *higher* strength
//!    produces a *more-blurred* result. We sample a pixel near a
//!    sharp red/blue seam — at higher strengths the blur kernel
//!    pulls in more of the opposite color, so the lower of (R, B) at
//!    that pixel goes UP as strength increases.
//!
//! Custom(f32) is also clamped (sanity check).

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{BlurStrength, Color, Fill, Graphics, PrivacyBlur, RenderTexture, Stage};

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

fn render_base(app: &Application, renderer: &Renderer) -> RenderTexture {
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

fn min_color_evidence(rt: &RenderTexture, app: &Application) -> u8 {
    let bytes = rt.read_pixels(app);
    let stride = (W as usize) * 4;
    // Sample on the red side, far from the seam (NDC -0.3 ≈ 38 px from
    // seam in a 128-wide image). Soft blur (radius 6) can't reach this
    // far; Medium (12) reaches a touch; Strong (24) pulls in real blue.
    // So B should grow strictly across the three strengths.
    let x = (W as usize * 35) / 100;
    let y = (H as usize) / 2;
    let i = y * stride + x * 4;
    bytes[i + 2] // B channel — the "other" color's bleed-in.
}

#[test]
fn strength_presets_are_monotonic() {
    assert!(BlurStrength::Soft.radius_px() < BlurStrength::Medium.radius_px());
    assert!(BlurStrength::Medium.radius_px() < BlurStrength::Strong.radius_px());
}

#[test]
fn custom_strength_is_clamped() {
    assert!(
        BlurStrength::Custom(-5.0).radius_px().abs() < 1e-6,
        "negative radius clamps to 0"
    );
    assert!(
        (BlurStrength::Custom(1000.0).radius_px() - 64.0).abs() < 1e-6,
        "huge radius clamps to 64"
    );
    assert!(
        (BlurStrength::Custom(15.5).radius_px() - 15.5).abs() < 1e-6,
        "in-range custom passes through unchanged"
    );
}

#[test]
fn higher_strength_produces_more_blur_evidence() {
    let (app, renderer) = boot();
    let base = render_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);

    let make = |strength: BlurStrength| -> u8 {
        let rt = RenderTexture::with_format(&app, W, H, format);
        let blur = PrivacyBlur::rect(region).with_strength(strength);
        renderer.apply_privacy_blur_data(&app, &blur, &base, &rt);
        min_color_evidence(&rt, &app)
    };

    let soft = make(BlurStrength::Soft);
    let medium = make(BlurStrength::Medium);
    let strong = make(BlurStrength::Strong);

    assert!(
        soft < medium,
        "Medium should pull in more blue than Soft; soft={soft}, medium={medium}"
    );
    assert!(
        medium < strong,
        "Strong should pull in more blue than Medium; medium={medium}, strong={strong}"
    );
}
