# Lasagna plot

Pack many entity-time series into a single "lasagna" of
horizontal heatmap rows — one entity per row, time across
columns, colour shows value. Reads patterns across hundreds of
entities that a multi-line chart spaghettis up.

The demo plots **US polio incidence per 100 k population by
state × half-year, 1952 → 1956**. The Salk inactivated polio
vaccine was approved 12 Apr 1955 and rolled out nationally that
spring; every state's incidence collapses to near-zero in the
two columns after the vaccine launch. The "before / after"
contrast is the visual evidence that anchored the global
eradication effort.

<div style="position: relative; aspect-ratio: 600 / 200; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/lasagna.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-lasagna" src="../demo/?chart=lasagna" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: US polio incidence 1952–56"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/Polio_vaccine" target="_blank" rel="noopener">Source: Polio vaccine — Wikipedia</a>
</p>

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
