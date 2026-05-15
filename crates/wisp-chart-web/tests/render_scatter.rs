#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Scatterplot snapshot — 30 points, 3 species, circle markers.

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
use wisp_chart::plot::{self, DataFrame, Mark, Plot, PointShape, ScaleKind, Value};

const W: u32 = 480;
const H: u32 = 320;

fn fixture() -> DataFrame {
    // Roughly correlated x/y across 3 species.
    let rows: Vec<(f32, f32, &'static str)> = vec![
        (1.5, 2.1, "A"),
        (2.2, 2.8, "A"),
        (3.1, 4.0, "A"),
        (4.5, 5.2, "A"),
        (5.0, 5.9, "A"),
        (6.3, 7.1, "A"),
        (7.0, 8.2, "A"),
        (8.5, 9.4, "A"),
        (9.1, 9.8, "A"),
        (10.0, 11.2, "A"),
        (1.0, 4.0, "B"),
        (2.5, 5.0, "B"),
        (3.7, 6.4, "B"),
        (4.8, 7.2, "B"),
        (5.5, 8.1, "B"),
        (6.8, 9.0, "B"),
        (7.9, 10.1, "B"),
        (9.0, 11.3, "B"),
        (10.5, 12.0, "B"),
        (11.0, 13.0, "B"),
        (1.2, 6.0, "C"),
        (3.0, 7.5, "C"),
        (4.5, 9.0, "C"),
        (6.0, 10.5, "C"),
        (7.5, 12.0, "C"),
        (9.0, 13.5, "C"),
        (10.5, 14.5, "C"),
        (12.0, 15.0, "C"),
    ];
    DataFrame::from_rows(&rows, |(x, y, sp)| {
        vec![
            ("x".into(), Value::Number(*x)),
            ("y".into(), Value::Number(*y)),
            ("species".into(), Value::Category((*sp).into())),
        ]
    })
}

#[test]
fn scatter_renders_to_snapshot() {
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
        .encode(plot::x("x", ScaleKind::Linear))
        .encode(plot::y("y", ScaleKind::Linear))
        .encode(plot::color("species"));

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
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/scatter.png");
    if let Some(parent) = snapshot.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write scatter.png");
}
