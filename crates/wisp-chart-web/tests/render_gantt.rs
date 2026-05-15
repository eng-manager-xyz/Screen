#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Native render test for [`wisp_chart_web::render_gantt`].
//!
//! Runs the full pipeline (`wisp::Application` → `wisp::Renderer`
//! → `wisp_chart::Gantt::emit_graphics` → render pass) against an
//! offscreen `Rgba8Unorm` `RenderTexture` on whichever wgpu
//! backend the host exposes (Metal / Vulkan / DX12). Asserts two
//! pixels that pin the layout AND the colour table:
//!
//! 1. The header region (above the chart's first row) reads as
//!    `theme.bg` — proves `Gantt::emit_graphics`'s background
//!    primitive painted.
//! 2. The centre of the first bar reads as Matt's Wong-navy
//!    `#0072b2` (within a small SDF-AA tolerance) — proves the
//!    layout math placed the bar where expected AND the explicit
//!    `PersonMap` override resolved to the right colour.
//!
//! Also writes the rendered output to
//! `_docs/wisp-chart-book/src/assets/wisp-chart-web/gantt-demo.png`
//! — the PR-visible proof that this test asserted on real pixels.
//!
//! Does **not** validate the `BROWSER_WEBGPU` surface-presentation
//! path. That's `tests/headless_webgpu.rs`'s job (local-only).

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "viewport dims and pixel coords are bounded by W=960, H=400 — \
              well below the f32 precision boundary, and `round()` plus the \
              non-negative pixel-rect outputs make sign loss / truncation safe."
)]

use std::path::PathBuf;

use glam::Vec2;
use pollster::block_on;
use wisp::RenderTexture;
use wisp::application::{AppConfig, Application};
use wisp_chart::Theme;
use wisp_chart::gantt::layout::bar_pixel_rect;
use wisp_chart_web::{render_gantt, sample_gantt};

const W: u32 = 960;
const H: u32 = 400;

#[test]
fn render_gantt_paints_background_and_known_bar() {
    let mut app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("Application::new");

    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);

    let gantt = sample_gantt();
    let theme = Theme::light();
    let viewport = Vec2::new(W as f32, H as f32);

    app.device().push_error_scope(wgpu::ErrorFilter::Validation);
    render_gantt(
        &mut app,
        rt.view(),
        wgpu::TextureFormat::Rgba8Unorm,
        viewport,
        &gantt,
        &theme,
    )
    .expect("render_gantt");
    let validation = block_on(app.device().pop_error_scope());
    assert!(
        validation.is_none(),
        "wgpu validation error during render_gantt: {validation:?}"
    );

    let bytes = rt.read_pixels(&app);
    assert_eq!(
        bytes.len(),
        (W * H * 4) as usize,
        "unexpected readback length"
    );

    // Snapshot FIRST so a failing assertion still leaves the
    // committed PNG up-to-date for inspection.
    let snapshot_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/gantt-demo.png");
    if let Some(parent) = snapshot_path.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot_path, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write gantt-demo.png");

    // Assertion 1: a pixel inside the header band (y < header_height,
    // x > gutter) reads as theme.bg = white. Picked y = 10, x = 400
    // — well inside header, well past gutter.
    let bg_pixel = read_pixel(&bytes, 400, 10);
    assert_eq!(
        bg_pixel,
        [255, 255, 255, 255],
        "header-area pixel should be theme.bg = #ffffff"
    );

    // Assertion 2: centre pixel of bar[0] reads as Matt's Wong
    // navy `#0072b2` (within ±2 to absorb SDF anti-alias noise).
    let bar0 = &gantt.bars[0];
    let rect = bar_pixel_rect(bar0, &gantt, &theme, viewport.x).expect("bar0 resolves");
    let cx = (rect.x + rect.w * 0.5).round() as u32;
    let cy = (rect.y + rect.h * 0.5).round() as u32;
    let centre = read_pixel(&bytes, cx, cy);
    let expected_matt = [0x00, 0x72, 0xb2, 0xff];
    assert_pixel_near(
        centre,
        expected_matt,
        2,
        &format!("bar[0] centre @ ({cx}, {cy})"),
    );

    // Assertion 3: centre pixel of bar[2] (M-DYN row, owner
    // Alice = Wong vermillion `#d55e00`) reads as vermillion.
    let bar2 = &gantt.bars[2];
    let rect2 = bar_pixel_rect(bar2, &gantt, &theme, viewport.x).expect("bar2 resolves");
    let cx2 = (rect2.x + rect2.w * 0.5).round() as u32;
    let cy2 = (rect2.y + rect2.h * 0.5).round() as u32;
    let centre2 = read_pixel(&bytes, cx2, cy2);
    let expected_alice = [0xd5, 0x5e, 0x00, 0xff];
    assert_pixel_near(
        centre2,
        expected_alice,
        2,
        &format!("bar[2] centre @ ({cx2}, {cy2})"),
    );
}

fn read_pixel(bytes: &[u8], x: u32, y: u32) -> [u8; 4] {
    let stride = (W * 4) as usize;
    let i = (y as usize) * stride + (x as usize) * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

fn assert_pixel_near(actual: [u8; 4], expected: [u8; 4], tol: u8, label: &str) {
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        let diff = a.abs_diff(*e);
        assert!(
            diff <= tol,
            "{label}: channel {i} got {a}, expected {e} ± {tol} \
             (full pixel got {actual:?}, expected {expected:?})"
        );
    }
}
