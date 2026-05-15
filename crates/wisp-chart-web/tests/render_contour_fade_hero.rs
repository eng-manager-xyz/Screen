#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Hero snapshot for the `wisp-animation` book's
//! `Animatable trait + impls` chapter (M-ANIM.1 / AUT-228).
//!
//! Renders the contour fixture at alpha = 0.5 so the still-frame
//! fallback communicates "this chart is mid-fade". Uses the same
//! container.alpha field the live demo mutates.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "viewport bounded by W=360, H=360"
)]

use std::path::PathBuf;

use glam::Vec2;
use pollster::block_on;
use wisp::RenderTexture;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp_chart::Theme;
use wisp_chart_web::fixtures::contour_fixture;

const W: u32 = 360;
const H: u32 = 360;
const MID_FADE_ALPHA: f32 = 0.5;

#[test]
fn contour_fade_hero_renders_to_snapshot() {
    let mut app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("Application::new");
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");
    let theme = Theme::light();
    let viewport = Vec2::new(W as f32, H as f32);
    let chart = contour_fixture();
    let mut graphics = chart.emit_graphics(&theme, viewport);
    graphics.container.alpha = MID_FADE_ALPHA;
    let root = app.stage().root();
    let _ = app.stage_mut().add_child(root, graphics);

    app.device().push_error_scope(wgpu::ErrorFilter::Validation);
    let _stats = renderer.render_stage(&app, rt.view(), wisp::Color::WHITE, app.stage());
    let validation = block_on(app.device().pop_error_scope());
    assert!(
        validation.is_none(),
        "wgpu validation error: {validation:?}"
    );
    let bytes = rt.read_pixels(&app);
    let snapshot: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-animation-book/src/assets/wisp-animation/animatable-hero.png");
    if let Some(parent) = snapshot.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write animatable-hero.png");
}
