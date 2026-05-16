#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Area-chart snapshot — filled region between line + baseline.

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
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::RenderTexture;
use wisp_chart::Theme;
use wisp_chart::plot::{self, DataFrame, Interpolation, Mark, Plot, ScaleKind, Value};

const W: u32 = 480;
const H: u32 = 320;

fn fixture() -> DataFrame {
    let rows: Vec<(&'static str, f32)> = vec![
        ("Q1", 24.0),
        ("Q2", 38.0),
        ("Q3", 32.0),
        ("Q4", 56.0),
        ("Q5", 48.0),
        ("Q6", 64.0),
        ("Q7", 72.0),
    ];
    DataFrame::from_rows(&rows, |(q, v)| {
        vec![
            ("quarter".into(), Value::Category((*q).into())),
            ("value".into(), Value::Number(*v)),
        ]
    })
}

#[test]
fn area_renders_to_snapshot() {
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
        .mark(Mark::Area {
            interpolation: Interpolation::Linear,
        })
        .x_title("Period")
        .y_title("Revenue")
        .encode(plot::x("quarter", ScaleKind::Band))
        .encode(plot::y("value", ScaleKind::Linear));

    let graphics = plot.render(&theme, viewport);
    let root = app.stage().root();
    let _ = app.stage_mut().add_child(root, graphics);

    let pipeline = wisp_chart::chart_text::pipeline_with_inter(&app, wisp::wgpu::TextureFormat::Rgba8UnormSrgb);
    for node in plot.axis_text_nodes(&app, &pipeline, &theme, viewport) {
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
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/area.png");
    if let Some(parent) = snapshot.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8).expect("write area.png");
}
