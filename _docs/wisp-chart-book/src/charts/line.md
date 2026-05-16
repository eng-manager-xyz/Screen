# Line chart

A line chart connects rows of a `DataFrame` in order with a
stroked polyline. Multi-series support comes via a `Color`
encoding: each distinct category becomes its own line.

## Demo — US unemployment rate, 1929–1941

The demo plots **annual US unemployment** through the Great
Depression and into early World War II. The peak (24.9 % in
1933) is the worst single year on record; the 1937–38 secondary
peak is the "Roosevelt Recession" after premature monetary
tightening. The chart fades in over 1.5 s with `Ease::OutCubic`
— a subtle "the chart is rendering itself" cue rather than a
hard appear.

<div style="position: relative; aspect-ratio: 480 / 320; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/line.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-line" src="../demo/?chart=line&animate=reveal" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: US unemployment 1929–1941"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <button type="button" class="replay-btn" onclick="(function(){var f=document.getElementById('demo-line');f.src=f.src.split('&_t=')[0]+'&_t='+Date.now();})()"><span class="replay-icon">↻</span> Replay animation</button>
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/Unemployment_in_the_United_States" target="_blank" rel="noopener">Source: Unemployment in the United States — Wikipedia</a>
</p>

## Public surface

```rust,ignore
let plot = Plot::new(daily)
    .mark(Mark::Line {
        interpolation: Interpolation::Linear,
        marker: Some(PointStyle::Circle),
    })
    .encode(plot::x("date", ScaleKind::Band))
    .encode(plot::y("value", ScaleKind::Linear))
    .encode(plot::color("metric"));
```

## Mark variants

| Variant                            | When to use                                   |
|------------------------------------|-----------------------------------------------|
| `Mark::Line { Linear, None }`      | Standard time-series / continuous-x lines     |
| `Mark::Line { Step, None }`        | Monotonic step series (quarterly milestones, billing tiers) |
| `Mark::Line { *, Some(Circle) }`   | Sparse data — readers spot individual points  |

## Interpolation

```admonish info
`Interpolation::Linear` connects (x₁, y₁) → (x₂, y₂) directly.
`Interpolation::Step` inserts an L-shaped joint at (x₂, y₁),
producing horizontal-then-vertical segments. Each step segment
doubles the primitive count vs Linear.
```

## Multi-series via Color encoding

When a `Color` encoding is present, the renderer splits rows by
the color column's value and emits one polyline per series. Each
series picks a palette colour from the theme. Use [`Plot::legend`]
to auto-build a matching legend.

[`Plot::legend`]: ../api/wisp_chart/plot/struct.Plot.html#method.legend

## Theme integration

| Theme field                       | Drives                                |
|-----------------------------------|---------------------------------------|
| `theme.plot.line_width_px`        | Stroke thickness of every line        |
| `theme.plot.line_marker_radius_px`| Radius of `PointStyle::Circle` markers|
| `theme.palette`                   | Per-series stroke colour              |

## Coordinate convention

```admonish warning
For the `Band` X scale (categorical x), each row's x position is
the centre of its band. For continuous X (Linear scale on a
numeric column), the x position is the scale's `map(value)`. The
fundamental NDC flip is the same as bars — see [Axes](./axes.md)
for the coordinate-convention note.
```
