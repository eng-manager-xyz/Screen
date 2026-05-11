# Clip + spotlight on vector masks

[Linear: AUT-58](https://linear.app/harwood/issue/AUT-58)

<table>
<tr>
<td valign="top" width="50%">

![](../../assets/wisp/clip-rounded.png)

*`apply_clip_vector` — rounded crop driven by a `Vector`.*

</td>
<td valign="top" width="50%">

![](../../assets/wisp/spotlight.png)

*`apply_spotlight_vector` — same vector data, spotlight composition.*

</td>
</tr>
</table>

Output is pixel-identical to the M-MASK.1 / M-MASK.6 equivalents —
the vector path lets you drive both off the same `Vector` value, and
adds freehand-polygon variants of each.

Closes the M-VEC.4..6 refactor zone. `apply_clip` (the rounded-crop
foundation from M-MASK.1) and `apply_spotlight` (M-MASK.6) now route
through the shared mask + compose path. Path-driven variants of both
land here.

```rust
use glam::Vec2;
use wisp::{Color, Vector, VectorShape};

// Path-driven crop (impossible before M-VEC.6):
let diamond = Vector::new(VectorShape::path(vec![
    Vec2::new( 0.0,  0.6),
    Vec2::new( 0.6,  0.0),
    Vec2::new( 0.0, -0.6),
    Vec2::new(-0.6,  0.0),
]));
renderer.apply_clip_vector(&app, &diamond, &foreground, &output);

// Path-driven spotlight (impossible before M-VEC.6):
renderer.apply_spotlight_vector(
    &app,
    &diamond,
    Color::rgba(0.0, 0.0, 0.0, 0.7),
    &base,
    &output,
);
```

## Architecture

- **`apply_clip` (and the new `apply_clip_vector`)** — uses
  `cached_vector_mask_texture` + `apply_mask_to_texture`. Output is
  byte-equivalent to the previous `ClipPipeline` path.
- **`apply_spotlight_vector`** — same pattern but with the
  *inverted* mask. Analytic shapes use
  `cached_mask_texture_inverted`; the path variant routes through
  `path_clip.apply` in `invert: true` mode (the cached path-mask
  doesn't have an inverted form yet — straightforward future
  extension).

## Auto-dispatch path NOT refactored

`render_stage` auto-dispatches `Container::clip = Some(MaskShape)`
through the inline `ClipPipeline` directly. That path is *hot* —
called per dispatched node every frame — and the existing single-
shader implementation already optimizes it. The vector-mask refactor
adds an extra render pass per call which is fine for explicit
primitives (offset by mask cache hits) but would be a regression on
the hot path. The auto-dispatch keeps using the inline `clip`
pipeline; explicit `apply_clip` calls go through the new path.

## Done when

- [x] `apply_clip(MaskShape, ...)` byte-equivalent (4 M-MASK.1 +
  related tests pass unchanged).
- [x] `apply_spotlight(MaskShape, ...)` byte-equivalent (3 M-MASK.6
  tests pass unchanged).
- [x] `apply_clip_vector(vector, ...)` accepts paths.
- [x] `apply_spotlight_vector(vector, ...)` accepts paths.
- [x] New tests cover path-driven clip and spotlight.
- [x] Auto-dispatch path documented as deliberately unchanged.
- [x] `just gate` green.

## API

- [`wisp::Renderer::apply_clip`](../../api/wisp/render/struct.Renderer.html#method.apply_clip)
- [`wisp::Renderer::apply_clip_vector`](../../api/wisp/render/struct.Renderer.html#method.apply_clip_vector)
- [`wisp::Renderer::apply_spotlight`](../../api/wisp/render/struct.Renderer.html#method.apply_spotlight)
- [`wisp::Renderer::apply_spotlight_vector`](../../api/wisp/render/struct.Renderer.html#method.apply_spotlight_vector)
