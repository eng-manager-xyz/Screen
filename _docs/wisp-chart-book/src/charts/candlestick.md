# Candlestick chart

OHLC price per period rendered as a body (open → close) with a
thin wick spanning the period high → low.

<div style="position: relative; aspect-ratio: 480 / 240; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/candlestick.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=candlestick" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: candlestick"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::finance::{Candlestick, Period};

let c = Candlestick::new(vec![
    Period::new(100.0, 110.0, 95.0, 108.0),
    Period::new(108.0, 115.0, 105.0, 102.0),
    /* ... */
]);
let g = c.emit_graphics(&theme, Vec2::new(480.0, 240.0));
```

```admonish info
Up periods (`close >= open`) use `up_color` (green default);
down periods use `down_color` (red default). Override before
emission for brand colours.
```

## Why not a Plot mark

```admonish note
`Period { open, high, low, close }` doesn't fit the
`(X, Y, Color)` channel model of the [Plot facade](./plot.md) —
it's 4 numeric fields per row. Candlestick / OHLC / waterfall
ship as self-contained value types under `wisp_chart::finance`
instead.
```
