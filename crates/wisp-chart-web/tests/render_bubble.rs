#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Bubble-chart snapshot — Point mark + Size encoding (Area mapping).

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
use wisp_chart::plot::{self, DataFrame, Mark, Plot, PointShape, ScaleKind, SizeMapping, Value};

const W: u32 = 480;
const H: u32 = 320;

fn fixture() -> DataFrame {
    // Gapminder-style: gdp × life expectancy × population × continent.
    let rows: Vec<(f32, f32, f32, &'static str)> = vec![
        (2.0, 65.0, 100.0, "Africa"),
        (3.0, 68.0, 200.0, "Africa"),
        (4.5, 72.0, 80.0, "Africa"),
        (6.0, 70.0, 300.0, "Africa"),
        (7.5, 76.0, 50.0, "Asia"),
        (10.0, 75.0, 1400.0, "Asia"),
        (12.0, 78.0, 200.0, "Asia"),
        (15.0, 82.0, 600.0, "Asia"),
        (18.0, 81.0, 100.0, "Europe"),
        (22.0, 83.0, 80.0, "Europe"),
        (28.0, 84.0, 60.0, "Europe"),
        (35.0, 81.5, 330.0, "Americas"),
        (42.0, 83.5, 50.0, "Americas"),
    ];
    DataFrame::from_rows(&rows, |(gdp, life, pop, cont)| {
        vec![
            ("gdp".into(), Value::Number(*gdp)),
            ("life".into(), Value::Number(*life)),
            ("population".into(), Value::Number(*pop)),
            ("continent".into(), Value::Category((*cont).into())),
        ]
    })
}

#[test]
fn bubble_renders_to_snapshot() {
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
        .mark(Mark::Point {
            shape: PointShape::Circle,
        })
        .encode(plot::x("gdp", ScaleKind::Linear))
        .encode(plot::y("life", ScaleKind::Linear))
        .encode(plot::size("population").size_mapping(SizeMapping::Area))
        .encode(plot::color("continent"));

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
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/bubble.png");
    if let Some(parent) = snapshot.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8).expect("write bubble.png");
}
