# Ternary plot

Plot 3-component compositional data on an equilateral triangle.
Each point's position uniquely encodes all three component
ratios; constructors normalise so the components sum to 1. The
classic use case is soil composition (sand / silt / clay) but
ternary diagrams turn up everywhere portfolios sum to a constant.

The demo plots **eight reference points from the USDA
soil-texture triangle** — sand at the bottom-left vertex, silt
at bottom-right, clay at the top. The points span the canonical
texture classes (sand, sandy loam, **loam** at the visual
centre, silty clay loam, clay…) — the same classification
agronomists have used since the *Soil Survey Manual* published
the diagram in 1951.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 420px; margin: 1rem 0; background: url('../assets/wisp-chart-web/ternary.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-ternary" src="../demo/?chart=ternary" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: USDA soil-texture triangle"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/Soil_texture" target="_blank" rel="noopener">Source: Soil texture — Wikipedia</a>
</p>

## Public surface

```rust,ignore
use wisp_chart::ternary::{TernaryPlot, TernaryPoint};

let points = vec![
    TernaryPoint::new(0.5, 0.3, 0.2, color),
    TernaryPoint::new(0.2, 0.3, 0.5, color),
    // ...
];
let plot = TernaryPlot::new("Sand", "Silt", "Clay", points);
let g = plot.emit_graphics(&theme, Vec2::new(360.0, 360.0));
```

```admonish info title="Barycentric → cartesian"
For triangle vertices `A` (bottom-left), `B` (bottom-right), `C`
(top), a point with normalised components `(a, b, c)` lands at
`a·A + b·B + c·C`. v1 draws the triangle outline, internal grid
lines at 25 / 50 / 75 %, and one ellipse per point.
```

```admonish tip title="Use it when…"
- Three categories must sum to a fixed total (percentages,
  composition, portfolio weights).
- Comparing many compositions at once — the triangle reveals
  clustering that a stacked-bar over time would hide.
- Sediment / petrology / metallurgy — the textbook home of
  ternary diagrams.
```
