# Legend

A legend maps a categorical [`Color`] encoding's values to their
palette swatches. The `Plot` facade exposes `Plot::legend(theme)`
to auto-build one from the data; callers can also construct a
`Legend` from scratch when the chart isn't a standard `Plot`.

[`Color`]: ../api/wisp_chart/plot/enum.Channel.html

## Public surface

| Type                             | Purpose                            |
|----------------------------------|------------------------------------|
| [`Legend`]                       | The composed legend value          |
| [`LegendItem`]                   | One swatch + label pair            |
| [`SwatchStyle`]                  | `ColorBox` / `LineSample` / `PointMarker` |
| [`LegendOrientation`]            | `Vertical` / `Horizontal`          |

[`Legend`]: ../api/wisp_chart/legend/struct.Legend.html
[`LegendItem`]: ../api/wisp_chart/legend/struct.LegendItem.html
[`SwatchStyle`]: ../api/wisp_chart/legend/enum.SwatchStyle.html
[`LegendOrientation`]: ../api/wisp_chart/legend/enum.LegendOrientation.html

## Swatch styles

```admonish info
The mark type drives swatch style: bars / areas / cells use
`ColorBox`; line + trend marks use `LineSample`; scatter / dot
marks use `PointMarker`. Mixing styles in one legend is allowed
when a chart layers multiple marks (e.g. a bar + line dual axis).
```

## Auto-build from a Plot

```rust,ignore
let plot = Plot::new(df)
    .mark(Mark::Bar { value_labels: false })
    .encode(plot::x("quarter", ScaleKind::Band))
    .encode(plot::y("revenue", ScaleKind::Linear))
    .encode(plot::color("region"));

let legend = plot.legend(&theme);
// Caller positions + renders the legend separately:
let legend_graphics = legend.emit_graphics(
    Vec2::new(viewport.x - 120.0, 20.0),
    viewport,
    &theme.legend,
    font.cell_pixels() as f32,
);
let _ = stage.add_child(root, legend_graphics);

let labels = legend.emit_text_labels(
    Vec2::new(viewport.x - 120.0, 20.0),
    viewport,
    &theme.legend,
    theme.text_primary,
    &font,
);
for t in labels {
    let _ = stage.add_child(root, t);
}
```

## Orientation

| Orientation | When to use                                           |
|-------------|-------------------------------------------------------|
| Vertical    | Narrow side panels, tall charts, many categories      |
| Horizontal  | Above / below the plot area, ≤ ~6 categories, wide    |

Horizontal layouts wrap to a new row when the running x exceeds
the viewport width.

## Manual construction

```admonish tip
Use the builder when the legend isn't 1:1 with a `Plot`'s color
encoding — e.g. annotating two reference lines on a custom chart
or pulling the same legend into multiple charts.
```

```rust,ignore
let legend = Legend::new()
    .item("Q1", SwatchStyle::ColorBox(navy))
    .item("Q2", SwatchStyle::ColorBox(vermillion))
    .orientation(LegendOrientation::Horizontal);
```
