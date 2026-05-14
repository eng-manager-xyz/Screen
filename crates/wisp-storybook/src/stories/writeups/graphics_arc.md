Two new SDF primitives in `Graphics`:

* `draw_annular_sector(center, r_inner, r_outer, start, end)` — filled
  pie slice (`r_inner = 0`) or donut slice (`r_inner > 0`). Angles in
  radians, `0` aligned with `+x`, CCW positive.
* `draw_arc(center, radius, start, end, stroke_width)` — thin stroked
  curve; lowers to an annular sector with
  `r_inner = radius - stroke_width / 2`, `r_outer = radius + stroke_width / 2`.

The SDF unifies pie slices, donuts, partial annular sectors, and stroked
arcs in one fragment-shader branch. Implementation is in
`graphics_solid.wgsl::sdf_annular_sector`: rotate the local point so the
wedge centerline aligns with `+y`, mirror across `+y` to halve the
geometry, then return `r - r_outer` (disc case) or
`max(r - r_outer, r_inner - r)` (annulus case) for points inside the
angular wedge, and the distance to the radial wedge edge otherwise.

For the chart layer this is the enabler under:

* **Pie / donut** charts (M-CHART.17 / AUT-197) — one annular sector per
  slice.
* **Sunburst** (M-CHART.36 / AUT-216) — nested annular sectors with
  parent angular ranges.
* **Gauge** (M-CHART.15 / AUT-195) — coloured threshold-zone arcs plus a
  needle.
