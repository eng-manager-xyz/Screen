#![allow(
    clippy::doc_markdown,
    reason = "hero-test prose references variant names without backticks"
)]

//! Hero snapshot for the `wisp-animation` book's
//! [Animation trait + Driver chapter][1].
//!
//! Renders the polar plot at a mid-animation rotation (~0.6 rad)
//! so the still-frame fallback behind the iframe communicates
//! "this chart is being spun by an animation" rather than
//! "this chart is stationary".
//!
//! [1]: ../../../_docs/wisp-animation-book/src/chunks/animation-trait-driver.md
//!
//! Reuses the same path the live demo uses: `polar_plot_fixture` →
//! `emit_graphics` → mutate `container.transform.rotation` →
//! `Renderer::render_stage`. The only difference is fixed-rotation
//! vs animated rotation.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "viewport bounded by W=360, H=360"
)]

use std::path::PathBuf;

use glam::Vec2;
use pollster::block_on;
use wisp::RenderTexture;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::scene::Transform;
use wisp_chart::Theme;
use wisp_chart_web::fixtures::polar_plot_fixture;

const W: u32 = 360;
const H: u32 = 360;
const MID_ANIMATION_ROTATION: f32 = 0.6_f32;

#[test]
fn polar_spin_hero_renders_to_snapshot() {
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
    let plot = polar_plot_fixture();
    let mut graphics = plot.emit_graphics(&theme, viewport);
    graphics.container.transform = Transform::from_rotation(MID_ANIMATION_ROTATION);

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
    let snapshot: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../_docs/wisp-animation-book/src/assets/wisp-animation/animation-trait-driver-hero.png",
    );
    if let Some(parent) = snapshot.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write animation-trait-driver-hero.png");
}
