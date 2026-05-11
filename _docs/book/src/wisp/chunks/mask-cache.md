# Mask texture cache

[Linear: AUT-44](https://linear.app/harwood/issue/AUT-44)

![](../../assets/wisp/mask-texture.png)

*The textures the cache stores — alpha RTs from M-DYN.1's
`MaskTexturePipeline`. Static scenes (a fixed-position privacy
crop, a stable webcam overlay) regenerate these once and reuse the
cached `Arc<RenderTexture>` across every frame.*

`Renderer::cached_mask_texture(shape, w, h)` and the inverted
companion return `Arc<RenderTexture>`s memoized on `(shape data,
dimensions, invert flag)`. Identical inputs across frames return the
same GPU texture instead of regenerating.

```rust
use wisp::{MaskShape, math::Rect};

// First call: GPU work. Subsequent calls with the same args: O(1).
let mask = renderer.cached_mask_texture(
    &app,
    MaskShape::rounded_rect(Rect::new(-0.5, -0.5, 1.0, 1.0), 0.2),
    256,
    256,
);

// Cache observability.
let (hits, misses) = renderer.mask_cache_stats();
println!("mask cache: {hits} hits, {misses} misses");
```

The cache backs M-VEC.4..6's vector-driven mask refactor: when a
`PrivacyBlur` / `DimOutside` / vector mask is re-evaluated each
frame, identical static regions produce the same key and reuse the
existing texture.

## Architecture

- **Keying.** `MaskKey` bit-casts every `f32` field in the shape
  (rect coords, radius, ellipse half-extents) to `u32`, then hashes
  the resulting `[u32; N]` representation. Exact-bit equality means
  a re-emitted identical shape value Just Works; NaN is handled
  consistently (canonical NaN bits are equal to themselves).
- **Eviction.** FIFO at `MAX_ENTRIES = 64`. Backed by a `HashMap` +
  `VecDeque`; on overflow the oldest insertion is dropped. The cap
  bounds GPU memory at ~16 MB worst case (64 × 256² × 4 bytes), well
  under the budget on integrated GPUs.
- **Sharing.** Returns `Arc<RenderTexture>` so the cache and the
  caller can both hold references. Drop the `Arc` and the cache may
  still hold the texture; clear the cache and any outstanding `Arc`
  still owns its data until the last reference goes.
- **Path masks not cached in V1.** Hashing `Vec<glam::Vec2>` is
  non-trivial and freehand polygons typically mutate between frames.
  Use `generate_path_mask_texture` directly; manage caching at the
  call site if needed.

## API

- [`wisp::Renderer::cached_mask_texture`](../../api/wisp/render/struct.Renderer.html#method.cached_mask_texture)
- [`wisp::Renderer::cached_mask_texture_inverted`](../../api/wisp/render/struct.Renderer.html#method.cached_mask_texture_inverted)
- [`wisp::Renderer::mask_cache_stats`](../../api/wisp/render/struct.Renderer.html#method.mask_cache_stats)
- [`wisp::Renderer::clear_mask_cache`](../../api/wisp/render/struct.Renderer.html#method.clear_mask_cache)
