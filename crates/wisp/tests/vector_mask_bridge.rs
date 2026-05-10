//! AUT-55 — render vectors to alpha-mask textures.
//!
//! `Renderer::generate_vector_mask_texture` and the cached
//! companion route to either the SDF or path mask pipeline based on
//! the `VectorShape` variant. The output should match
//! `generate_mask_texture` (analytic) or `generate_path_mask_texture`
//! (path) byte-for-byte — the bridge adds zero pixel-level
//! difference.
//!
//! Four contracts:
//! 1. Analytic Vector → mask matches direct `generate_mask_texture`
//!    on the equivalent `MaskShape`.
//! 2. Path Vector → mask matches direct `generate_path_mask_texture`
//!    on the same point list.
//! 3. Cached analytic Vector hits the cache on the second call.
//! 4. Cached path Vector does NOT hit the cache (V1 limitation —
//!    paths bypass).

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{MaskShape, RenderTexture, Vector, VectorShape};

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
fn analytic_vector_mask_matches_direct_mask_shape() {
    let (app, renderer) = boot();
    let rect = Rect::new(-0.4, -0.4, 0.8, 0.8);
    let radius = 0.15;

    let direct = renderer.generate_mask_texture(&app, MaskShape::rounded_rect(rect, radius), W, H);
    let bridged = renderer.generate_vector_mask_texture(
        &app,
        &Vector::new(VectorShape::rounded_rect(rect, radius)),
        W,
        H,
    );

    // Sample three positions: center inside, far corner outside,
    // a known-edge pixel.
    for (x, y) in [(W / 2, H / 2), (2, 2), (10, 10)] {
        assert_eq!(
            read_alpha(&direct, &app, x, y),
            read_alpha(&bridged, &app, x, y),
            "alpha differs at ({x}, {y})"
        );
    }
}

#[test]
fn path_vector_mask_matches_direct_path_call() {
    let (app, renderer) = boot();
    let star: Vec<Vec2> = (0i16..10)
        .map(|i| {
            let r = if i % 2 == 0 { 0.7 } else { 0.3 };
            let theta = f32::from(i) * std::f32::consts::PI / 5.0 - std::f32::consts::FRAC_PI_2;
            Vec2::new(theta.cos() * r, theta.sin() * r)
        })
        .collect();

    let direct = renderer.generate_path_mask_texture(&app, &star, W, H);
    let bridged = renderer.generate_vector_mask_texture(
        &app,
        &Vector::new(VectorShape::path(star.clone())),
        W,
        H,
    );

    for (x, y) in [(W / 2, H / 2), (2, 2), (10, 50)] {
        assert_eq!(
            read_alpha(&direct, &app, x, y),
            read_alpha(&bridged, &app, x, y),
            "alpha differs at ({x}, {y})"
        );
    }
}

#[test]
fn cached_analytic_vector_hits_on_second_call() {
    let (app, renderer) = boot();
    let v = Vector::new(VectorShape::circle(Vec2::ZERO, 0.4));

    let _first = renderer.cached_vector_mask_texture(&app, &v, W, H);
    let (h0, m0) = renderer.mask_cache_stats();
    assert_eq!((h0, m0), (0, 1), "first analytic call: 1 miss");

    let _second = renderer.cached_vector_mask_texture(&app, &v, W, H);
    let (h1, m1) = renderer.mask_cache_stats();
    assert_eq!((h1, m1), (1, 1), "second analytic call: 1 hit");
}

#[test]
fn cached_path_vector_does_not_use_cache() {
    let (app, renderer) = boot();
    let v = Vector::new(VectorShape::path(vec![
        Vec2::new(-0.5, -0.5),
        Vec2::new(0.5, -0.5),
        Vec2::new(0.0, 0.5),
    ]));

    let _first = renderer.cached_vector_mask_texture(&app, &v, W, H);
    let _second = renderer.cached_vector_mask_texture(&app, &v, W, H);
    let (hits, misses) = renderer.mask_cache_stats();
    // Path bypasses the cache — neither call increments hits/misses.
    assert_eq!((hits, misses), (0, 0), "path bypasses the SDF cache");
}
