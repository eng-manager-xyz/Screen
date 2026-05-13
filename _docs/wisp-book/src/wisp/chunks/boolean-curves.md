# Boolean ops on curved paths — flatten tolerance

The boolean engine ([previous chapter](./path-boolean.md)) takes
`Path` inputs but operates on **polygons** internally — `combine()`
flattens every `QuadTo` / `CubicTo` to a polyline before clipping.
This chapter covers the trade-off you control through
`BoolOptions::flatten_tolerance`.

## Why flatten at all

Path-boolean algorithms (Vatti, Greiner-Hormann, Bentley-Ottmann)
are formulated over straight-line segments. Curves don't have a
closed-form intersection algorithm that's efficient for general
boolean ops — every robust implementation flattens first.

The cost is fidelity: the output is a polygon. Re-fitting curves
to the result is a separate problem (out of scope; future work
catalogued in [M-BOOL.18](https://linear.app/harwood/issue/AUT-179)).

## Default tolerance: 0.5 device pixels

```admonish info title="Reading the default"
`BoolOptions::default().flatten_tolerance == 0.005` in **NDC units**
(`[-1, 1]`). At a 1920×1080 viewport, NDC `0.005` ≈ `1080 * 0.005 / 2`
≈ **2.7 pixels** in screen space along the long axis. That's the
"smooth-at-arm's-length" tolerance — visible polygonization only
appears under a zoom or on retina displays at 100% scale.
```

The default trades a few extra edges per curve for output that
looks indistinguishable from a true curve at typical viewing sizes.

## The trade-off table

| Tolerance | Edges/circle (radius ≈ 0.4 NDC) | Use when |
|---|---|---|
| `0.001` | ~80–100 | Print export, retina screenshots, geometry under heavy zoom |
| `0.005` (default) | ~24–32 | Live preview, 1080p compositions, storybook captures |
| `0.05` | ~10–12 | Low-res thumbnails, motion-blur sources (re-blurred anyway) |
| `0.1` | ~6–8 | Visibly polygonal — only when polygonization is the look |

(Edge counts measured against the in-tree `circle()` test helper
at the listed tolerances; see
`crates/wisp/src/scene/path/boolean.rs::flatten_tolerance_controls_output_resolution`.)

```admonish warning title="Tolerance is per-flattening, not per-result"
The `flatten_tolerance` controls how finely **input** curves get
chopped before clipping. The clip output may carry **fewer** edges
than the flattened input (collinear segments get merged) — so the
final polygon vertex count is "inputs flattened at tolerance T,
then simplified by the clip." Halving `tolerance` does not
necessarily double output edges.
```

## What's an acceptable tolerance for your use case

The shorthand: **set tolerance to the smallest pixel size you want
to be invisible**.

- Storybook captures at 1024×1024 → ~half a pixel ≈ NDC `0.001`,
  but `0.005` is usually fine because the eye doesn't resolve
  sub-pixel curvature.
- Mask textures for vector clipping → match the mask resolution.
  A 512×512 mask doesn't benefit from sub-NDC-`0.005` precision;
  the mask quantises to its own grid first.
- Export-quality booleans (PDF, SVG-out, print) → `0.001` or
  tighter; the consumer may re-stroke and zoom.

## Gotchas

1. **Collinear input edges** (rounded rect straight sides meeting
   another straight side at the same y) trip the engine's
   coincident-edge handling and can split a single union into
   multiple subpaths. **Workaround:** offset the inputs so their
   straight edges don't overlap, or compose with circles / pure
   curves where edges are never collinear.
2. **Self-intersecting input** after flattening → undefined output
   (same as v1). Keep input convex or pre-validate.
3. **`f32` precision** — at NDC `< 1e-6`, two segments that should
   be coincident may drift by less than `f32::EPSILON` and the
   engine can either merge or split them. Don't push tolerance
   below `0.0001`.

## What v1 ships

- All four ops (`Union`, `Intersection`, `Difference`, `Xor`) on
  Bezier-containing inputs.
- Per-subpath flattening — multi-subpath `Path` inputs preserve
  their disjoint regions through the op.
- Three regression tests in `boolean.rs`:
  - `curve_input_union_produces_curved_outline` — two overlapping
    circles produce one contour with materially more edges than a
    square baseline.
  - `curve_input_difference_carves_circle_out_of_circle` — crescent
    shape from `A − B` keeps `A`'s far side and carves `B`'s centre.
  - `flatten_tolerance_controls_output_resolution` — tighter
    tolerance produces strictly more edges than a looser one.

The public `Path::flatten_subpaths(tolerance) -> Vec<Vec<Vec2>>`
helper exposes the same per-subpath flattening the engine does
internally, for consumers that need to inspect the polyline form
without running a full boolean op.

## Deferred

- Curve re-fitting on the output ([M-BOOL.18 / AUT-179](https://linear.app/harwood/issue/AUT-179)).
- Tolerance-aware caching ([M-BOOL.14 / AUT-175](https://linear.app/harwood/issue/AUT-175)).
- Curve-aware fill rules ([M-BOOL.8 / AUT-169](https://linear.app/harwood/issue/AUT-169)).
