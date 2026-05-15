# Faceted density

A composition pattern, not a separate value type. Render one
[KDE](./kde.md) per facet (category, time window, group) inside
the [trellis](./trellis.md) small-multiples layout — same data
shape, repeated per facet.

<div style="position: relative; aspect-ratio: 3 / 2; max-width: 540px; margin: 1rem 0; background: url('../assets/wisp-chart-web/kde.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=kde" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: faceted KDE (single facet)"></iframe>
</div>

## Pattern

```rust,ignore
use wisp_chart::distributions::KdePlot;
use wisp_chart::multi::Trellis;

let facets: Vec<KdePlot> = groups
    .iter()
    .map(|group| KdePlot::new(group.samples.clone()))
    .collect();

let trellis = Trellis::new(facets, /* cols */ 3, /* gap_px */ 16.0);
let g = trellis.emit_graphics(&theme, viewport);
```

```admonish info title="Why faceted instead of overlaid"
Overlaying many KDEs on one axis is legible up to ~5 series.
Past that, the eye loses which curve is which even with
distinct colours. Faceting trades axis-comparison ease for
"each facet reads cleanly" — the right choice for >5 groups.
```

```admonish tip title="Picking facet count"
If `n` series fit in `cols × ⌈n/cols⌉` facets with `cols ≈ √n`,
the aspect ratio of the grid stays close to 1:1. The
[trellis](./trellis.md) chapter has the full layout recipe.
```
