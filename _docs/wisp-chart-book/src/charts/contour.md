# Contour plot

Draw iso-level contours over a 2D scalar field using marching
squares. Useful for topology, terrain, response surfaces, and 2D
density visualisations where the lines themselves are the
feature.

The demo draws **the bivariate normal density** Sir Francis
Galton popularised in his 1885 quincunx + regression-board
demonstration — a single radial Gaussian peak with five
nested iso-density contours. The same shape underlies modern
density estimation and 2D KDEs.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 420px; margin: 1rem 0; background: url('../assets/wisp-chart-web/contour.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-contour" src="../demo/?chart=contour" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: bivariate normal contours"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/Galton_board" target="_blank" rel="noopener">Source: Galton board — Wikipedia</a>
</p>

## Public surface

```rust,ignore
use wisp_chart::contour::ContourPlot;

let field: Vec<f32> = sample_function_on_grid();  // rows × cols
let plot = ContourPlot::new(
    field,
    /* cols */ 48,
    /* rows */ 48,
    vec![0.15, 0.35, 0.55, 0.75, 0.9],   // iso-levels
);
let g = plot.emit_graphics(&theme, viewport);
```

```admonish info title="Marching squares"
Each grid cell's four corners are compared to the iso-level
threshold; the 16 possible above/below sign-bit combinations map
to 16 line-segment cases. The implementation is a flat `match` —
no degenerate ambiguities are smoothed (saddle points pick a
fixed orientation), which keeps the output deterministic across
runs and platforms.
```

```admonish tip title="Contour vs. filled heatmap"
- A [2D histogram](./histogram2d.md) or
  [table heatmap](./table-heatmap.md) shows the bulk of the
  field via colour — easy to read overall shape.
- A contour plot shows specific *levels* — easy to read
  "everything above 0.75 is here". Compose both: filled heatmap
  underneath, contour lines on top.
```
