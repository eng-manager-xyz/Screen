#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Gauge snapshot — semicircle + 3 threshold zones + needle.

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
use wisp::RenderTexture;
use wisp_chart::Theme;
use wisp_chart::color::Color as ChartColor;
use wisp_chart::indicator::{Gauge, Zone};

const W: u32 = 320;
const H: u32 = 200;

#[test]
fn gauge_renders_to_snapshot() {
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

    let gauge = Gauge {
        value: 73.0,
        domain: (0.0, 100.0),
        zones: vec![
            Zone::new((0.0, 60.0), ChartColor::from_hex("#27ae60").unwrap()),
            Zone::new((60.0, 85.0), ChartColor::from_hex("#f5a623").unwrap()),
            Zone::new((85.0, 100.0), ChartColor::from_hex("#e74c3c").unwrap()),
        ],
    };

    let graphics = gauge.emit_graphics(&theme, viewport);
    let root = app.stage().root();
    let _ = app.stage_mut().add_child(root, graphics);

    let pipeline = wisp_chart::chart_text::pipeline_with_inter(
        &app,
        wisp::wgpu::TextureFormat::Rgba8UnormSrgb,
    );
    for node in gauge.emit_text_nodes(&app, &pipeline, &theme, viewport) {
        let _ = app.stage_mut().add_child(root, node);
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
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/gauge.png");
    if let Some(parent) = snapshot.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8).expect("write gauge.png");
}
