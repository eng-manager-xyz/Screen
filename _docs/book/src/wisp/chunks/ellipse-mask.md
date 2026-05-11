# Ellipse mask

[Linear: AUT-34](https://linear.app/harwood/issue/AUT-34)

`MaskShape::Ellipse { center, half_extents }` adds anisotropic
elliptical cutouts. Unlike `Circle`, ellipse needs a real new SDF
since the rounded-rect formula doesn't degenerate to an ellipse with
unequal half-extents.

> **Note on `MaskShape::Circle`** ([Linear: AUT-30](https://linear.app/harwood/issue/AUT-30)) —
> a circle is just an ellipse with `half_extents.x == half_extents.y`,
> so it's available through this same family. Earlier work also
> exposed `MaskShape::Circle` directly as a degenerate `RoundedRect`
> (`half_extents = (r, r)`, `corner_radius = r`); both forms produce
> the same coverage texture. New code should prefer `Ellipse` or
> `RoundedRect` — they're the cleaner primitives.

```rust
use wisp::MaskShape;

let wide = MaskShape::ellipse(glam::Vec2::ZERO, glam::Vec2::new(0.85, 0.4));
let tall = MaskShape::ellipse(glam::Vec2::ZERO, glam::Vec2::new(0.4, 0.85));
```

![](../../assets/wisp/ellipse-mask.png)

## Architecture

A `shape_kind: f32` flag in the clip uniforms picks between the
rounded-rect SDF and the new ellipse SDF in `clip.wgsl`. Same
pipeline, same bind-group layout — one extra `if` in the WGSL plus
one new SDF helper.

```wgsl
fn sdf_ellipse(p: vec2<f32>, half: vec2<f32>) -> f32 {
    let s = p / max(half, vec2<f32>(1e-6));
    let inside = dot(s, s) - 1.0;
    return inside * min(half.x, half.y);
}
```

Why a pseudo-SDF: the closed-form ellipse SDF involves a quartic
root, expensive on every fragment. The scaled-quadratic
`(x/a)^2 + (y/b)^2 - 1` shares the same zero level set; multiplying
by `min(a, b)` puts the result in roughly NDC distance units so the
existing AA-band code (`smoothstep` over `aa = 2/min(w, h)`) still
produces a ~1-pixel-wide edge — visually indistinguishable from the
exact SDF for masking purposes.

All four mask primitives (`apply_clip` / `apply_privacy_blur` /
`apply_solid_redaction` / `apply_spotlight` / `apply_dim_outside_data`)
accept the new variant automatically — same pattern as
`MaskShape::Circle` from M-MASK.8.

## Done when

- [x] Ellipse renders in `wisp` (`MaskShape::Ellipse`).
- [x] Privacy / focus composition paths accept ellipse (proven by
  `clip_ellipse::ellipse_plugs_into_solid_redaction`).
- [x] Story `ellipse-mask` shows three variants (wide / tall / round).
- [x] Snapshot coverage updated.
- [x] Same primitive backs preview + headless export.
- [x] `just gate` green.

## API

[`wisp::MaskShape::Ellipse`](../../api/wisp/scene/clip/enum.MaskShape.html#variant.Ellipse)
