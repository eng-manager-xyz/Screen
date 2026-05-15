#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Bar + error-bars overlay snapshot.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "viewport bounded by W=480, H=320"
)]

use std::path::PathBuf;

use glam::Vec2;
use pollster::block_on;
use wisp::RenderTexture;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp_chart::Theme;
use wisp_chart::plot::{self, Mark, Plot, ScaleKind};
use wisp_chart_web::fixtures::{bar_fixture, error_bars_fixture};

const W: u32 = 480;
const H: u32 = 320;

#[test]
fn error_bars_overlay_renders_to_snapshot() {
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

    let bar = Plot::new(bar_fixture())
        .axes(false)
        .mark(Mark::Bar {
            value_labels: false,
        })
        .encode(plot::x("quarter", ScaleKind::Band))
        .encode(plot::y("revenue", ScaleKind::Linear));
    let bar_g = bar.render(&theme, viewport);
    let root = app.stage().root();
    let _ = app.stage_mut().add_child(root, bar_g);

    let plot_rect = Rect::new(60.0, 40.0, viewport.x - 80.0, viewport.y - 80.0);
    let bars = error_bars_fixture();
    let overlay = bars.emit_graphics_in_rect(&theme, viewport, plot_rect);
    let _ = app.stage_mut().add_child(root, overlay);

    app.device().push_error_scope(wgpu::ErrorFilter::Validation);
    let _stats = renderer.render_stage(&app, rt.view(), wisp::Color::WHITE, app.stage());
    let validation = block_on(app.device().pop_error_scope());
    assert!(
        validation.is_none(),
        "wgpu validation error: {validation:?}"
    );

    let bytes = rt.read_pixels(&app);
    let snapshot: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/error-bars.png");
    if let Some(parent) = snapshot.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write error-bars.png");
}
