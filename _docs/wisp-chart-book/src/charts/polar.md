# Polar coordinate plot

Charts where angle has meaning — wind direction, compass
bearing, time-of-day distributions. v1 ships a wind-rose-style
polar bar variant: concentric grid + radial spokes + one filled
sector per category, sized to its value.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-chart-web/polar.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=polar" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: polar plot"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::polar::{PolarPlot, PolarCoord};
use wisp_chart::color::Color;

let plot = PolarPlot::new(
    vec!["N".into(), "NE".into(), "E".into(),  "SE".into(),
         "S".into(), "SW".into(), "W".into(),  "NW".into()],
    vec![12.0, 18.0, 22.0, 30.0, 25.0, 16.0, 14.0, 8.0],
);
let g = plot.emit_graphics(&theme, Vec2::new(320.0, 320.0));
```

## Coord convention

```admonish info
The polar coord system is exposed via [`PolarCoord`] for callers
who want to compose their own primitives on a polar layout:

| Angle (rad)      | Direction       |
|------------------|-----------------|
| `0`              | Right (`+x`)    |
| `π/2`            | Top (`-y` screen)|
| `π`              | Left (`-x`)     |
| `3π/2`           | Bottom (`+y` screen)|

`PolarCoord::to_pixel(θ, r ∈ [0, 1])` projects to pixel space:
`(centre.x + r·cos θ, centre.y − r·sin θ)`. Screen `+y` is down,
so `sin(θ)` is negated to keep "0 = right, π/2 = top" reading.
```

## Sector layout

```admonish note
Sectors start at angle `π/2` (top — N on a compass) and go
**clockwise** through the category list. This matches compass
convention: N → NE → E → … → NW → back to N.
```

## Beyond wind roses

```admonish tip
For richer polar marks (lines, points, custom sectors), use
`PolarCoord` directly to convert your data to pixel positions
and emit `wisp::Graphics` primitives. The `PolarPlot` value
type is a ready-baked wind-rose; the coord system underneath is
general.
```

## Related polar charts

For specific polar shapes that have their own constructors and
chapter — read them first if your use case fits:

- [Pie / donut](./pie.md) — categorical proportions filling 2π.
- [Sunburst](./sunburst.md) — radial hierarchy across rings.
- [Radar](./radar.md) — multi-axis polygon overlay.
- [Gauge](./gauge.md) — semicircle indicator with threshold zones.
