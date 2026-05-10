//! AUT-44 — dynamic mask texture cache.
//!
//! Five contracts:
//! 1. First call is a miss; second identical call is a hit (no
//!    regeneration).
//! 2. Different shapes produce different keys.
//! 3. Different dimensions produce different keys (even with the
//!    same shape).
//! 4. The inverted variant has a separate key from the non-inverted.
//! 5. `clear_mask_cache` drops every entry.

use pollster::block_on;
use wisp::MaskShape;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;

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

#[test]
fn second_call_with_same_inputs_hits_cache() {
    let (app, renderer) = boot();
    let shape = MaskShape::rect(Rect::new(-0.5, -0.5, 1.0, 1.0));

    let _first = renderer.cached_mask_texture(&app, shape, W, H);
    let (h0, m0) = renderer.mask_cache_stats();
    assert_eq!((h0, m0), (0, 1), "first call: 0 hits, 1 miss");

    let _second = renderer.cached_mask_texture(&app, shape, W, H);
    let (h1, m1) = renderer.mask_cache_stats();
    assert_eq!((h1, m1), (1, 1), "second call: 1 hit, still 1 miss");
}

#[test]
fn different_shapes_are_distinct_keys() {
    let (app, renderer) = boot();
    let rect = MaskShape::rect(Rect::new(-0.5, -0.5, 1.0, 1.0));
    let rounded = MaskShape::rounded_rect(Rect::new(-0.5, -0.5, 1.0, 1.0), 0.2);

    let _ = renderer.cached_mask_texture(&app, rect, W, H);
    let _ = renderer.cached_mask_texture(&app, rounded, W, H);
    let (hits, misses) = renderer.mask_cache_stats();
    assert_eq!((hits, misses), (0, 2), "two distinct shapes, two misses");
}

#[test]
fn different_dimensions_are_distinct_keys() {
    let (app, renderer) = boot();
    let shape = MaskShape::rect(Rect::new(-0.5, -0.5, 1.0, 1.0));

    let _ = renderer.cached_mask_texture(&app, shape, W, H);
    let _ = renderer.cached_mask_texture(&app, shape, W * 2, H * 2);
    let (hits, misses) = renderer.mask_cache_stats();
    assert_eq!((hits, misses), (0, 2), "different dims should miss");
}

#[test]
fn inverted_variant_uses_separate_key() {
    let (app, renderer) = boot();
    let shape = MaskShape::rect(Rect::new(-0.5, -0.5, 1.0, 1.0));

    let _ = renderer.cached_mask_texture(&app, shape, W, H);
    let _ = renderer.cached_mask_texture_inverted(&app, shape, W, H);
    let (hits, misses) = renderer.mask_cache_stats();
    assert_eq!(
        (hits, misses),
        (0, 2),
        "inverted should not collide with normal"
    );

    let _ = renderer.cached_mask_texture_inverted(&app, shape, W, H);
    let (hits2, misses2) = renderer.mask_cache_stats();
    assert_eq!(
        (hits2, misses2),
        (1, 2),
        "second inverted call hits its own key"
    );
}

#[test]
fn clear_mask_cache_drops_entries() {
    let (app, renderer) = boot();
    let shape = MaskShape::rect(Rect::new(-0.5, -0.5, 1.0, 1.0));

    let _ = renderer.cached_mask_texture(&app, shape, W, H);
    renderer.clear_mask_cache();

    // After clear, the next identical call is a miss again.
    let _ = renderer.cached_mask_texture(&app, shape, W, H);
    let (hits, misses) = renderer.mask_cache_stats();
    assert_eq!((hits, misses), (0, 2), "clear forces re-generation");
}
