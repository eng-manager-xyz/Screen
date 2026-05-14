//! Connected-scatterplot snapshot — `Mark::Line` with Linear X +
//! Order encoding.

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
use wisp::render::Renderer;
use wisp_chart::Theme;
use wisp_chart::plot::{self, DataFrame, Interpolation, Mark, Plot, PointStyle, ScaleKind, Value};

const W: u32 = 480;
const H: u32 = 320;

fn fixture() -> DataFrame {
    // Phillips-curve-style fixture: inflation vs unemployment, sorted
    // by quarter index. Rows intentionally NOT in order so the Order
    // encoding has work to do.
    let rows: Vec<(f32, f32, f32)> = vec![
        (3.0, 5.5, 3.0),
        (2.5, 6.0, 1.0),
        (2.8, 5.8, 2.0),
        (3.5, 5.2, 4.0),
        (4.2, 4.9, 5.0),
        (5.0, 4.5, 6.0),
        (4.5, 4.7, 7.0),
        (5.8, 4.3, 8.0),
    ];
    DataFrame::from_rows(&rows, |(infl, unemp, step)| {
        vec![
            ("inflation".into(), Value::Number(*infl)),
            ("unemployment".into(), Value::Number(*unemp)),
            ("step".into(), Value::Number(*step)),
        ]
    })
}

#[test]
fn connected_scatter_renders_to_snapshot() {
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
    let plot = Plot::new(fixture())
        .mark(Mark::Line {
            interpolation: Interpolation::Linear,
            marker: Some(PointStyle::Circle),
        })
        .encode(plot::x("inflation", ScaleKind::Linear))
        .encode(plot::y("unemployment", ScaleKind::Linear))
        .encode(plot::order("step"));

    let graphics = plot.render(&theme, viewport);
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
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/connected-scatter.png");
    if let Some(parent) = snapshot.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write connected-scatter.png");
}
