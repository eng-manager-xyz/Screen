#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Hero for FLIP chapter (M-ANIM.14). Two ellipses mid-swap —
//! one is halfway from left → right, the other halfway from
//! right → left. Captures what a FLIP transition looks like at
//! the midpoint of the interpolation.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "viewport bounded"
)]

use std::path::PathBuf;

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::scene::{Fill, Graphics};
use wisp::{Color, RenderTexture};

const W: u32 = 480;
const H: u32 = 160;

#[test]
fn flip_hero_renders_to_snapshot() {
    let mut app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("Application::new");
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");
    let blue = Color {
        r: 0.0,
        g: 0.45,
        b: 0.7,
        a: 1.0,
    };
    let red = Color {
        r: 0.85,
        g: 0.15,
        b: 0.15,
        a: 1.0,
    };
    let root = app.stage().root();
    // Midpoint of left-right swap: both ellipses sit on top of
    // each other at x = 0.
    for color in [blue, red] {
        let mut g = Graphics::new();
        g.fill(Fill::Solid(color));
        g.container.alpha = 0.7;
        g.draw_ellipse(Vec2::new(0.0, 0.0), Vec2::splat(0.22));
        let _ = app.stage_mut().add_child(root, g);
    }
    // Ghost markers at the two anchor positions.
    let ghost = Color {
        r: 0.7,
        g: 0.7,
        b: 0.7,
        a: 0.4,
    };
    for x in [-0.5_f32, 0.5] {
        let mut g = Graphics::new();
        g.fill(Fill::Solid(ghost));
        g.draw_ellipse(Vec2::new(x, 0.0), Vec2::splat(0.22));
        let _ = app.stage_mut().add_child(root, g);
    }
    app.device().push_error_scope(wgpu::ErrorFilter::Validation);
    let _ = renderer.render_stage(&app, rt.view(), Color::WHITE, app.stage());
    let validation = block_on(app.device().pop_error_scope());
    assert!(validation.is_none());
    let bytes = rt.read_pixels(&app);
    let snapshot: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-animation-book/src/assets/wisp-animation/flip-hero.png");
    std::fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write flip-hero.png");
}
