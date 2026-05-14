New `Graphics::draw_polygon(vertices)` primitive — fills any convex
polygon listed CCW. Six shapes in this story (square, triangle,
pentagon, hexagon, trapezoid, octagon) exercise the fan-triangulated
triangle-list pipeline that runs alongside the existing SDF primitives
inside `GraphicsPipeline`.

Implementation: each polygon is fan-triangulated CPU-side from
vertex 0, world-matrix is baked into clip-space coordinates during
the scene walk, and the triangles flow through `graphics_polygon.wgsl`
— a separate WGSL with a triangle-list `VertexBufferLayout` per
vertex (`position`, `color`). The SDF instances still go through
`graphics_solid.wgsl`; both share the polygon node's blend-mode
bucket so they composite predictably.

```admonish warning title="Convex-only for v1"
Fan triangulation produces visible overlap for non-convex input.
For non-convex polygons (filled contour bands, complex SVG paths)
the path is a follow-up tessellator chunk that introduces
`lyon_tessellation` and a polygon SDF for edge AA. Today's
consumers are convex by construction (area fills, sankey ribbons,
funnel-area trapezoids, ternary outline) so the v1 scope ships
them without the extra dep.
```

```admonish note title="No edge AA in v1"
Triangle-list polygons render with hard pixel edges. Where crisp
edges matter, stroke the perimeter with `draw_line` calls — those
are already SDF-anti-aliased via the existing graphics pipeline.
```

For the chart layer this primitive unblocks:

* **Area chart** (AUT-190) — filled polygon below the line curve.
* **Sankey** (AUT-217) — ribbons after flattening cubic Beziers.
* **Funnel** (AUT-218) — both bands + area modes.
* **Contour filled** (AUT-209, partial) — convex bands today;
  full-non-convex contour deferred to the tessellator chunk.
* **Ternary** (AUT-210) — the simplex triangle outline.
