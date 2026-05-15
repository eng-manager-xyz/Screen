#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Hero for ColorSpace chapter (M-ANIM.13). Three ellipses
//! showing the midpoint colour of red→green in LinearRgb /
//! Oklab / Oklch. The differences are subtle but real.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "viewport bounded"
)]

use std::path::PathBuf;
use std::time::Duration;

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::scene::{Fill, Graphics};
use wisp::{Color, RenderTexture};
use wisp_animation::{Animation, ColorTween};

const W: u32 = 480;
const H: u32 = 160;

#[test]
fn color_spaces_hero_renders_to_snapshot() {
    let mut app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("Application::new");
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let red = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    let green = Color {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    let mid = Duration::from_millis(500);

    let lrgb = ColorTween::new(red, green, Duration::from_secs(1)).sample(mid);
    let oklab = ColorTween::new(red, green, Duration::from_secs(1))
        .in_oklab()
        .sample(mid);
    let oklch = ColorTween::new(red, green, Duration::from_secs(1))
        .in_oklch()
        .sample(mid);

    let root = app.stage().root();
    for (x, color) in [(-0.6_f32, lrgb), (0.0, oklab), (0.6, oklch)] {
        let mut g = Graphics::new();
        g.fill(Fill::Solid(color));
        g.draw_ellipse(Vec2::new(x, 0.0), Vec2::splat(0.22));
        let _ = app.stage_mut().add_child(root, g);
    }
    app.device().push_error_scope(wgpu::ErrorFilter::Validation);
    let _ = renderer.render_stage(&app, rt.view(), Color::WHITE, app.stage());
    let validation = block_on(app.device().pop_error_scope());
    assert!(validation.is_none());
    let bytes = rt.read_pixels(&app);
    let snapshot: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-animation-book/src/assets/wisp-animation/color-spaces-hero.png");
    std::fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write color-spaces-hero.png");
}
