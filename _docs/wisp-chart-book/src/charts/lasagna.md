# Lasagna plot

Pack many entity-time series into a single "lasagna" of
horizontal heatmap rows — one entity per row, time across
columns, colour shows value. Reads patterns across hundreds of
entities that a multi-line chart spaghettis up.

<div style="position: relative; aspect-ratio: 600 / 200; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/lasagna.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=lasagna" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: lasagna heatmap"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::heatmap::{LasagnaHeatmap, SequentialPalette};

let l = LasagnaHeatmap::new(
    vec!["entity-1".into(), "entity-2".into(), /* ... */],
    (0..24).map(|h| format!("{h:02}h")).collect(),
    vec![
        vec![/* 24 values */],
        vec![/* ... */],
    ],
).palette(SequentialPalette::magma());
```

```admonish tip
Cells render flush — no gap — which is what makes a lasagna
"lasagna" rather than a [table heatmap](./table-heatmap.md).
The continuous stripe lets the eye scan an entity's trajectory
across time without inter-cell scaffolding.
```
