# Composition primitives with explicit masks — M-DYN.3..6

The high-level mask primitives (`apply_privacy_blur`,
`apply_solid_redaction`, `apply_spotlight`, `apply_clip*`) generate
the mask texture internally. Sometimes you want the mask externally:
share one alpha texture across multiple effects in the same frame
without regenerating it three times.

These four explicit-mask companion primitives accept the mask as a
parameter:

| Primitive | What's inside | What's outside | Issue |
|---|---|---|---|
| `compose_blur_through_mask(base, radius, mask, output)` | blurred base | base unchanged | M-DYN.3 / AUT-45 |
| `compose_solid_through_mask(base, color, mask, output)` | solid `color` | base unchanged | M-DYN.4 / AUT-46 |
| `compose_dim_through_inverted_mask(base, dim_color, inverted_mask, output)` | base unchanged (mask=0) | `dim_color` over base (mask=1) | M-DYN.5 / AUT-47 |
| `apply_clip_vector(vector, fg, output)` with `Circle` / `RoundedRect` | webcam frame | transparent | M-DYN.6 / AUT-48 |

```rust
use wisp::{MaskShape, math::Rect};

let region = Rect::new(-0.4, -0.4, 0.8, 0.8);
let shape = MaskShape::rounded_rect(region, 0.15);

// Generate the mask once.
let mask = renderer.generate_mask_texture(&app, shape, w, h);

// Use it for blur, redaction, and spotlight on the same frame.
renderer.compose_blur_through_mask(&app, &base, 12.0, &mask, &out_blur);
renderer.compose_solid_through_mask(
    &app, &base, Color::rgba_u8(20, 20, 20, 255), &mask, &out_redact,
);
let inverted = renderer.generate_mask_texture_inverted(&app, shape, w, h);
renderer.compose_dim_through_inverted_mask(
    &app, &base, Color::rgba(0.0, 0.0, 0.0, 0.7), &inverted, &out_spot,
);
```

## Architecture

These primitives are *the* explicit-mask versions of the high-level
methods. The high-level methods now route through them:

```text
apply_privacy_blur(MaskShape, ...)
   └─ wraps in Vector
   └─ apply_privacy_blur_vector(Vector, ...)
      └─ cached_vector_mask_texture(Vector) → mask
      └─ compose_blur_through_mask(base, radius, mask, output)
```

Same pattern for redaction and spotlight. The lower-level
primitives are public so callers that already have a mask texture
(maybe shared across effects, maybe loaded from a file, maybe from
a custom shader) can use them directly.

## M-DYN.6 — webcam crops are already there

`apply_clip_vector(vector, foreground, output)` already accepts
`VectorShape::Circle` and `VectorShape::RoundedRect`. M-DYN.6 spec
says webcam overlays should crop through the dynamic mask path —
that's exactly what `apply_clip_vector` does (mask generated via
`MaskTexturePipeline`, composed via `MaskComposePipeline`). No new
primitive needed; the chapter exists to make the connection
explicit. See [M-VEC.6 chapter](./vector-clip-spotlight.md) for
details.

## Tests

- `crates/wisp/tests/blur_mask_reuse.rs` (M-DYN.3) — explicit-mask
  blur matches the high-level path; one mask shared across blur and
  `apply_mask_to_texture` produces correct output.
- `crates/wisp/tests/compose_through_mask.rs` (M-DYN.4 + .5) —
  explicit-mask redaction and spotlight both match their high-level
  counterparts byte-equivalent.

## Done when

- [x] All four primitives (`compose_blur_through_mask`,
  `compose_solid_through_mask`,
  `compose_dim_through_inverted_mask`, plus `apply_clip_vector` for
  webcam crops) are public and documented.
- [x] High-level `apply_*` methods route through them internally.
- [x] Tests prove byte-equivalence with the high-level paths.
- [x] `just gate` green.

## API

- [`wisp::Renderer::compose_blur_through_mask`](../../api/wisp/render/struct.Renderer.html#method.compose_blur_through_mask)
- [`wisp::Renderer::compose_solid_through_mask`](../../api/wisp/render/struct.Renderer.html#method.compose_solid_through_mask)
- [`wisp::Renderer::compose_dim_through_inverted_mask`](../../api/wisp/render/struct.Renderer.html#method.compose_dim_through_inverted_mask)
- [`wisp::Renderer::apply_clip_vector`](../../api/wisp/render/struct.Renderer.html#method.apply_clip_vector)
