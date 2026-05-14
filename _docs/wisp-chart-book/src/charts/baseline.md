# Baseline chart

Area chart split by a horizontal reference line — fill above the
baseline in one colour (profit), below in another (loss).
Common for diff series, deviation-from-target, gains-vs-losses.

<div style="position: relative; aspect-ratio: 480 / 240; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/baseline.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=baseline" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: baseline chart"></iframe>
</div>

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
