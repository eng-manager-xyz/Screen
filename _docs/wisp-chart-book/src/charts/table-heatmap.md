# Table heatmap

Show a 2D matrix of values — confusion matrices, hour×day
activity, regional×product sales. Colour intensity replaces
explicit numeric labels for fast pattern recognition.

The demo plots **weekly excess-mortality rate (per 1 000) during
the 1918 influenza pandemic** across five US cities × eight
weeks (late Sep – mid-Nov). Cities that imposed early NPIs —
school closures, public-gathering bans — cap visibly lower than
cities that delayed; Philadelphia's catastrophic week-4 peak
followed its decision to allow a 200 000-person Liberty Loan
parade on Sep 28. The data anchors Markel et al.'s 2007 *JAMA*
analysis of NPI effectiveness.

<div style="position: relative; aspect-ratio: 400 / 240; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/table-heatmap.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-table-heatmap" src="../demo/?chart=table-heatmap" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: 1918 flu weekly mortality"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/1918_flu_pandemic_in_the_United_States" target="_blank" rel="noopener">Source: 1918 flu pandemic in the United States — Wikipedia</a>
</p>

## Public surface

```rust,ignore
use wisp_chart::heatmap::{TableHeatmap, SequentialPalette};

let h = TableHeatmap::new(
    vec!["Mon".into(), "Tue".into(), "Wed".into()],
    vec!["00h".into(), "06h".into(), "12h".into(), "18h".into()],
    vec![
        vec![ 5.0, 20.0, 40.0, 18.0],
        vec![ 8.0, 25.0, 50.0, 22.0],
        vec![10.0, 30.0, 55.0, 24.0],
    ],
).palette(SequentialPalette::blues());
let g = h.emit_graphics(&theme, Vec2::new(400.0, 240.0));
```

## Palette options

| Palette                       | Use case                           |
|-------------------------------|------------------------------------|
| `SequentialPalette::blues()`  | Default — single-hue magnitude     |
| `SequentialPalette::magma()`  | Heat / intensity reads             |
| `SequentialPalette::github()` | Discrete-level contribution graphs |
| `SequentialPalette::new(stops)` | Custom palette from your colours |

```admonish info
Each cell's colour is `palette.sample((value - lo) / (hi - lo))`
where `(lo, hi)` is the matrix's numeric extent. Linear
interpolation between adjacent palette stops.
```
