//! AUT-63 — boolean operations on alpha-mask textures.
//!
//! Three contracts:
//! 1. Union: pixel covered by either mask → alpha 255.
//! 2. Intersect: pixel covered by both masks → alpha 255; only one
//!    mask → alpha 0.
//! 3. Subtract: pixel covered by `a` but not `b` → alpha 255; both →
//!    alpha 0; only `b` → alpha 0.

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{MaskCombineOp, MaskShape, RenderTexture};

const W: u32 = 64;
const H: u32 = 64;

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

fn read_alpha(rt: &RenderTexture, app: &Application, x: u32, y: u32) -> u8 {
    let bytes = rt.read_pixels(app);
    let stride = (W as usize) * 4;
    let i = (y as usize) * stride + (x as usize) * 4;
    bytes[i + 3]
}

#[test]
fn union_covers_either_mask() {
    let (app, renderer) = boot();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    // Two non-overlapping rects: left half and right half.
    let a = renderer.generate_mask_texture(
        &app,
        MaskShape::rect(Rect::new(-1.0, -0.5, 0.8, 1.0)),
        W,
        H,
    );
    let b =
        renderer.generate_mask_texture(&app, MaskShape::rect(Rect::new(0.2, -0.5, 0.8, 1.0)), W, H);
    let out = RenderTexture::with_format(&app, W, H, format);
    renderer.combine_masks(&app, &a, &b, MaskCombineOp::Union, &out);

    // Far left (only `a`) → alpha 255.
    assert_eq!(read_alpha(&out, &app, 4, H / 2), 255, "union far-left");
    // Far right (only `b`) → alpha 255.
    assert_eq!(read_alpha(&out, &app, W - 4, H / 2), 255, "union far-right");
}

#[test]
fn intersect_covers_only_overlap() {
    let (app, renderer) = boot();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    // Two overlapping rects centered: [-0.6, 0.4] × [-0.4, 0.6].
    let a = renderer.generate_mask_texture(
        &app,
        MaskShape::rect(Rect::new(-0.6, -0.5, 1.0, 1.0)),
        W,
        H,
    );
    let b = renderer.generate_mask_texture(
        &app,
        MaskShape::rect(Rect::new(-0.4, -0.5, 1.0, 1.0)),
        W,
        H,
    );
    let out = RenderTexture::with_format(&app, W, H, format);
    renderer.combine_masks(&app, &a, &b, MaskCombineOp::Intersect, &out);

    // Only-`a` zone (NDC -0.5, mid) → alpha 0.
    // NDC -0.5 → pixel (((-0.5+1)/2)*W) = W/4. Integer math avoids
    // clippy::cast_possible_truncation / cast_sign_loss.
    let only_a_x: u32 = W / 4;
    assert_eq!(
        read_alpha(&out, &app, only_a_x, H / 2),
        0,
        "intersect only-a zone"
    );
    // Both-zone center (NDC 0.0) → alpha 255.
    assert_eq!(
        read_alpha(&out, &app, W / 2, H / 2),
        255,
        "intersect overlap zone"
    );
}

#[test]
fn subtract_covers_a_minus_b() {
    let (app, renderer) = boot();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    // `a` is the full center region; `b` is a smaller central region.
    let a = renderer.generate_mask_texture(
        &app,
        MaskShape::rect(Rect::new(-0.6, -0.6, 1.2, 1.2)),
        W,
        H,
    );
    let b = renderer.generate_mask_texture(
        &app,
        MaskShape::rect(Rect::new(-0.2, -0.2, 0.4, 0.4)),
        W,
        H,
    );
    let out = RenderTexture::with_format(&app, W, H, format);
    renderer.combine_masks(&app, &a, &b, MaskCombineOp::Subtract, &out);

    // Center (covered by both) → alpha 0 (a - b cancelled).
    assert_eq!(
        read_alpha(&out, &app, W / 2, H / 2),
        0,
        "subtract center cancelled"
    );
    // Edge of a, outside b (NDC -0.5, mid) → alpha 255 (only `a`).
    // NDC -0.5 → pixel (((-0.5+1)/2)*W) = W/4. Integer math avoids
    // clippy::cast_possible_truncation / cast_sign_loss.
    let only_a_x: u32 = W / 4;
    assert_eq!(
        read_alpha(&out, &app, only_a_x, H / 2),
        255,
        "subtract preserves a-only"
    );
}
