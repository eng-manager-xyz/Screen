//! M0.11 — `VideoTexture` + `RenderTexture` integration tests.
//!
//! `RenderTexture::read_pixels` enables real pixel-level assertions for the
//! first time. Subsequent chunks (M0.12+ Graphics, M0.16+ filters) use this
//! pattern as their primary verification path.

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, RenderTexture, Sprite, Stage, Texture, VideoTexture};

fn boot() -> Application {
    block_on(Application::new(AppConfig::default())).expect("init wisp")
}

#[test]
fn video_texture_dimensions_round_trip() {
    let app = boot();
    let vt = VideoTexture::new(&app, 320, 240);
    assert_eq!(vt.width(), 320);
    assert_eq!(vt.height(), 240);
}

#[test]
fn video_texture_upload_accepts_correct_size() {
    let app = boot();
    let vt = VideoTexture::new(&app, 16, 16);
    let bytes = vec![64u8; 16 * 16 * 4];
    // Should not panic; wgpu validation surfaces via device poll.
    vt.upload_bgra(&app, &bytes);
    app.device().poll(wgpu::Maintain::Wait);
}

#[test]
#[should_panic(expected = "byte length mismatch")]
fn video_texture_upload_rejects_wrong_size() {
    let app = boot();
    let vt = VideoTexture::new(&app, 16, 16);
    let too_small = vec![0u8; 100];
    vt.upload_bgra(&app, &too_small);
}

#[test]
fn render_texture_read_pixels_round_trips_clear_color() {
    let app = boot();
    // Linear (non-sRGB) format so the clear color round-trips byte-for-byte.
    let rt = RenderTexture::with_format(&app, 16, 16, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");
    let stage = Stage::new();

    let clear = Color::rgba_u8(255, 64, 32, 255);
    let _ = renderer.render_stage(&app, rt.view(), clear, &stage);

    let bytes = rt.read_pixels(&app);
    assert_eq!(bytes.len(), 16 * 16 * 4);

    // In a linear-format target, the clear color's u8 channels round-trip exactly.
    assert_eq!(bytes[0], 255);
    assert_eq!(bytes[1], 64);
    assert_eq!(bytes[2], 32);
    assert_eq!(bytes[3], 255);
}

#[test]
fn render_texture_captures_sprite() {
    let app = boot();
    let rt = RenderTexture::new(&app, 32, 32);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    // Solid green texture — 4×4 fully covers any sprite under transform.
    let mut bytes = Vec::with_capacity(4 * 4 * 4);
    for _ in 0..(4 * 4) {
        bytes.extend_from_slice(&[0, 255, 0, 255]);
    }
    let texture = Texture::from_rgba(&app, 4, 4, &bytes);

    let mut stage = Stage::new();
    let root = stage.root();
    // Sprite covering all of NDC: scale (2,2), centered (anchor 0.5 + position 0).
    let mut sprite = Sprite::from_texture(texture).with_anchor(glam::Vec2::splat(0.5));
    sprite.container.transform.scale = glam::Vec2::splat(2.0);
    let _ = stage.add_child(root, sprite);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);

    let pixels = rt.read_pixels(&app);
    // Center pixel of a 32×32 image — index (16, 16).
    let idx = (16 * 32 + 16) * 4;
    let r = pixels[idx];
    let g = pixels[idx + 1];
    let b = pixels[idx + 2];
    assert!(g > 200, "expected green sprite at center, got G={g}");
    assert!(r < 50, "center R should be ~0, got {r}");
    assert!(b < 50, "center B should be ~0, got {b}");
}
