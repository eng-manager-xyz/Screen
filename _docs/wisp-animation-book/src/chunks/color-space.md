# Color interpolation (`ColorSpace`)

`ColorTween` interpolates `wisp::Color` between two endpoints
under a chosen colour space. Default is **LinearRgb** because
wisp composes and clears in linear sRGB — that's the cheapest
and most predictable shape. Opt into **Oklab** (perceptually
uniform Lab) or **Oklch** (perceptual + polar hue arc) when
crossing hue boundaries.

<div style="position: relative; aspect-ratio: 3 / 1; max-width: 540px; margin: 1rem 0; background: url('../assets/wisp-animation/color-spaces-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?animate=color" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: 3-ellipse colour-cycle in LinearRgb / Oklab / Oklch"></iframe>
</div>

The demo cycles three ellipses through `red → green → blue → red`
— left in LinearRgb, middle in Oklab, right in Oklch. The
LinearRgb midpoints look muddier; Oklab is perceptually
smoother; Oklch traces a hue arc.

## API surface

```rust,ignore
use std::time::Duration;
use wisp::Color;
use wisp_animation::{Animation, ColorTween};

let red = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
let green = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };

let lrgb = ColorTween::new(red, green, Duration::from_secs(1));         // default
let oklab = ColorTween::new(red, green, Duration::from_secs(1)).in_oklab();
let oklch = ColorTween::new(red, green, Duration::from_secs(1)).in_oklch();
```

## When to reach for each

```admonish info
- **LinearRgb** — default. Fast. Use unless you have a reason.
- **Oklab** — palette transitions where the midpoint matters
  (sequential heatmap reveals, brand-colour rotations).
- **Oklch** — hue sweeps where you want the short-arc path
  through hue space (rainbow gradients, single-axis "colour
  picker" animations).
```

## Test invariants

- Endpoints match across all three spaces (within `1e-3`).
- Oklab midpoint differs measurably from LinearRgb midpoint
  for red→green (`||lrgb_mid − oklab_mid|| > 0.05`).
- Oklch red → magenta takes the short-arc path (midpoint has
  low green channel).

Full source: [`crates/wisp-animation/src/color_space.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/color_space.rs).
