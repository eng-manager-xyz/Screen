# Pie / donut chart

Categorical proportions of a whole — budget allocation, market
share, traffic-source mix. Donut variant adds a centred hole.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-chart-web/pie.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=pie" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: pie chart"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::polar::{Pie, Slice};
use wisp_chart::color::Color;

let pie = Pie::new(vec![
    Slice::new(45.0, "Organic",  Color::from_hex("#0072b2").unwrap()),
    Slice::new(25.0, "Paid",     Color::from_hex("#d55e00").unwrap()),
    Slice::new(15.0, "Social",   Color::from_hex("#009e73").unwrap()),
    Slice::new(10.0, "Referral", Color::from_hex("#cc79a7").unwrap()),
    Slice::new(5.0,  "Direct",   Color::from_hex("#f0e442").unwrap()),
]);
let g = pie.emit_graphics(&theme, Vec2::new(320.0, 320.0));
```

## Donut variant

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-chart-web/donut.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=donut" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: donut chart"></iframe>
</div>

```rust,ignore
let donut = Pie::new(slices).hollow_ratio(0.5);
```

`hollow_ratio` is clamped to `[0, 0.95]`. `0.0` = pie. Common
donut hole values are `0.4` – `0.6` (typical brand donuts) or
`0.7+` (thin ring).

## Layout

```admonish info
Slices render in input order, winding CCW from the `+x` axis
(3 o'clock). Each slice's angular span is `value / total * 2π`.
The pie centre sits at the centre of `viewport_px`; outer
radius is `min(width, height) * 0.45`.
```

## Caveats

```admonish warning
Pie charts are perceptually unreliable for comparing slices
that aren't dramatically different in size — readers can't
estimate angle ratios well. Prefer a [bar chart](./grouped-bar.md)
when the ranking of values matters.
```
