//! Hero snapshot for the Stagger chapter (M-ANIM.8).
//! Five-dot row with center-out alpha gradient frozen at a wave
//! moment — dot 2 (centre) is brightest, dots 1 + 3 dimmer, dots
//! 0 + 4 dimmest.

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
use wisp_chart::Theme;

const W: u32 = 360;
const H: u32 = 120;

#[test]
fn stagger_hero_renders_to_snapshot() {
    let mut app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("Application::new");
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");
    let _theme = Theme::light();
    let count = 5_usize;
    // Alpha pattern at a mid-wave moment — centre brightest.
    let alphas = [0.25_f32, 0.55, 1.0, 0.55, 0.25];
    let root = app.stage().root();
    for i in 0..count {
        let mut g = Graphics::new();
        g.fill(Fill::Solid(Color {
            r: 0.0,
            g: 0.45,
            b: 0.7,
            a: 1.0,
        }));
        let centre_x = ((i as f32 / (count as f32 - 1.0)) - 0.5) * 1.2;
        g.draw_ellipse(Vec2::new(centre_x, 0.0), Vec2::splat(0.18));
        g.container.alpha = alphas[i];
        let _ = app.stage_mut().add_child(root, g);
    }
    app.device().push_error_scope(wgpu::ErrorFilter::Validation);
    let _ = renderer.render_stage(&app, rt.view(), Color::WHITE, app.stage());
    let validation = block_on(app.device().pop_error_scope());
    assert!(validation.is_none());
    let bytes = rt.read_pixels(&app);
    let snapshot: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-animation-book/src/assets/wisp-animation/stagger-hero.png");
    std::fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write stagger-hero.png");
}
