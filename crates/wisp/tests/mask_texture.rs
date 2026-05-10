//! AUT-43 — dynamic alpha-mask texture primitive.
//!
//! `Renderer::generate_mask_texture(shape, w, h)` produces a coverage
//! `RenderTexture`. The output stores `(m, m, m, m)` so consumers can
//! sample as alpha or as grayscale. Five contracts:
//!
//! 1. Rect: fully-inside pixel reads alpha 255; fully-outside reads 0.
//! 2. Rounded-rect: bounding-corner-but-rounded-corner-cutout reads 0
//!    (proves the SDF is honored, not just the bounding box).
//! 3. Circle: corner-of-bounding-square reads 0.
//! 4. Ellipse: anisotropic — point inside bounds but outside curve
//!    reads 0; point inside the ellipse reads ~255.
//! 5. Path: center of a five-pointed star reads ~255 (path variant
//!    works).
//!
//! Inverted variant gets one sanity check (inside reads 0, outside 255).

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{MaskShape, RenderTexture};

const W: u32 = 128;
const H: u32 = 128;

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

fn read_pixel(rt: &RenderTexture, app: &Application, x: u32, y: u32) -> [u8; 4] {
    let bytes = rt.read_pixels(app);
    let stride = (W as usize) * 4;
    let i = (y as usize) * stride + (x as usize) * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

#[test]
fn rect_mask_inside_full_outside_zero() {
    let (app, renderer) = boot();
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    let rt = renderer.generate_mask_texture(&app, MaskShape::rect(region), W, H);

    let inside = read_pixel(&rt, &app, W / 2, H / 2);
    assert_eq!(inside[3], 255, "rect center alpha should be 255");
    assert_eq!(inside[0], 255, "rect center R should mirror alpha");

    let outside = read_pixel(&rt, &app, 4, 4);
    assert_eq!(outside[3], 0, "rect outside alpha should be 0");
    assert_eq!(outside[0], 0, "rect outside R should mirror alpha");
}

#[test]
fn rounded_rect_mask_carves_corner() {
    let (app, renderer) = boot();
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    let rt = renderer.generate_mask_texture(&app, MaskShape::rounded_rect(region, 0.3), W, H);

    // Pixel (33, 33) → NDC ≈ (-0.484, +0.484). Distance to round-
    // corner center (-0.2, +0.2) is ~0.402, which is 0.102 NDC past
    // radius 0.3 — well outside the ~0.016 NDC AA band. Mask=0.
    let cutout = read_pixel(&rt, &app, 33, 33);
    assert_eq!(cutout[3], 0, "rounded-corner cutout should be alpha 0");

    let center = read_pixel(&rt, &app, W / 2, H / 2);
    assert_eq!(center[3], 255, "rounded center should be alpha 255");
}

#[test]
fn circle_mask_corner_zero() {
    let (app, renderer) = boot();
    let rt = renderer.generate_mask_texture(&app, MaskShape::circle(Vec2::ZERO, 0.5), W, H);

    // (NDC -0.45, +0.45): distance from origin = sqrt(0.405) ≈ 0.636
    // > 0.5 → outside circle. Pixel (35, 35).
    let corner = read_pixel(&rt, &app, 35, 35);
    assert_eq!(corner[3], 0, "circle bounding-corner should be alpha 0");

    let center = read_pixel(&rt, &app, W / 2, H / 2);
    assert_eq!(center[3], 255, "circle center should be alpha 255");
}

#[test]
fn ellipse_mask_anisotropic_cutoff() {
    let (app, renderer) = boot();
    // Wide ellipse a=0.7, b=0.3.
    let rt = renderer.generate_mask_texture(
        &app,
        MaskShape::ellipse(Vec2::ZERO, Vec2::new(0.7, 0.3)),
        W,
        H,
    );

    let center = read_pixel(&rt, &app, W / 2, H / 2);
    assert_eq!(center[3], 255, "ellipse center alpha should be 255");

    // (NDC 0.6, 0.25) — inside [-0.7,0.7]×[-0.3,0.3] bounds, but
    // (0.6/0.7)^2 + (0.25/0.3)^2 = 1.43 > 1 → outside ellipse curve.
    // Pixel (102, 48).
    let outside_curve = read_pixel(&rt, &app, 102, 48);
    assert_eq!(
        outside_curve[3], 0,
        "ellipse inside-bounds-outside-curve should be alpha 0"
    );
}

#[test]
fn inverted_mask_swaps_inside_outside() {
    let (app, renderer) = boot();
    let region = Rect::new(-0.5, -0.5, 1.0, 1.0);
    let rt = renderer.generate_mask_texture_inverted(&app, MaskShape::rect(region), W, H);

    let inside = read_pixel(&rt, &app, W / 2, H / 2);
    assert_eq!(inside[3], 0, "inverted: rect center should be alpha 0");

    let outside = read_pixel(&rt, &app, 4, 4);
    assert_eq!(
        outside[3], 255,
        "inverted: rect outside should be alpha 255"
    );
}

#[test]
fn path_mask_star_center_full_alpha() {
    let (app, renderer) = boot();
    let star: Vec<Vec2> = (0i16..10)
        .map(|i| {
            let r = if i % 2 == 0 { 0.7 } else { 0.3 };
            let theta = f32::from(i) * std::f32::consts::PI / 5.0 - std::f32::consts::FRAC_PI_2;
            Vec2::new(theta.cos() * r, theta.sin() * r)
        })
        .collect();
    let rt = renderer.generate_path_mask_texture(&app, &star, W, H);

    let center = read_pixel(&rt, &app, W / 2, H / 2);
    assert_eq!(center[3], 255, "star center alpha should be 255");

    // Concave gap (NDC -0.65, +0.55) → pixel (22, 28). Outside star.
    let gap = read_pixel(&rt, &app, 22, 28);
    assert_eq!(gap[3], 0, "concave-gap alpha should be 0");
}
