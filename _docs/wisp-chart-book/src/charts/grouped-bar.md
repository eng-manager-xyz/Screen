# Grouped bar chart

Side-by-side comparison of 2–5 series within each X-band — e.g.
revenue per region per quarter. Each outer band (a quarter) is
subdivided into one inner band per series (a region).

<div style="position: relative; aspect-ratio: 480 / 320; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/grouped-bar.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=grouped-bar" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: grouped bar chart"></iframe>
</div>

## Public surface

The grouped layout is one extra encoding on top of the standard
bar chart:

```rust,ignore
let plot = Plot::new(rows)
    .mark(Mark::Bar { value_labels: false })
    .encode(plot::x("quarter", ScaleKind::Band))
    .encode(plot::y("revenue", ScaleKind::Linear))
    .encode(plot::color("region"))
    .encode(plot::x_offset("region"));
```

Adding `plot::x_offset(field)` re-bands the X axis: each unique
value of `field` becomes a sub-band within the outer X band.

## Layout

```admonish info
The outer X band (e.g. "Q1") spans some pixel range
`[bx0, bx1]`. The grouped layout constructs an inner
`BandScale` over the distinct XOffset categories with that
range as its pixel range and a 10% inner padding, then asks for
the sub-band's `[ix0, ix1]`.
```

## Pairing with Legend

When the same column drives both `Color` and `XOffset`,
[`Plot::legend`](../api/wisp_chart/plot/struct.Plot.html#method.legend)
returns a legend whose colours match the bar segments exactly.
Use the [legend chapter](./legend.md) for placement.

## Theme integration

Grouped bars reuse the bar palette + theme. Inner band padding
is fixed at `0.10` today; an explicit `PlotTheme.bar_inner_padding`
field lands when the value needs to be customisable.
