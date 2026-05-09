//! M0.12 — Graphics solid-fill integration tests.
//!
//! Renders rect + rounded rect to a `RenderTexture` and verifies pixel-level
//! presence/absence at sample points.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, RenderTexture, Stage, Stroke};

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
fn linear_gradient_visually_red_at_top_blue_at_bottom() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, 32, 32, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let mut g = Graphics::new();
    // Coordinate system: +Y up (NDC). For "red at the visual top, blue at the
    // visual bottom" we put the gradient START at +Y (top) and END at -Y (bottom).
    g.fill(Fill::LinearGradient {
        start: Vec2::new(0.0, 0.5),
        end: Vec2::new(0.0, -0.5),
        color_a: Color::rgba_u8(255, 0, 0, 255),
        color_b: Color::rgba_u8(0, 0, 255, 255),
    });
    g.draw_rect(Rect::new(-0.5, -0.5, 1.0, 1.0));

    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);

    let bytes = rt.read_pixels(&app);
    // Rect spans rows 8..24 (NDC y in [-0.5, +0.5]). Sample well inside.
    // Row 10 ≈ NDC y +0.41 — near top (start, color_a = red).
    let top_idx = (10 * 32 + 16) * 4;
    assert!(
        bytes[top_idx] > 200,
        "top R should be ~255, got {}",
        bytes[top_idx]
    );
    assert!(
        bytes[top_idx + 2] < 60,
        "top B should be ~0, got {}",
        bytes[top_idx + 2]
    );

    // Row 21 ≈ NDC y -0.34 — near bottom (end, color_b = blue).
    let bot_idx = (21 * 32 + 16) * 4;
    assert!(
        bytes[bot_idx] < 60,
        "bottom R should be ~0, got {}",
        bytes[bot_idx]
    );
    assert!(
        bytes[bot_idx + 2] > 200,
        "bottom B should be ~255, got {}",
        bytes[bot_idx + 2]
    );
}

#[test]
fn radial_gradient_center_to_edge() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, 32, 32, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let mut g = Graphics::new();
    g.fill(Fill::RadialGradient {
        center: Vec2::ZERO,
        radius: 0.5,
        color_a: Color::rgba_u8(255, 255, 255, 255),
        color_b: Color::rgba_u8(0, 0, 0, 255),
    });
    g.draw_rect(Rect::new(-0.5, -0.5, 1.0, 1.0));

    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);

    let bytes = rt.read_pixels(&app);
    // Center pixel — bright (close to white).
    let center_idx = (16 * 32 + 16) * 4;
    assert!(
        bytes[center_idx] > 200,
        "center should be near white, got R={}",
        bytes[center_idx]
    );
    // Pixel at half-radius distance (col 20, NDC x=0.25, t=0.5) — gray-ish.
    let mid_idx = (16 * 32 + 20) * 4;
    let mid_r = bytes[mid_idx];
    assert!(
        (60..=200).contains(&mid_r),
        "half-radius pixel should be partway through gradient, got R={mid_r}"
    );
}

#[test]
fn ellipse_fills_center_clears_corner() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, 64, 64, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(255, 0, 0, 255)));
    // Ellipse centered at NDC origin, radii ~ full half-extents.
    g.draw_ellipse(Vec2::ZERO, Vec2::new(0.9, 0.9));

    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);

    let bytes = rt.read_pixels(&app);
    // Center pixel — inside ellipse, expect red.
    let center = (32 * 64 + 32) * 4;
    assert!(
        bytes[center] > 200,
        "center R should be ~255 (red), got {}",
        bytes[center]
    );
    // Corner pixel — outside ellipse, expect black.
    let corner = 0;
    assert!(
        bytes[corner] < 50,
        "corner R should be ~0 (black, outside ellipse), got {}",
        bytes[corner]
    );
}

#[test]
fn line_renders_along_diagonal() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, 64, 64, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::WHITE));
    // Diagonal line through NDC origin with thickness 0.06.
    g.draw_line(Vec2::new(-0.5, -0.5), Vec2::new(0.5, 0.5), 0.06);

    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);

    let bytes = rt.read_pixels(&app);
    // Center pixel sits on the line — expect bright.
    let center = (32 * 64 + 32) * 4;
    assert!(
        bytes[center] > 150,
        "center R should be on the line, got {}",
        bytes[center]
    );
    // Pixel far off the diagonal (top-right corner) — expect dark.
    let off = (5 * 64 + 58) * 4;
    assert!(
        bytes[off] < 50,
        "off-diagonal pixel should be black, got {}",
        bytes[off]
    );
}

#[test]
fn stroked_rect_emits_two_instances_one_draw_call() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, 64, 64, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(0, 200, 0, 255)));
    g.stroke(Some(Stroke::new(0.05, Color::rgba_u8(255, 0, 0, 255))));
    g.draw_rect(Rect::new(-0.5, -0.5, 1.0, 1.0));

    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);

    let stats = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    // 1 logical primitive, but 1 draw call (fill + stroke instances batch).
    assert_eq!(stats.graphics_drawn, 1);
    assert_eq!(stats.draw_calls, 1);

    let bytes = rt.read_pixels(&app);
    // Interior — green fill.
    let center = (32 * 64 + 32) * 4;
    assert!(
        bytes[center + 1] > 150,
        "interior should be green, got G={}",
        bytes[center + 1]
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
