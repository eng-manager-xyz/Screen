# Density (KDE)

Kernel-density estimate over a 1D sample. Smooths the discrete
[histogram](./histogram.md) into a continuous density curve.
Defaults to a Gaussian kernel + Silverman's rule of thumb for the
bandwidth.

<div style="position: relative; aspect-ratio: 3 / 2; max-width: 540px; margin: 1rem 0; background: url('../assets/wisp-chart-web/kde.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=kde" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: KDE"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::distributions::{BandwidthRule, KdePlot};

let kde = KdePlot::new(samples)
    .bandwidth(BandwidthRule::Silverman);    // or Manual(0.5)
let g = kde.emit_graphics(&theme, Vec2::new(360.0, 240.0));
```

```admonish info title="Bandwidth rules"
- `BandwidthRule::Silverman` — `1.06·σ·n^(-1/5)`. Robust default
  for unimodal data. Mildly over-smooths bimodal distributions.
- `BandwidthRule::Manual(h)` — pick your own. Useful for
  reproducing a published figure with a known bandwidth.
```

```admonish tip title="Faceted density"
Render one KDE per category by repeating this fixture inside the
[trellis / small-multiples](./trellis.md) layout. Same value
type, different containing layout — the
[faceted-density](./faceted-density.md) chapter walks through it.
```
