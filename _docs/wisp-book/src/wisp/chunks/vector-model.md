# Vector shape model

[Linear: AUT-53](https://linear.app/harwood/issue/AUT-53)

![](../../assets/wisp/vector-render.png)

*Sample `Vector`s rendered as a contact sheet — rounded rect,
circle, ellipse, freehand path, line stroke. Each tile is the output
of `Renderer::render_stage` after building a one-node `Vector`
scene.* The shape data shown here is exactly what `VectorShape` +
`Vector` carry — the rest of this chapter is the type surface that
produces it.

`VectorShape` and `Vector` are Wisp's shared shape language. Every
visual tool — masks, crops, highlights, callouts, cursor effects,
later SVG import — drives off the same data so each tool doesn't
invent its own geometry model.

This is **not** SVG support. It is Wisp's own deterministic primitive
set, shaped to ship under our budget. M-VEC.13 may add a small SVG
*subset* import (rect, circle, ellipse, path); full SVG (CSS cascade,
animations, filters, external resources) is explicitly deferred —
see AUT-71/72/73 guardrails.

```rust
use glam::Vec2;
use wisp::{Color, Fill, Vector, VectorShape, VectorStroke, math::Rect};

let circle = Vector::new(VectorShape::circle(Vec2::ZERO, 0.4))
    .with_fill(Fill::Solid(Color::rgba(1.0, 0.5, 0.0, 1.0)))
    .with_stroke(VectorStroke::new(0.02, Color::WHITE))
    .with_opacity(0.9);
```

## Shape catalog

`VectorShape` is non-exhaustive. Initial variants:

| Variant | Notes |
|---|---|
| `Rect { rect }` | Sharp-corner axis-aligned. |
| `RoundedRect { rect, radius }` | Corner radius in NDC. |
| `Circle { center, radius }` | Square bounding box. |
| `Ellipse { center, half_extents }` | Anisotropic. |
| `Path { points: Vec<Vec2> }` | Closed polygon, up to 32 vertices. |

Future shape variants (M-VEC.10 path stroke commands, M-VEC.13 SVG
import, M-VEC.16 feathered) will extend the enum without breaking
callers (`#[non_exhaustive]`).

## Compatibility with `MaskShape`

`MaskShape` (the analytic SDF subset shipped during M-MASK) and
`VectorShape` overlap in their non-path variants. Conversion is
explicit:

```rust
let v = VectorShape::rounded_rect(rect, 0.2);
let mask: Option<MaskShape> = v.as_mask_shape();
// Some(MaskShape::RoundedRect { rect, radius: 0.2 })
```

This is what M-VEC.4..6 will use to refactor existing mask primitives
onto the vector model without rewriting the SDF shader.

For paths: `VectorShape::Path` carries an owned `Vec<Vec2>` so
`VectorShape` is `Clone`, not `Copy`. That's the same reason
`MaskShape::Path` was never added — see M-MASK.10's chapter.
`as_path_points()` exposes the slice for the path-mask machinery.

## API

- [`wisp::VectorShape`](../../api/wisp/scene/vector/enum.VectorShape.html)
- [`wisp::Vector`](../../api/wisp/scene/vector/struct.Vector.html)
- [`wisp::VectorStroke`](../../api/wisp/scene/vector/struct.VectorStroke.html)
