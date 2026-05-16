# Baseline chart

Area chart split by a horizontal reference line — fill above the
baseline in one colour (profit), below in another (loss).
Common for diff series, deviation-from-target, gains-vs-losses.

The demo plots the **US Federal Funds Rate, 1965–1985**,
baselined against the long-run 5 % anchor most macro texts use
when discussing the Volcker disinflation. The visible spikes
above the baseline track the late-60s Vietnam-era overheating,
the post-oil-shock inflation, and the **Volcker shock of
1979–82** that crushed double-digit inflation by taking the
funds rate above 16 % — a peak you can read straight off the
1981 fill.

<div style="position: relative; aspect-ratio: 480 / 240; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/baseline.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-baseline" src="../demo/?chart=baseline" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: US Fed Funds Rate vs 5%"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/Federal_funds_rate" target="_blank" rel="noopener">Source: Federal funds rate — Wikipedia</a>
</p>

## Public surface

```rust,ignore
use wisp_chart::baseline::BaselineChart;

let bc = BaselineChart::new(
    vec![(0.0, 10.0), (1.0, 25.0), (2.0, -10.0), (3.0, 15.0)],
    0.0, // baseline y-value
);
let g = bc.emit_graphics(&theme, Vec2::new(480.0, 240.0));
```

## Per-segment colouring

```admonish info
Each segment between consecutive points is coloured by the
sign of the average y of its endpoints relative to the
baseline. Segments that cross the baseline currently render
with the average-side colour — a future refinement could
split the segment at the crossing point for exact colouring.
```
