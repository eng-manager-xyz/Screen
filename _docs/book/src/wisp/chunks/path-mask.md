# Freehand path mask — M-MASK.10 / AUT-35

`Renderer::apply_path_clip(points, foreground, output)` and
`Renderer::apply_solid_redaction_path(points, color, base, output)`
add freehand-shape masking. Unlike the SDF shapes, paths are
expressed as raw point lists; the WGSL runs a classic
crossings-test point-in-polygon at every pixel.

```rust
use glam::Vec2;
use wisp::Color;

let star: Vec<Vec2> = (0i16..10).map(|i| {
    let r = if i % 2 == 0 { 0.85 } else { 0.35 };
    let theta = f32::from(i) * std::f32::consts::PI / 5.0
        - std::f32::consts::FRAC_PI_2;
    Vec2::new(theta.cos() * r, theta.sin() * r)
}).collect();

renderer.apply_path_clip(&app, &star, &foreground, &output);
renderer.apply_solid_redaction_path(
    &app,
    &star,
    Color::rgba_u8(20, 20, 20, 255),
    &base,
    &output,
);
```

![](../../assets/wisp/path-mask.png)

## Architecture

A new `PathClipPipeline` lives alongside the SDF-based
`ClipPipeline`. The WGSL fragment shader (`path_clip.wgsl`):

- Accepts up to 32 polygon vertices via a uniform buffer
  (`array<vec4<f32>, 32>` for alignment; only `.xy` used).
- Runs the classic crossings test (Jordan curve theorem) — for each
  edge `(a, b)` of the closed polygon, tally crossings of the
  horizontal ray going `+x` from the fragment point. Odd parity =
  inside.
- Multiplies the foreground sample's alpha by the inside test.

V1 is hard-edge (no AA). The rasterized output is integer-pixel
accurate; AA can be retrofitted later via a `distance-to-nearest-edge`
approximation in the same fragment shader.

The 32-point cap is a uniform-buffer size limit. Above that, the
storage-buffer route (or polygon-segment-batched render passes)
unlocks larger paths.

## Why this isn't a `MaskShape::Path` variant

`MaskShape` is `Copy` — every variant holds POD data so the enum
stays cheap to pass by value through the auto-dispatch path. A path
needs an owned `Vec<Vec2>` (or `Arc<[Vec2]>`) to store the points,
which would force `MaskShape` to drop `Copy` and adopt `Clone`. The
ripple to existing call sites isn't worth it for a premium-shape
expansion. Path-clip lives next to the SDF clip, accessed via its
own dedicated public methods.

## Done when

- [x] Freeform/path region renders in `wisp` (`apply_path_clip`).
- [x] Privacy / focus composition paths can use the path mask
  (`apply_solid_redaction_path` is the reference example; the same
  pattern wraps blur / spotlight when needed).
- [x] Story `path-mask` shows alpha-cutout and solid-redaction
  variants of a five-pointed star.
- [x] Snapshot coverage updated.
- [x] Data model is reusable — `&[Vec2]` polygon, ready for any future
  drawing tool to feed in.
- [x] `just gate` green.

## API

- [`wisp::Renderer::apply_path_clip`](../../api/wisp/render/struct.Renderer.html#method.apply_path_clip)
- [`wisp::Renderer::apply_solid_redaction_path`](../../api/wisp/render/struct.Renderer.html#method.apply_solid_redaction_path)
