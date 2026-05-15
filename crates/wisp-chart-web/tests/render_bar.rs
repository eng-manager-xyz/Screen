#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! AUT-186 / M-CHART.6 — integration test for the Plot facade
//! rendering a `Mark::Bar`.
//!
//! Builds a 4-bar fixture through the grammar
//! (`Plot::new(df).mark(Bar).encode(X(Band)).encode(Y(Linear))`),
//! renders to an offscreen `Rgba8Unorm` `RenderTexture` via
//! `wisp::Renderer`, reads pixels back, and asserts:
//! 1. background pixel above the bars is `theme.bg` (white),
//! 2. centre pixel of `Q4` (the tallest bar) is inside the bar
//!    AND non-white.
//!
//! Snapshot PNG written to
//! `_docs/wisp-chart-book/src/assets/wisp-chart-web/bar-quarterly.png`
//! as the PR-visible proof.

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
use wisp_chart::plot::{self, DataFrame, Mark, Plot, ScaleKind, Value};

const W: u32 = 480;
const H: u32 = 320;

#[derive(Clone)]
struct Sale {
    quarter: &'static str,
    revenue: f32,
}

fn fixture_df() -> DataFrame {
    let rows = vec![
        Sale {
            quarter: "Q1",
            revenue: 38.0,
        },
        Sale {
            quarter: "Q2",
            revenue: 52.0,
        },
        Sale {
            quarter: "Q3",
            revenue: 47.0,
        },
        Sale {
            quarter: "Q4",
            revenue: 64.0,
        },
    ];
    DataFrame::from_rows(&rows, |s| {
        vec![
            ("quarter".into(), Value::Category(s.quarter.into())),
            ("revenue".into(), Value::Number(s.revenue)),
        ]
    })
}

fn read_pixel(bytes: &[u8], x: u32, y: u32) -> [u8; 4] {
    let stride = (W * 4) as usize;
    let i = (y as usize) * stride + (x as usize) * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

#[test]
fn plot_facade_renders_4_quarter_bar_chart() {
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
    let plot = Plot::new(fixture_df())
        .mark(Mark::Bar {
            value_labels: false,
        })
        .x_title("Quarter")
        .y_title("Revenue")
        .encode(plot::x("quarter", ScaleKind::Band))
        .encode(plot::y("revenue", ScaleKind::Linear));

    let graphics = plot.render(&theme, viewport);
    let root = app.stage().root();
    let _ = app.stage_mut().add_child(root, graphics);

    // Axis text — Plot can't bake these into Graphics because
    // Text needs a Font. Wire the bitmap_8x8 font here and add
    // each label / title as a sibling Text node.
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

    // Snapshot first so a failing assertion still leaves the
    // committed PNG up-to-date for inspection.
    let snapshot_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/bar-quarterly.png");
    if let Some(parent) = snapshot_path.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot_path, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write bar-quarterly.png");

    // Assertion 1: pixel high in the chart area (above bars) is
    // theme.bg = white. y = 50 is in the header area where no
    // bar reaches.
    let bg = read_pixel(&bytes, W / 2, 50);
    assert_eq!(
        bg,
        [255, 255, 255, 255],
        "header area should be white, got {bg:?}"
    );

    // Assertion 2: Q4 (the tallest bar at x ≈ 0.875 of plot
    // width) has its centre pixel non-white. The bar's top is
    // around y = 60ish (since 64 is the max revenue, the bar
    // extends from baseline y ≈ 280 up to ~60 px from header).
    // Sample y = 200 which is roughly in the middle of Q4's
    // bar.
    let q4_center_x = (W as f32 * 0.85) as u32;
    let q4_mid = read_pixel(&bytes, q4_center_x, 200);
    assert!(
        q4_mid != [255, 255, 255, 255],
        "Q4 bar centre should be a non-white fill colour, got {q4_mid:?}"
    );
}
