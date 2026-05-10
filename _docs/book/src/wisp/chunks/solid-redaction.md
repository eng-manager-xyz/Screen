# Solid redaction — M-MASK.5 / AUT-23

`Renderer::apply_solid_redaction(shape, color, base, output)` is the
*trust* counterpart to privacy blur. Instead of attenuating detail
inside a shape (blur), it replaces every pixel inside the shape with
an opaque `color`. Reuse of the same `MaskShape` enum means rect /
rounded-rect / circle / ellipse / freehand-path all work identically
to the blur primitive.

```rust
use wisp::{Color, MaskShape, math::Rect};

let region = Rect::new(-0.5, -0.3, 1.0, 0.5);
renderer.apply_solid_redaction(
    &app,
    MaskShape::rounded_rect(region, 0.12),
    Color::rgba_u8(20, 20, 20, 255),
    &base,
    &output,
);
```

![](../../assets/wisp/solid-redaction.png)

## Architecture

- **Same composition shape as privacy blur.** Three RTs (`fill`,
  `masked`, `output`); the only difference vs `apply_privacy_blur` is
  step 1 — instead of running `BlurFilter` over `base`, we clear a
  scratch RT to the redaction color via `LoadOp::Clear`. Keeps the
  pipeline cache small (one extra clear pass, no new shaders).
- **`Color → wgpu::Color` is a four-line `f64::from`.** Linear f32
  inputs map directly onto the clear-color floats. No gamma curve to
  worry about because the renderer's output format is `Rgba8Unorm` (or
  `Rgba8UnormSrgb` for display, in which case wgpu does the gamma
  itself).
- **Use opaque colors.** A non-1 alpha lets `base` show through, which
  defeats the trust use case. Tests pin the inside-region pixel to
  `(R, G, B, 255)` exactly.

## When to choose redaction over blur

Privacy blur is *polish* — text becomes unreadable but shape and
motion still hint through. Solid redaction is *trust* — no
information leaks through. A future inspector should communicate this
in copy: solid is the safe default for high-stakes content (API keys,
passwords, customer IDs); blur is the polished default for visual
privacy (faces, screen names, low-stakes URLs).

## Done when

- [x] Rectangle and rounded-rect both fully cover the selected region.
- [x] Pixel tests confirm exact-color fill inside, base bit-exact
  outside, and rounded corner carved away on `RoundedRect`.
- [x] Storybook story `solid-redaction` ships.
- [x] Output deterministic in story snapshots.
- [x] Same render primitive backs preview + headless export paths
  (single method on `Renderer`).
- [x] `just gate` green.

## API

[`wisp::Renderer::apply_solid_redaction`](../../api/wisp/render/struct.Renderer.html#method.apply_solid_redaction)
