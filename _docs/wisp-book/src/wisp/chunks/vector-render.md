# Render vector primitives

[Linear: AUT-54](https://linear.app/harwood/issue/AUT-54)

`Vector::add_to_stage(&mut stage, parent)` (and the underlying
`to_graphics()` lower-level method) make a `Vector` primitive
renderable. Analytic shapes (rect / rounded-rect / circle / ellipse)
convert directly into a `Graphics` node that the existing graphics
pipeline draws.

```rust
use glam::Vec2;
use wisp::{Color, Fill, Transform, Vector, VectorShape, VectorStroke};
use wisp::math::Rect;

let root = stage.root();
let _id = Vector::new(VectorShape::ellipse(Vec2::ZERO, Vec2::new(1.0, 0.55)))
    .with_fill(Fill::Solid(Color::rgba_u8(120, 200, 130, 255)))
    .with_stroke(VectorStroke::new(0.06, Color::WHITE))
    .with_transform(Transform {
        position: Vec2::new(0.35, 0.0),
        scale: Vec2::splat(0.14),
        ..Transform::default()
    })
    .add_to_stage(&mut stage, root);
```

![](../../assets/wisp/vector-render.png)

## Architecture

This is a **thin layer** on top of the existing graphics rasterizer
— no new pipeline, no new shader. `Vector::to_graphics()` walks the
match arms and emits the matching `Graphics::draw_*` call:

| `VectorShape` | `Graphics` call |
|---|---|
| `Rect { rect }` | `draw_rect(rect)` |
| `RoundedRect { rect, radius }` | `draw_rounded_rect(rect, radius)` |
| `Circle { center, radius }` | `draw_ellipse(center, Vec2::splat(radius))` |
| `Ellipse { center, half_extents }` | `draw_ellipse(center, half_extents)` |
| `Path { points }` | `None` (deferred to M-VEC.10) |

`Fill` and `VectorStroke` route into the existing `Graphics::fill()`
and `Graphics::stroke()` setters. `Transform` lands on the produced
`Graphics::container.transform`.

`opacity` is folded into fill + stroke colors as an alpha multiplier
at conversion time. The renderer doesn't have a per-node opacity
channel today; this is the practical equivalent for V1 and matches
how PixiJS-style stacks have historically modeled "opacity on the
graphics primitive" (multiply into the paint).

## Path rendering — deferred

`VectorShape::Path` returns `None` from `to_graphics()` because the
existing graphics pipeline doesn't draw paths as visible geometry.
M-VEC.10 (AUT-62) lands `move_to / line_to / quadratic / cubic /
close / stroke / fill` path commands. Until then, paths can only
drive masks via the path-clip and path-mask-texture primitives.

## API

- [`wisp::Vector::to_graphics`](../../api/wisp/scene/vector/struct.Vector.html#method.to_graphics)
- [`wisp::Vector::add_to_stage`](../../api/wisp/scene/vector/struct.Vector.html#method.add_to_stage)
