# Waterfall chart

Show how a starting value evolves through a sequence of positive
/ negative contributions to a final value — revenue waterfall
(revenue → costs → margin), budget decomposition, P&L bridges.

<div style="position: relative; aspect-ratio: 480 / 240; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/waterfall.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=waterfall" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: waterfall chart"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::finance::{Waterfall, WaterfallRow};

let w = Waterfall::new(vec![
    WaterfallRow::summary("Start",   100.0),
    WaterfallRow::contribution("Revenue",  80.0),
    WaterfallRow::contribution("COGS",    -30.0),
    WaterfallRow::contribution("Opex",    -25.0),
    WaterfallRow::contribution("Tax",     -10.0),
    WaterfallRow::summary("End",     115.0),
]);
```

## Row kinds

| Constructor                       | Visual                              |
|-----------------------------------|-------------------------------------|
| `WaterfallRow::summary(label, v)` | Full-height bar from 0 to `v` — typically Start / End totals |
| `WaterfallRow::contribution(label, d)` | Floating bar sitting on the running total, length \|d\|, coloured by sign |

```admonish info
Positive contributions use `positive_color` (green default).
Negative contributions use `negative_color` (red default).
Summary bars use `summary_color` (blue default).
```
