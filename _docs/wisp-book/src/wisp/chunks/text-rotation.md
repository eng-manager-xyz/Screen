# Text rotation under Container transform — M-TEXT.20

`wisp::Text` glyphs are placed by a world matrix that's the
composition of every `Container::transform` between the text node
and the stage root. That includes the text's own
`text.container.transform.rotation`. The propagation is automatic
— no separate rotation field on `Text`, no glyphon-side bypass.

```admonish important title="Why this matters for charts"
Y-axis titles need to read bottom-to-top, which is `Text` rotated
-π/2 (90° clockwise on a `+Y`-up renderer). Radar / polar axis
labels follow the same path. Without rotation propagation, every
chart family that needs vertical text would need a wisp-side
enabler. With it, the chart layer just sets
`text.container.transform.rotation` and renders.
```

## Convention

- Rotation is **counter-clockwise** in radians: `0.0` = unrotated;
  `+π/2` = quarter turn CCW; `-π/2` = quarter turn CW (the
  Y-axis-label rotation).
- The rotation **pivot is the text's local origin** by default —
  the same point `transform.position` places in world space. To
  rotate around a different anchor (e.g. the centre of the
  string), set `transform.pivot` accordingly.
- Rotation composes with `transform.scale` and parent containers
  through `Mat3` multiplication in
  [`scene::transform::compose`](../../api/wisp/scene/transform/fn.compose.html).

## Verified by

`crates/wisp/tests/text_rotation.rs` — three assertions:

1. Unrotated 5-char text produces a wide bounding box (`width > height`).
2. The same text with `rotation = -π/2` produces a tall bounding
   box (`height > width`).
3. The aspect-ratio flip is large enough to be unambiguous
   (`h_aspect > 1.5`, `v_aspect < 0.67`).

These tests double as anti-regression guards: any future change
to the text pipeline that drops the `world` matrix multiplication
on glyph instances will fail them.

## Example

```rust
use wisp::{Color, Font, Stage, Text};

let font = Font::bitmap_8x8(&app);
let mut title = Text::new(font, "Revenue ($M)").with_cell_size(0.04);
title.color = Color::rgba_u8(34, 34, 34, 255);
// Position next to the Y-axis tick labels.
title.container.transform.position = glam::Vec2::new(-0.9, 0.0);
// Rotate -90° so the text reads bottom-to-top.
title.container.transform.rotation = -std::f32::consts::FRAC_PI_2;

stage.add_child(stage.root(), title);
```

---

[`Text` API](../../api/wisp/scene/struct.Text.html) · [`Container`](../../api/wisp/scene/struct.Container.html) · [`Transform`](../../api/wisp/scene/struct.Transform.html)
