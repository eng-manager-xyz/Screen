#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Line-mark snapshot — captures a 3-series step / linear line
//! chart with markers to `line.png` for the mdBook chapter
//! `_docs/wisp-chart-book/src/charts/line.md`.

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
use wisp::{Font, RenderTexture};
use wisp_chart::Theme;
use wisp_chart::plot::{self, DataFrame, Interpolation, Mark, Plot, PointStyle, ScaleKind, Value};

const W: u32 = 480;
const H: u32 = 320;

#[derive(Clone)]
struct Point {
    quarter: &'static str,
    revenue: f32,
    region: &'static str,
}

fn fixture() -> DataFrame {
    let rows = vec![
        Point {
            quarter: "Q1",
            revenue: 38.0,
            region: "NA",
        },
        Point {
            quarter: "Q2",
            revenue: 52.0,
            region: "NA",
        },
        Point {
            quarter: "Q3",
            revenue: 47.0,
            region: "NA",
        },
        Point {
            quarter: "Q4",
            revenue: 64.0,
            region: "NA",
        },
        Point {
            quarter: "Q1",
            revenue: 22.0,
            region: "EU",
        },
        Point {
            quarter: "Q2",
            revenue: 30.0,
            region: "EU",
        },
        Point {
            quarter: "Q3",
            revenue: 42.0,
            region: "EU",
        },
        Point {
            quarter: "Q4",
            revenue: 48.0,
            region: "EU",
        },
    ];
    DataFrame::from_rows(&rows, |p| {
        vec![
            ("quarter".into(), Value::Category(p.quarter.into())),
            ("revenue".into(), Value::Number(p.revenue)),
            ("region".into(), Value::Category(p.region.into())),
        ]
    })
}

#[test]
fn line_chart_renders_to_snapshot() {
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
        .x_title("Quarter")
        .y_title("Revenue")
        .encode(plot::x("quarter", ScaleKind::Band))
        .encode(plot::y("revenue", ScaleKind::Linear))
        .encode(plot::color("region"));

    let graphics = plot.render(&theme, viewport);
    let root = app.stage().root();
    let _ = app.stage_mut().add_child(root, graphics);

    let font = Font::bitmap_8x8(&app);
    for text in plot.axis_text_labels(&theme, viewport, &font) {
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
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/line.png");
    if let Some(parent) = snapshot.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8).expect("write line.png");
}
