# Ellipse mask — M-MASK.9 / AUT-34

`MaskShape::Ellipse { center, half_extents }` adds anisotropic
elliptical cutouts. Unlike `Circle`, ellipse needs a real new SDF
since the rounded-rect formula doesn't degenerate to an ellipse with
unequal half-extents.

The story shows three variants over the same textured frame:

- **Left** — wide ellipse (a=0.85, b=0.4), letterbox-style.
- **Middle** — tall ellipse (a=0.4, b=0.85), avatar-style.
- **Right** — square (a=b=0.7), which is exactly the same circle the
  AUT-30 webcam-shapes story uses, just routed through the ellipse
  branch — lets us verify visual parity.

The SDF is a scaled-quadratic pseudo-distance:
`(x/a)^2 + (y/b)^2 - 1`, multiplied by `min(a, b)` to put it in
roughly NDC distance units so the existing AA-band math still
produces a ~1-pixel edge softness. Not Euclidean distance, but
visually indistinguishable for masking — and infinitely cheaper than
the closed-form ellipse SDF (which involves a quartic).

A `shape_kind: f32` flag in the clip uniforms picks between the
rounded-rect and ellipse branches. Same pipeline, same bind-group
layout — just one extra `if` in the WGSL.
