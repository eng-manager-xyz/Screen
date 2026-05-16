# Radar / spider chart

Multi-axis polygon overlay for multivariate comparison. Each
entity becomes one polygon; vertices land on the per-axis value
projected onto a polar coord system.

The demo plots the **1960 Rome Olympics medal table**: USA vs
USSR across five categories (gold / silver / bronze / track-and-
field medals / total). 1960 was the first Summer Games the USSR
topped the table at — the kickoff of the Cold War medal
rivalry that ran through the 1988 Seoul boycott. The visible
gap on the **Total** spoke shows the USSR's 32-medal margin.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-chart-web/radar.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-radar" src="../demo/?chart=radar" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: 1960 Olympics USA vs USSR"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/1960_Summer_Olympics_medal_table" target="_blank" rel="noopener">Source: 1960 Summer Olympics medal table — Wikipedia</a>
</p>

## Public surface

```rust,ignore
use wisp_chart::polar::{Radar, RadarAxis, RadarSeries};
use wisp_chart::color::Color;

let r = Radar::new(
    vec![
        RadarAxis::new("speed",       (0.0, 100.0)),
        RadarAxis::new("range",       (0.0, 100.0)),
        RadarAxis::new("comfort",     (0.0, 100.0)),
        RadarAxis::new("efficiency",  (0.0, 100.0)),
        RadarAxis::new("price",       (0.0, 100.0)),
    ],
    vec![
        RadarSeries::new("Model A", vec![80.0, 70.0, 60.0, 90.0, 50.0], Color::from_hex("#0072b2").unwrap()),
        RadarSeries::new("Model B", vec![60.0, 85.0, 80.0, 70.0, 75.0], Color::from_hex("#d55e00").unwrap()),
    ],
);
```

## Layout

```admonish info
Axes are placed at evenly-spaced angles starting at the top
(`+π/2`), winding CCW. Each axis has its own `(min, max)`
domain so heterogeneous units can share the chart.
Concentric polygon gridlines at 25%, 50%, 75%, 100% reference
the scale.
```

## Convex caveat

```admonish warning
wisp's `draw_polygon` is convex-only in v1 (fan triangulation).
Radar polygons are always **star-convex** from the centre, so
the fan from vertex 0 still renders correctly. If you generate
the input vertices another way (custom order, non-star-shaped),
expect visible triangle artifacts.
```

## Best uses

```admonish tip
Radar reads best for **3–6 axes** and **2–4 series**. Beyond
that the polygon overlap turns into visual noise. For
high-dimensional comparisons use a
[parallel-coordinates plot](./parallel-coords.md) instead.
```
