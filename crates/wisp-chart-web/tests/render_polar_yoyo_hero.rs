//! Hero snapshot for the Repeat/Reverse/Yoyo chapter
//! (M-ANIM.4 / AUT-231). Polar at scale 0.8 — midway in the
//! 0.6 → 1.0 yoyo cycle.

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
use wisp::scene::Transform;
use wisp_chart::Theme;
use wisp_chart_web::fixtures::polar_plot_fixture;

const W: u32 = 360;
const H: u32 = 360;
const MID_YOYO_SCALE: f32 = 0.8;

#[test]
fn polar_yoyo_hero_renders_to_snapshot() {
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
    let plot = polar_plot_fixture();
    let mut graphics = plot.emit_graphics(&theme, viewport);
    graphics.container.transform = Transform::from_scale(Vec2::splat(MID_YOYO_SCALE));
    let root = app.stage().root();
    let _ = app.stage_mut().add_child(root, graphics);
    app.device().push_error_scope(wgpu::ErrorFilter::Validation);
    let _ = renderer.render_stage(&app, rt.view(), wisp::Color::WHITE, app.stage());
    let validation = block_on(app.device().pop_error_scope());
    assert!(validation.is_none());
    let bytes = rt.read_pixels(&app);
    let snapshot: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-animation-book/src/assets/wisp-animation/repeat-hero.png");
    std::fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write repeat-hero.png");
}
