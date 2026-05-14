# Bubble chart

A bubble chart is a scatterplot with a third magnitude encoded
as marker size — e.g. GDP × life-expectancy × population.

<div style="position: relative; aspect-ratio: 480 / 320; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/bubble.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=bubble" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: bubble chart"></iframe>
</div>

## Public surface

```rust,ignore
let plot = Plot::new(countries)
    .mark(Mark::Point { shape: PointShape::Circle })
    .encode(plot::x("gdp", ScaleKind::Log))
    .encode(plot::y("life_expectancy", ScaleKind::Linear))
    .encode(plot::size("population").size_mapping(SizeMapping::Area))
    .encode(plot::color("continent"));
```

## Area vs radius mapping

```admonish important
Always use `SizeMapping::Area` for magnitude data. Radius
mapping is visually misleading: a 4× value renders 16× larger
because area = πr². Area mapping preserves the perceptual link
between value and visible bubble size.
```

| `SizeMapping` | Behaviour                              | When to use         |
|---------------|----------------------------------------|---------------------|
| `Area` (default) | sqrt(scaled value) → radius. 4× value → 4× visible bubble. | Magnitudes, populations, totals |
| `Radius`      | Scaled value → radius directly.        | When the value is already a length/distance |

## Multi-encoding read

A typical bubble chart uses 4 channels at once: X, Y, Size,
Color. Combined with [`Plot::legend`](../api/wisp_chart/plot/struct.Plot.html#method.legend),
the reader sees the Color legend automatically; future tickets
will add an explicit Size legend overlay for the third dimension.
