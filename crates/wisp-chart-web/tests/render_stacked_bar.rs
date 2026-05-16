#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Stacked-bar mark snapshot — `Transform::Stack` composition.

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
use wisp_chart::plot::{self, DataFrame, Mark, Plot, ScaleKind, Transform, Value};

const W: u32 = 480;
const H: u32 = 320;

#[derive(Clone)]
struct Sale {
    quarter: &'static str,
    region: &'static str,
    revenue: f32,
}

fn fixture() -> DataFrame {
    let rows = vec![
        Sale {
            quarter: "Q1",
            region: "NA",
            revenue: 38.0,
        },
        Sale {
            quarter: "Q1",
            region: "EU",
            revenue: 22.0,
        },
        Sale {
            quarter: "Q1",
            region: "APAC",
            revenue: 14.0,
        },
        Sale {
            quarter: "Q2",
            region: "NA",
            revenue: 52.0,
        },
        Sale {
            quarter: "Q2",
            region: "EU",
            revenue: 27.0,
        },
        Sale {
            quarter: "Q2",
            region: "APAC",
            revenue: 18.0,
        },
        Sale {
            quarter: "Q3",
            region: "NA",
            revenue: 47.0,
        },
        Sale {
            quarter: "Q3",
            region: "EU",
            revenue: 33.0,
        },
        Sale {
            quarter: "Q3",
            region: "APAC",
            revenue: 22.0,
        },
        Sale {
            quarter: "Q4",
            region: "NA",
            revenue: 64.0,
        },
        Sale {
            quarter: "Q4",
            region: "EU",
            revenue: 40.0,
        },
        Sale {
            quarter: "Q4",
            region: "APAC",
            revenue: 28.0,
        },
    ];
    DataFrame::from_rows(&rows, |s| {
        vec![
            ("quarter".into(), Value::Category(s.quarter.into())),
            ("region".into(), Value::Category(s.region.into())),
            ("revenue".into(), Value::Number(s.revenue)),
        ]
    })
}

#[test]
fn stacked_bar_renders_to_snapshot() {
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
        .mark(Mark::Bar {
            value_labels: false,
        })
        .x_title("Quarter")
        .y_title("Revenue")
        .encode(plot::x("quarter", ScaleKind::Band))
        .encode(plot::y("revenue", ScaleKind::Linear))
        .encode(plot::color("region"))
        .transform(Transform::Stack { normalize: false });

    let graphics = plot.render(&theme, viewport);
    let root = app.stage().root();
    let _ = app.stage_mut().add_child(root, graphics);

    let pipeline = wisp_chart::chart_text::pipeline_with_inter(
        &app,
        wisp::wgpu::TextureFormat::Rgba8UnormSrgb,
    );
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
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/stacked-bar.png");
    if let Some(parent) = snapshot.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write stacked-bar.png");
}
