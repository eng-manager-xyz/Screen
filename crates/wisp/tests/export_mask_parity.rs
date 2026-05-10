//! AUT-27 — preview / export pixel parity for masked scenes.
//!
//! The renderer has a single `render_stage` entry point that drives
//! both preview and headless export. The architectural contract is
//! "render once, get the same bytes" regardless of which RT view the
//! caller binds. These tests lock that contract in for every M-MASK
//! / M-VEC primitive that touches mask data:
//!
//! - `apply_clip` (rounded crop)
//! - `apply_privacy_blur` (privacy mask)
//! - `apply_solid_redaction` (trust mask)
//! - `apply_spotlight` (focus mask)
//! - The freehand `*_vector` and `*_path_clip` variants
//!
//! Strategy: build a scene, render to RT-A (the "preview" RT) and
//! RT-B (the "export" RT) with identical inputs, assert
//! `read_pixels` returns equal byte slices. Same code path, same
//! bytes — guards against any future preview-only shortcut creeping
//! in.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, MaskShape, RenderTexture, Stage, Vector, VectorShape};

const W: u32 = 96;
const H: u32 = 96;

fn skip_on_software_adapter() -> bool {
    if std::env::var_os("WISP_SKIP_GPU_FILTER_TESTS").is_some() {
        eprintln!(
            "WISP_SKIP_GPU_FILTER_TESTS set — skipping export-parity test \
             (lavapipe loses the device on multi-bind-group filter pipelines)"
        );
        return true;
    }
    false
}

fn boot() -> (Application, Renderer) {
    let app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("app");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("renderer");
    (app, renderer)
}

fn red_blue_split_base(app: &Application, renderer: &Renderer) -> RenderTexture {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let base = RenderTexture::with_format(app, W, H, format);
    let mut stage = Stage::new();
    let mut left = Graphics::new();
    left.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
    left.draw_rect(Rect::new(-1.0, -1.0, 1.0, 2.0));
    let _ = stage.add_child(stage.root(), left);
    let mut right = Graphics::new();
    right.fill(Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)));
    right.draw_rect(Rect::new(0.0, -1.0, 1.0, 2.0));
    let _ = stage.add_child(stage.root(), right);
    let _ = renderer.render_stage(app, base.view(), Color::BLACK, &stage);
    base
}

fn run_twice<F>(app: &Application, mut render: F) -> (Vec<u8>, Vec<u8>)
where
    F: FnMut(&RenderTexture),
{
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let preview = RenderTexture::with_format(app, W, H, format);
    let export = RenderTexture::with_format(app, W, H, format);
    render(&preview);
    render(&export);
    (preview.read_pixels(app), export.read_pixels(app))
}

#[test]
fn rounded_clip_preview_matches_export() {
    let (app, renderer) = boot();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let foreground = RenderTexture::with_format(&app, W, H, format);
    {
        let mut stage = Stage::new();
        let mut g = Graphics::new();
        g.fill(Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)));
        g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
        let _ = stage.add_child(stage.root(), g);
        let _ = renderer.render_stage(&app, foreground.view(), Color::TRANSPARENT, &stage);
    }
    let shape = MaskShape::rounded_rect(Rect::new(-0.5, -0.5, 1.0, 1.0), 0.2);

    let (a, b) = run_twice(&app, |out| {
        renderer.apply_clip(&app, shape, &foreground, out);
    });
    assert_eq!(a, b, "preview ≠ export bytes for apply_clip");
}

#[test]
fn solid_redaction_preview_matches_export() {
    let (app, renderer) = boot();
    let base = red_blue_split_base(&app, &renderer);
    let shape = MaskShape::rounded_rect(Rect::new(-0.4, -0.4, 0.8, 0.8), 0.15);
    let color = Color::rgba_u8(20, 20, 20, 255);

    let (a, b) = run_twice(&app, |out| {
        renderer.apply_solid_redaction(&app, shape, color, &base, out);
    });
    assert_eq!(a, b, "preview ≠ export bytes for apply_solid_redaction");
}

#[test]
fn spotlight_preview_matches_export() {
    let (app, renderer) = boot();
    let base = red_blue_split_base(&app, &renderer);
    let shape = MaskShape::rounded_rect(Rect::new(-0.4, -0.4, 0.8, 0.8), 0.15);

    let (a, b) = run_twice(&app, |out| {
        renderer.apply_spotlight(&app, shape, Color::rgba(0.0, 0.0, 0.0, 0.7), &base, out);
    });
    assert_eq!(a, b, "preview ≠ export bytes for apply_spotlight");
}

#[test]
fn privacy_blur_preview_matches_export() {
    if skip_on_software_adapter() {
        return;
    }
    let (app, renderer) = boot();
    let base = red_blue_split_base(&app, &renderer);
    let shape = MaskShape::rounded_rect(Rect::new(-0.4, -0.4, 0.8, 0.8), 0.15);

    let (a, b) = run_twice(&app, |out| {
        renderer.apply_privacy_blur(&app, shape, 8.0, &base, out);
    });
    assert_eq!(a, b, "preview ≠ export bytes for apply_privacy_blur");
}

#[test]
fn path_clip_vector_preview_matches_export() {
    let (app, renderer) = boot();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let foreground = RenderTexture::with_format(&app, W, H, format);
    {
        let mut stage = Stage::new();
        let mut g = Graphics::new();
        g.fill(Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)));
        g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
        let _ = stage.add_child(stage.root(), g);
        let _ = renderer.render_stage(&app, foreground.view(), Color::TRANSPARENT, &stage);
    }
    let diamond = Vector::new(VectorShape::path(vec![
        Vec2::new(0.0, 0.6),
        Vec2::new(0.6, 0.0),
        Vec2::new(0.0, -0.6),
        Vec2::new(-0.6, 0.0),
    ]));

    let (a, b) = run_twice(&app, |out| {
        renderer.apply_clip_vector(&app, &diamond, &foreground, out);
    });
    assert_eq!(a, b, "preview ≠ export bytes for apply_clip_vector (path)");
}
