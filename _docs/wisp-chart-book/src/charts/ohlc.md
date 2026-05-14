# OHLC bar chart

Pre-candlestick OHLC visualisation. A thin vertical line for
the period range with two small horizontal ticks — left = open,
right = close. More compact than candles for dense charts;
preferred by some traders.

<div style="position: relative; aspect-ratio: 480 / 240; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/ohlc.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=ohlc" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: OHLC bar chart"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::finance::{Ohlc, Period};

let o = Ohlc::new(vec![
    Period::new(100.0, 110.0, 95.0, 108.0),
    /* ... */
]);
let g = o.emit_graphics(&theme, Vec2::new(480.0, 240.0));
```

Tick length is `tick_length_fraction × band_width` (default `0.3`).
