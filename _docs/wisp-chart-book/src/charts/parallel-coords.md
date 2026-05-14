# Parallel coordinates plot

Explore multivariate datasets (4–12 dimensions) by drawing one
polyline per row across parallel vertical axes — each axis
normalised to its own domain. Patterns (clusters, outliers,
anti-correlated dimensions) emerge from polyline shapes.

<div style="position: relative; aspect-ratio: 480 / 280; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/parallel-coords.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=parallel-coords" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: parallel coordinates"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::distributions::{ParallelCoords, ParallelAxis, ParallelRow};
use wisp_chart::color::Color;

let c = |hex| Color::from_hex(hex).unwrap();
let pc = ParallelCoords::new(
    vec![
        ParallelAxis::new("mpg", (10.0, 50.0)),
        ParallelAxis::new("cyl", ( 3.0,  8.0)),
        ParallelAxis::new("hp",  (60.0, 300.0)),
        ParallelAxis::new("wt",  ( 1.5,   5.5)),
    ],
    vec![
        ParallelRow::new(vec![32.0, 4.0,  95.0, 2.2], c("#0072b2")),
        ParallelRow::new(vec![14.0, 8.0, 280.0, 4.4], c("#009e73")),
        /* ... */
    ],
);
```

```admonish info
Each axis carries its own `(min, max)` domain so heterogeneous
units share the chart cleanly. Values are clamped to `[0, 1]`
of their axis before pixel mapping.
```

## Visual reads

```admonish tip
**Crossing patterns**: lines that cross between two adjacent
axes signal negative correlation. Lines that stay parallel
signal positive correlation. **Clusters**: bundles of lines
following a common shape across most axes indicate sub-groups
worth investigating with a more pointed chart (scatter, box).
```
