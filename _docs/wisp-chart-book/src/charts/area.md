# Area chart

A line chart whose region between the curve and the baseline is
filled. Use for magnitude-over-time visualisations where the
area under the curve carries meaning.

## Demo — NASA's share of the US federal budget, 1962–1972

The demo plots **NASA spending as a percentage of the US federal
budget** through the Apollo era. The 1966 peak (~4.4 %) and the
sharp post-Apollo wind-down to <2 % by 1972 tell the rise-and-fall
of the moon program in one shape. Reveal eases in over 1.5 s
with `Ease::OutCubic`.

<div style="position: relative; aspect-ratio: 480 / 320; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/area.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-area" src="../demo/?chart=area&animate=reveal" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: NASA budget share 1962–1972"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <button type="button" onclick="(function(){var f=document.getElementById('demo-area');f.src=f.src.split('&_t=')[0]+'&_t='+Date.now();})()" style="padding: 0.35rem 0.8rem; border: 1px solid #888; background: #fff; cursor: pointer; font: inherit;">↻ Replay animation</button>
  <a href="https://en.wikipedia.org/wiki/Budget_of_NASA" target="_blank" rel="noopener" style="margin-left: 0.75rem;">Source: Budget of NASA — Wikipedia</a>
</p>

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
