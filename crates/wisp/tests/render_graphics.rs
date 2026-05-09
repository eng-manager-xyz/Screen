//! M0.12 — Graphics solid-fill integration tests.
//!
//! Renders rect + rounded rect to a `RenderTexture` and verifies pixel-level
//! presence/absence at sample points.

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, RenderTexture, Stage};

fn boot() -> Application {
    block_on(Application::new(AppConfig::default())).expect("init wisp")
}

#[test]
fn empty_graphics_node_emits_no_draw() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, 16, 16, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), Graphics::new());

    let stats = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    assert_eq!(stats.draw_calls, 0);
    assert_eq!(stats.graphics_drawn, 0);
}

#[test]
fn solid_rect_fills_specified_region() {
    let app = boot();
    // 32×32 in linear format. NDC quad covering [-0.5, 0.5] = pixels [8, 24).
    let rt = RenderTexture::with_format(&app, 32, 32, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(0, 255, 0, 255)));
    g.draw_rect(Rect::new(-0.5, -0.5, 1.0, 1.0));

    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let stats = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    assert_eq!(stats.draw_calls, 1);
    assert_eq!(stats.graphics_drawn, 1);

    let bytes = rt.read_pixels(&app);
    // Center pixel (16, 16): inside rect → should be green.
    let center_idx = (16 * 32 + 16) * 4;
    assert!(
        bytes[center_idx + 1] > 200,
        "center G should be ~255 (green), got {}",
        bytes[center_idx + 1]
    );
    // Corner pixel (1, 1): outside rect → should be cleared (black).
    let corner_idx = (32 + 1) * 4;
    assert!(
        bytes[corner_idx + 1] < 20,
        "corner G should be ~0 (black), got {}",
        bytes[corner_idx + 1]
    );
}

#[test]
fn rounded_rect_clears_corners() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, 64, 64, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(255, 255, 255, 255)));
    // Rounded rect occupying full NDC with substantial radius.
    g.draw_rounded_rect(Rect::new(-1.0, -1.0, 2.0, 2.0), 0.5);

    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);

    let bytes = rt.read_pixels(&app);
    // Center should be white.
    let center_idx = (32 * 64 + 32) * 4;
    assert!(
        bytes[center_idx] > 200,
        "center R should be white, got {}",
        bytes[center_idx]
    );
    // Top-left pixel should be cleared (rounded corner cuts it out).
    let corner_idx = 0;
    assert!(
        bytes[corner_idx] < 50,
        "top-left R should be black (corner cut), got {}",
        bytes[corner_idx]
    );
}

#[test]
fn many_primitives_batch_into_one_draw_call() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, 32, 32, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::WHITE));
    for i in 0u8..50 {
        let f = f32::from(i) / 50.0;
        g.draw_rect(Rect::new(f - 1.0, -0.05, 0.02, 0.1));
    }

    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let stats = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    assert_eq!(stats.graphics_drawn, 50);
    // All primitives across all Graphics nodes batch into a single draw call.
    assert_eq!(stats.draw_calls, 1);
}
