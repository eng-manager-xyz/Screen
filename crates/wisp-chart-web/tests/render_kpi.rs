#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! KPI-card snapshot — big number + label + delta + sparkline.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "viewport bounded by W=320, H=200"
)]

use std::path::PathBuf;

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Font, RenderTexture};
use wisp_chart::Theme;
use wisp_chart::indicator::{Delta, DeltaKind, Kpi};

const W: u32 = 320;
const H: u32 = 200;

#[test]
fn kpi_renders_to_snapshot() {
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

    let kpi = Kpi {
        value: 1_234_567.0,
        label: "Monthly Active Users".into(),
        delta: Some(Delta {
            kind: DeltaKind::Up,
            formatted: "+12.4% vs last mo".into(),
        }),
        sparkline: Some(vec![
            100.0, 105.0, 102.0, 110.0, 108.0, 115.0, 112.0, 118.0, 120.0, 125.0,
        ]),
    };

    let graphics = kpi.emit_graphics(&theme, viewport);
    let root = app.stage().root();
    let _ = app.stage_mut().add_child(root, graphics);

    let font = Font::bitmap_8x8(&app);
    for text in kpi.emit_text_labels(&theme, viewport, &font) {
        let _ = app.stage_mut().add_child(root, text);
    }

    app.device().push_error_scope(wgpu::ErrorFilter::Validation);
    let _stats = renderer.render_stage(&app, rt.view(), wisp::Color::WHITE, app.stage());
    let validation = block_on(app.device().pop_error_scope());
    assert!(
        validation.is_none(),
        "wgpu validation error: {validation:?}"
    );

    let bytes = rt.read_pixels(&app);
    let snapshot: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/kpi.png");
    if let Some(parent) = snapshot.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8).expect("write kpi.png");
}
