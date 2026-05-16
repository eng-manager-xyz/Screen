# 2D histogram (binned heatmap)

Bin a 2D point cloud into a fixed grid and emit one filled cell
per bin, coloured by count. Useful when a scatterplot
over-plots so heavily that individual points stop being legible.

The demo plots **the Hertzsprung–Russell diagram** — stellar
effective temperature (log K, X) against absolute magnitude
(M_V, Y, brighter up) for ~640 synthesised stars. The dense
diagonal running top-right → bottom-left is the **main
sequence**; the cluster top-left is **white dwarfs**; the
upper-right scatter is **red giants + supergiants**. The chart
was published independently by Hertzsprung (1911) and Russell
(1913) and is the single most important diagram in stellar
astrophysics.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 420px; margin: 1rem 0; background: url('../assets/wisp-chart-web/histogram2d.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-histogram2d" src="../demo/?chart=histogram2d" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: Hertzsprung-Russell diagram"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/Hertzsprung%E2%80%93Russell_diagram" target="_blank" rel="noopener">Source: Hertzsprung–Russell diagram — Wikipedia</a>
</p>

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
