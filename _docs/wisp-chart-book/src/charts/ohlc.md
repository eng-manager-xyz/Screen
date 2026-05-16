# OHLC bar chart

Pre-candlestick OHLC visualisation. A thin vertical line for
the period range with two small horizontal ticks — left = open,
right = close. More compact than candles for dense charts;
preferred by some traders.

The demo plots the same dataset as the
[candlestick chapter](./candlestick.md): the Dow Jones across the
Wall Street Crash of 1929 (Oct 21–30, including the three Black
days). The OHLC encoding makes the open / close ticks easier to
compare across consecutive sessions when range spans are large.

<div style="position: relative; aspect-ratio: 480 / 240; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/ohlc.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-ohlc" src="../demo/?chart=ohlc" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: 1929 Crash as OHLC bars"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/Wall_Street_Crash_of_1929" target="_blank" rel="noopener">Source: Wall Street Crash of 1929 — Wikipedia</a>
</p>

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
