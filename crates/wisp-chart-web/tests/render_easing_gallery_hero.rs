#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Hero snapshot for the Easing-gallery chapter — renders the
//! 36-card grid at `progress = 0.6` so every curve has its dot at
//! a non-trivial place along the eased value.
//!
//! Runs against an offscreen `RenderTexture`; the resulting PNG is
//! the asset embedded into
//! `_docs/wisp-animation-book/src/chunks/easing-gallery.md` and
//! used as the iframe poster background.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "viewport bounded"
)]

use std::path::PathBuf;

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Font, RenderTexture};
use wisp_chart_web::easing_grid;

// Authoritative canvas size — must match `easing_grid::CANVAS_W/H`
// so the same NDC layout produces legible text in both the
// snapshot and the live WebGPU demo.
const W: u32 = wisp_chart_web::easing_grid::CANVAS_W;
const H: u32 = wisp_chart_web::easing_grid::CANVAS_H;

#[test]
fn easing_gallery_hero_renders_to_snapshot() {
    let mut app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("Application::new");

    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let font = Font::bitmap_8x8(&app);

    let root = app.stage().root();
    let _ = app
        .stage_mut()
        .add_child(root, easing_grid::build_static_layer());
    for label in easing_grid::build_labels(&font) {
        let _ = app.stage_mut().add_child(root, label);
    }

    // Frozen progress for the snapshot — 0.6 puts each dot ~60% of
    // the way along its curve, which is the most informative
    // single frame: Out-family curves are near their asymptote,
    // Back/Elastic are mid-rebound, Bounce is between bounces.
    let progress = 0.6;
    let _ = app
        .stage_mut()
        .add_child(root, easing_grid::build_dot_layer(progress));
    let _ = app
        .stage_mut()
        .add_child(root, easing_grid::build_progress_label(&font, progress));

    app.device().push_error_scope(wgpu::ErrorFilter::Validation);
    let _ = renderer.render_stage(&app, rt.view(), wisp::Color::WHITE, app.stage());
    let validation = block_on(app.device().pop_error_scope());
    assert!(validation.is_none());

    let bytes = rt.read_pixels(&app);
    let snapshot: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-animation-book/src/assets/wisp-animation/easing-gallery-hero.png");
    std::fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write easing-gallery-hero.png");
}
