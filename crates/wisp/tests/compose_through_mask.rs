//! AUT-46 / AUT-47 — explicit mask-texture composition primitives.
//!
//! M-DYN.4 ships `compose_solid_through_mask` (redaction with the
//! mask supplied externally). M-DYN.5 ships
//! `compose_dim_through_inverted_mask` (spotlight, same idea with an
//! inverted mask). Both are the lower-level companion to the
//! M-VEC.5 / M-VEC.6 high-level primitives — useful when callers
//! need to share a mask across multiple effects.
//!
//! This file locks in:
//! 1. Explicit-mask redaction matches the high-level path
//!    byte-equivalent.
//! 2. Explicit-inverted-mask spotlight matches the high-level path
//!    byte-equivalent.

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, MaskShape, RenderTexture, Stage};

const W: u32 = 96;
const H: u32 = 96;

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

fn red_base(app: &Application, renderer: &Renderer) -> RenderTexture {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let base = RenderTexture::with_format(app, W, H, format);
    let mut stage = Stage::new();
    let mut bg = Graphics::new();
    bg.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), bg);
    let _ = renderer.render_stage(app, base.view(), Color::BLACK, &stage);
    base
}

#[test]
fn explicit_mask_redaction_matches_high_level() {
    let (app, renderer) = boot();
    let base = red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let shape = MaskShape::rounded_rect(Rect::new(-0.4, -0.4, 0.8, 0.8), 0.15);
    let color = Color::rgba_u8(20, 30, 200, 255);

    let out_high = RenderTexture::with_format(&app, W, H, format);
    renderer.apply_solid_redaction(&app, shape, color, &base, &out_high);

    let mask = renderer.generate_mask_texture(&app, shape, W, H);
    let out_explicit = RenderTexture::with_format(&app, W, H, format);
    renderer.compose_solid_through_mask(&app, &base, color, &mask, &out_explicit);

    assert_eq!(
        out_high.read_pixels(&app),
        out_explicit.read_pixels(&app),
        "explicit-mask redaction must match apply_solid_redaction"
    );
}

#[test]
fn explicit_inverted_mask_spotlight_matches_high_level() {
    let (app, renderer) = boot();
    let base = red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let shape = MaskShape::rounded_rect(Rect::new(-0.4, -0.4, 0.8, 0.8), 0.15);
    let dim_color = Color::rgba(0.0, 0.0, 0.0, 0.7);

    let out_high = RenderTexture::with_format(&app, W, H, format);
    renderer.apply_spotlight(&app, shape, dim_color, &base, &out_high);

    let inverted_mask = renderer.generate_mask_texture_inverted(&app, shape, W, H);
    let out_explicit = RenderTexture::with_format(&app, W, H, format);
    renderer.compose_dim_through_inverted_mask(
        &app,
        &base,
        dim_color,
        &inverted_mask,
        &out_explicit,
    );

    assert_eq!(
        out_high.read_pixels(&app),
        out_explicit.read_pixels(&app),
        "explicit-mask spotlight must match apply_spotlight"
    );
}
