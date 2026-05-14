# Area chart

A line chart whose region between the curve and the baseline is
filled. Use for magnitude-over-time visualisations where the
area under the curve carries meaning.

## Public surface

```rust,ignore
let plot = Plot::new(daily)
    .mark(Mark::Area { interpolation: Interpolation::Linear })
    .encode(plot::x("date", ScaleKind::Band))
    .encode(plot::y("value", ScaleKind::Linear))
    .encode(plot::color("region"));
```

## Convex-quad-per-segment emission

```admonish info
wisp's `draw_polygon` is convex-only in v1 — it fan-triangulates
from vertex 0, which produces overlapping triangles when the
input polygon is non-convex. A typical area chart polygon
(line + baseline) is non-convex whenever the line bends.

The renderer sidesteps this by emitting one **convex
quadrilateral per segment**: `(x0, baseline) → (x1, baseline)
→ (x1, y1) → (x0, y0)`. Each quad is always convex, so the fan
triangulation is correct. Visually identical to one big polygon;
costs one extra primitive per segment.
```

## Interpolation modes

| Mode      | Quad shape                                         |
|-----------|----------------------------------------------------|
| `Linear`  | Slanted top edge connecting `(x0, y0) → (x1, y1)`  |
| `Step`    | Flat top edge at `y0` from `x0` to `x1` (step bar) |

## Pairing with Color encoding

A `Color` encoding splits rows into one series per category and
emits one polygon stream per series. Without an explicit
back-to-front render order today, overlapping areas can occlude
each other; future tickets will add z-ordering by series area.
