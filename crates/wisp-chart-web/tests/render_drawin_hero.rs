//! Hero snapshot for PathMorph / DrawIn chapter (M-ANIM.10).
//! Polar chart at S-curve's halfway point, scaled to 0.45.

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
use wisp::RenderTexture;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::scene::Transform;
use wisp_chart::Theme;
use wisp_chart_web::fixtures::polar_plot_fixture;

const W: u32 = 360;
const H: u32 = 360;

#[test]
fn drawin_hero_renders_to_snapshot() {
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
    graphics.container.transform = Transform {
        position: Vec2::new(0.0, 0.0),
        scale: Vec2::splat(0.45),
        rotation: 0.0,
        pivot: Vec2::ZERO,
        skew: Vec2::ZERO,
    };
    let root = app.stage().root();
    let _ = app.stage_mut().add_child(root, graphics);
    app.device().push_error_scope(wgpu::ErrorFilter::Validation);
    let _ = renderer.render_stage(&app, rt.view(), wisp::Color::WHITE, app.stage());
    let validation = block_on(app.device().pop_error_scope());
    assert!(validation.is_none());
    let bytes = rt.read_pixels(&app);
    let snapshot: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-animation-book/src/assets/wisp-animation/drawin-hero.png");
    std::fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write drawin-hero.png");
}
