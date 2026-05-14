//! Treemap snapshot.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "viewport bounded by W=480, H=300"
)]

use std::path::PathBuf;

use glam::Vec2;
use pollster::block_on;
use wisp::RenderTexture;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp_chart::Theme;
use wisp_chart_web::fixtures::treemap_fixture;

const W: u32 = 480;
const H: u32 = 300;

#[test]
fn treemap_renders_to_snapshot() {
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
    let t = treemap_fixture();
    let graphics = t.emit_graphics(&theme, viewport);
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
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/treemap.png");
    if let Some(parent) = snapshot.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write treemap.png");
}
