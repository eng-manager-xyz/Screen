# 2D histogram (binned heatmap)

Bin a 2D point cloud into a fixed grid and emit one filled cell
per bin, coloured by count. Useful when a scatterplot
over-plots so heavily that individual points stop being legible.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 420px; margin: 1rem 0; background: url('../assets/wisp-chart-web/histogram2d.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=histogram2d" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: 2D histogram"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::heatmap::Histogram2D;

let points: Vec<(f32, f32)> = collect_xy_observations();
let h2 = Histogram2D::from_points(
    &points,
    /* cols */ 24,
    /* rows */ 24,
    Some(((-5.0, 5.0), (-5.0, 5.0))),  // optional clipping extent
);
let g = h2.emit_graphics(&theme, viewport);
```

```admonish info title="Histogram2D vs. scatter"
Choose 2D-histogram when:

- N ≳ 10k points (over-plotted scatter).
- You want density-readable colour-encoding rather than
  shape-encoding.
- Outliers are less important than the bulk distribution.

Choose [scatter](./scatter.md) when individual points matter
(small N, outlier hunting, labelled-point overlays).
```
