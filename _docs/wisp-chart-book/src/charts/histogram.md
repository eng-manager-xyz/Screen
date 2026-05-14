# Histogram

Bin a sample of scalar observations into equal-width buckets and
emit one bar per bin. The default binning rule is the square-root
rule (`⌈√n⌉`); pass `BinCount::Fixed(n)` for a fixed bin count.

<div style="position: relative; aspect-ratio: 3 / 2; max-width: 540px; margin: 1rem 0; background: url('../assets/wisp-chart-web/histogram.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=histogram" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: histogram"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::distributions::{BinCount, Histogram};

let samples: Vec<f32> = collect_observations();
let hist = Histogram::from_samples(
    &samples,
    BinCount::Auto,           // sqrt-rule
    Some((0.0, 100.0)),       // optional clamping extent
);
let g = hist.emit_graphics(&theme, Vec2::new(360.0, 240.0));
```

```admonish info title="Binning rules"
- `BinCount::Auto` — `⌈√n⌉` bins. Cheap, robust, biased toward
  over-binning for very large samples.
- `BinCount::Fixed(k)` — explicit bin count. Use when comparing
  multiple histograms side-by-side so the bars line up.
```

```admonish tip title="Histogram vs. KDE"
A histogram shows you exactly which observations landed where —
useful for outlier hunting and reading off exact counts. A
[KDE](./kde.md) shows you the underlying density estimate —
useful when the bin-edge choice would distort the story. They
compose; some teams ship both stacked.
```
