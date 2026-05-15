//! Hero for the Performance chapter (M-ANIM.20). 12×12 grid of
//! ellipses with a wave-pattern alpha distribution.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::doc_markdown,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "viewport bounded; hero-test prose"
)]

use std::path::PathBuf;

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::scene::{Fill, Graphics};
use wisp::{Color, RenderTexture};
use wisp_chart::Theme;

const W: u32 = 360;
const H: u32 = 360;

#[test]
fn many_hero_renders_to_snapshot() {
    let mut app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("Application::new");
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");
    let _theme = Theme::light();
    let root = app.stage().root();
    let count = 12_u32;
    for row in 0..count {
        for col in 0..count {
            let mut g = Graphics::new();
            // Alpha forms a diagonal wave: center bright, corners
            // dim. The visual on a single static frame.
            let centre = (count - 1) as f32 / 2.0;
            let dx = col as f32 - centre;
            let dy = row as f32 - centre;
            let dist = (dx * dx + dy * dy).sqrt();
            let max_dist = (centre * centre * 2.0).sqrt();
            let alpha = 1.0 - (dist / max_dist).min(1.0);
            g.fill(Fill::Solid(Color {
                r: 0.0,
                g: 0.5,
                b: 0.85,
                a: 1.0,
            }));
            g.container.alpha = 0.2 + 0.8 * alpha;
            let x = (col as f32 / (count as f32 - 1.0) - 0.5) * 1.6;
            let y = (row as f32 / (count as f32 - 1.0) - 0.5) * 1.6;
            g.draw_ellipse(Vec2::new(x, y), Vec2::splat(0.05));
            let _ = app.stage_mut().add_child(root, g);
        }
    }
    app.device().push_error_scope(wgpu::ErrorFilter::Validation);
    let _ = renderer.render_stage(&app, rt.view(), Color::WHITE, app.stage());
    let validation = block_on(app.device().pop_error_scope());
    assert!(validation.is_none());
    let bytes = rt.read_pixels(&app);
    let snapshot: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-animation-book/src/assets/wisp-animation/many-hero.png");
    std::fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
    image::save_buffer(&snapshot, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write many-hero.png");
}
