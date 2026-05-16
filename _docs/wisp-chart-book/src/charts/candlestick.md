# Candlestick chart

OHLC price per period rendered as a body (open → close) with a
thin wick spanning the period high → low.

The demo plots **the Dow Jones Industrial Average around the Wall
Street Crash of 1929** — eight trading days from Mon Oct 21
through Wed Oct 30 1929, spanning *Black Thursday* (Oct 24),
*Black Monday* (Oct 28), and *Black Tuesday* (Oct 29). The
Oct 28 candle's body is the day the Dow lost 13 % in a single
session; Oct 29 cut another 12 %, then a partial recovery on
the 30th set the tone for the Great Depression to come.

<div style="position: relative; aspect-ratio: 480 / 240; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/candlestick.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-candlestick" src="../demo/?chart=candlestick" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: Wall Street Crash 1929"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/Wall_Street_Crash_of_1929" target="_blank" rel="noopener">Source: Wall Street Crash of 1929 — Wikipedia</a>
</p>

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
