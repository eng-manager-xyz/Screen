# Connected scatterplot

A scatterplot whose points are joined by a line in a meaningful
sequence — usually time. The reader sees the trajectory through
2D space, not just where points cluster.

The demo plots **the US Phillips curve, 1960 → 1980**, with
annual `(inflation, unemployment)` pairs joined in chronological
order. The 1960s sit in the lower-left in the classic downward
trade-off; the **1970s stagflation shock** punches the line out
toward the upper-right (both axes climbing together) — the
empirical observation that broke Keynesian consensus and
powered Milton Friedman's natural-rate theory.

<div style="position: relative; aspect-ratio: 480 / 320; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/connected-scatter.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-connected-scatter" src="../demo/?chart=connected-scatter" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: US Phillips curve 1960-1980"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/Phillips_curve" target="_blank" rel="noopener">Source: Phillips curve — Wikipedia</a>
</p>

## Public surface

```rust,ignore
let plot = Plot::new(quarterly)
    .mark(Mark::Line {
        interpolation: Interpolation::Linear,
        marker: Some(PointStyle::Circle),
    })
    .encode(plot::x("inflation", ScaleKind::Linear))
    .encode(plot::y("unemployment", ScaleKind::Linear))
    .encode(plot::order("quarter_index"))
    .encode(plot::color("decade"));
```

The minimum recipe: a line mark with markers on, X and Y both
`Linear`, and an `order` encoding that names the sort column.

## Order encoding

```admonish info
` Encoding::Order ` sorts each series's rows by the named
numeric column before line-segment emission. Without it, rows
are connected in `DataFrame` insertion order — fine for already-
sorted time series, wrong for shuffled input. Always set it when
your reader expects the line to follow time.
```

## Continuous-X line vs band-X line

The Plot facade detects the X scale kind and routes:

| X `ScaleKind`     | Layout                       | Use case                       |
|-------------------|------------------------------|--------------------------------|
| `Linear` / `Log` / `Time` | Continuous numeric axes  | Connected scatter, time-series |
| `Band` (default)  | Categorical bands at centres | Standard line chart, e.g. quarterly |

Same `Mark::Line` mark; different X scale picks the right
projection automatically.
