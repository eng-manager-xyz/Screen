# Freehand path mask — M-MASK.10 / AUT-35

`Renderer::apply_path_clip(points, foreground, output)` and
`Renderer::apply_solid_redaction_path(points, color, base, output)`
are the freehand-shape primitives. Unlike the SDF shapes (rect /
rounded-rect / circle / ellipse), a path can't be expressed as a
closed-form distance function — so the WGSL fragment runs a classic
crossings-test point-in-polygon at every pixel, against a
uniform-buffered point list (up to 32 vertices for V1).

The story shows two compositions side-by-side over the same textured
frame (skin-tone gradient + grid):

- **Left** — alpha cutout via `apply_path_clip`. The textured frame
  shows through inside the star, transparent everywhere else.
- **Right** — solid redaction via `apply_solid_redaction_path`. The
  textured frame is the "base"; the star region is filled with
  near-black.

The polygon is a 10-vertex five-pointed star — a classic concave
shape that no SDF can represent. The crossings test handles concavity
correctly (winding-number parity). Self-intersecting paths aren't on
the freehand-mask UX path; this primitive doesn't promise sensible
results for them.

V1 is hard-edge (no AA). AA can be retrofitted later via a
distance-to-nearest-edge approximation in the same fragment shader.
