# Scatterplot matrix (SPLOM)

Quickly survey all pairwise relationships in a multi-dimensional
dataset — N variables produce an N×N grid where each cell is a
scatter of the row's variable vs the column's.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 480px; margin: 1rem 0; background: url('../assets/wisp-chart-web/splom.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=splom" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: SPLOM"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::multi::{Splom, SplomDimension};

let s = Splom::new(vec![
    SplomDimension::new("mpg", vec![32.0, 28.0, 22.0, 18.0, 14.0, 12.0]),
    SplomDimension::new("cyl", vec![ 4.0,  4.0,  6.0,  6.0,  8.0,  8.0]),
    SplomDimension::new("hp",  vec![95.0,110.0,150.0,200.0,280.0,300.0]),
    SplomDimension::new("wt",  vec![ 2.2,  2.5,  3.0,  3.6,  4.4,  5.0]),
]);
let g = s.emit_graphics(&theme, Vec2::new(400.0, 400.0));
```

## Diagonal

```admonish info
v1 leaves the diagonal cells blank. A follow-on ticket replaces
each diagonal with a small histogram (or density / KDE) of that
single dimension. The off-diagonal mini-scatters are the
primary read until then.
```

## Sizing

```admonish tip
SPLOM viewports want **square aspect ratios** so each cell is
square — easier to compare angle and density across cells. 4-
dimension SPLOM at 400×400 px gives 100×100 px cells, which is
already a tight read; for 6+ dims aim for 600+ px on the long
edge.
```
