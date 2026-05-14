# Table heatmap

Show a 2D matrix of values — confusion matrices, hour×day
activity, regional×product sales. Colour intensity replaces
explicit numeric labels for fast pattern recognition.

<div style="position: relative; aspect-ratio: 400 / 240; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/table-heatmap.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=table-heatmap" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: table heatmap"></iframe>
</div>

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
