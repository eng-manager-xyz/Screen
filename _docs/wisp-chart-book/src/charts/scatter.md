# Scatterplot

Two continuous numeric variables plotted as points — correlation
explorations, A/B comparisons, sample distributions. Categorical
colour and varying size for richer reads.

![Scatterplot of 28 samples across 3 species](../assets/wisp-chart-web/scatter.png)

## Public surface

```rust,ignore
let plot = Plot::new(samples)
    .mark(Mark::Point { shape: PointShape::Circle })
    .encode(plot::x("height", ScaleKind::Linear))
    .encode(plot::y("weight", ScaleKind::Linear))
    .encode(plot::color("species"))
    .encode(plot::size("age"));
```

## Point shapes

| Shape       | Primitive used                          |
|-------------|------------------------------------------|
| `Circle`    | `Graphics::draw_ellipse`                 |
| `Square`    | `Graphics::draw_rect`                    |
| `Diamond`   | `Graphics::draw_polygon` (4 verts)       |
| `Triangle`  | `Graphics::draw_polygon` (3 verts)       |
| `Plus`      | Two crossed `draw_rect` calls            |

```admonish info
Both X and Y must use `ScaleKind::Linear` — scatter requires
continuous numeric axes. Categorical X is the domain of bar
charts.
```

## Size encoding

Adding `plot::size(field)` maps a numeric column to marker
radius via `LinearScale` mapped into `(3.0, 18.0)` pixel range.
Use sparingly — too many sizes overlap and obscure the
distribution shape.

## Theme integration

| Theme field                       | Drives                          |
|-----------------------------------|---------------------------------|
| `theme.plot.line_marker_radius_px`| Default marker radius (no Size) |
| `theme.palette`                   | Per-category fill colour        |
