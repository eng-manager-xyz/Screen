# Bullet chart

Stephen Few's compact performance-vs-target chart. A horizontal
(or vertical) bar with three qualitative ranges painted behind
it, a target marker line, and the current value as a thinner
foreground bar.

The demo reports **the 2005 DARPA Grand Challenge** — Stanford's
"Stanley" drove 132.2 mi across the Mojave (target: 132 mi) in
6 h 53 m to take the $2 M prize. The 2004 Challenge's best
result was 7.4 mi; one year later five robots finished. The
visible band just inside the target marker is the moment self-
driving cars stepped off the slide deck.

<div style="position: relative; aspect-ratio: 400 / 80; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/bullet.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-bullet" src="../demo/?chart=bullet" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: 2005 DARPA Grand Challenge"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/DARPA_Grand_Challenge_(2005)" target="_blank" rel="noopener">Source: DARPA Grand Challenge (2005) — Wikipedia</a>
</p>

## Public surface

```rust,ignore
use wisp_chart::indicator::{Bullet, Orientation};

let bullet = Bullet {
    value: 270.0,
    target: 250.0,
    ranges: [150.0, 225.0, 300.0], // poor → ok → good thresholds
    orientation: Orientation::Horizontal,
};

let g = bullet.emit_graphics(&theme, Vec2::new(400.0, 80.0));
let _ = stage.add_child(root, g);
```

## The five primitives

```admonish info
A bullet chart renders five primitives in this order:

1. **Poor** band — `[0, ranges[0]]`, light grey
2. **OK** band — `[0, ranges[1]]`, medium grey (paints over poor)
3. **Good** band — `[0, ranges[2]]`, darker grey (paints over OK)
4. **Value bar** — thinner foreground bar from 0 to `value`
5. **Target line** — vertical (or horizontal) marker at `target`

Bands paint over each other because each spans from 0 to its
threshold. The visible bands are the *differences*: poor is the
portion before `ranges[0]`, OK is the strip from `ranges[0]` to
`ranges[1]`, good is the strip from `ranges[1]` to `ranges[2]`.
```

## Orientation

| Orientation         | Use case                                  |
|---------------------|-------------------------------------------|
| `Horizontal` (default) | Dashboard rows, tight vertical packing  |
| `Vertical`          | Sidebar KPIs, when label fits to the left|

## Theme integration

| Field                                  | Drives                          |
|----------------------------------------|---------------------------------|
| `theme.indicator.bullet_poor_color`    | Poor (lowest) qualitative band  |
| `theme.indicator.bullet_ok_color`      | Satisfactory (middle) band      |
| `theme.indicator.bullet_good_color`    | Good (highest) band             |
| `theme.indicator.bullet_value_color`   | Value bar fill                  |
| `theme.indicator.bullet_target_color`  | Target marker line              |

## Why bullet vs gauge

```admonish tip
For one-shot dashboard tiles where vertical space is tight,
bullet wins — it fits inside a row that already has a label and
delta, while a gauge needs its own square aspect. Save gauges
for the *one* hero metric that justifies the footprint.
```
