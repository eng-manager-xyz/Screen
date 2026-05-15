# Easing gallery

A live, side-by-side reference of every easing curve in
`wisp-animation` — 36 mini charts in a single WebGPU canvas, each
plotting one `Ease` variant with a small square riding the curve.
The progress readout in the top-left ramps from `0.00` to `1.00`
over 2 s, holds for 3 s, then loops. Watch one card at a time to
build intuition, or scan the whole grid to compare families.

<div style="position: relative; aspect-ratio: 4 / 3; max-width: 720px; margin: 1rem 0; background: url('../assets/wisp-animation/easing-gallery-hero.png') center/contain no-repeat #1a1a1f; border: 1px solid #2a2a30;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?animate=easing" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: 36-curve easing gallery"></iframe>
</div>

The grid is grouped by row: row 0 is the **In** family (slow start),
row 1 is **Out** (slow finish), row 2 is **InOut** (slow at both
ends), and row 3 is the miscellaneous shapes — `Linear`,
`Steps(4)` / `Steps(8)`, `ThereAndBack`, and two CSS-style
`CubicBezier` curves.

## Reading a single card

Each card plots `y = ease(t)` for `t ∈ [0, 1]`. The dot's
horizontal position is the progress `t`; its vertical position is
the *eased* value `ease(t)`. So **`Linear` traces a 45° line**
because input == output, **`InQuad` curves up sharply near the
end** because `t²` is small for small `t`, and **`OutElastic`
spikes above the card's top edge** because the curve overshoots
`1.0` while settling.

```admonish tip title="Why row-by-row matters"
Compare `InQuad` (row 0) → `OutQuad` (row 1) → `InOutQuad` (row
2) of the same family. The In curve eases the *start*; the Out
curve eases the *finish*; the InOut version does both. That
three-step pattern is consistent across every named family in
the gallery.
```

## Using an easing

Easings are values, not types — pick one, hand it to whatever
needs shaping. The same `Ease` variant works on `Tween<V>`,
`Track<V>`, `ColorTween`, and the FLIP / Enter / Exit helpers.

```rust,ignore
use std::time::Duration;
use wisp_animation::{Animation, Ease, Tween};

// 1. Pick an ease — the shape of the motion.
let bounce_in = Tween::new(0.0_f32, 1.0, Duration::from_millis(800))
    .ease(Ease::InBounce);

// 2. Sample at any t. Pure function — t doesn't have to advance
//    monotonically; rewinding works.
let halfway = bounce_in.sample(Duration::from_millis(400));

// 3. CSS-style cubic-bezier matches the curves your designer
//    sketched in Figma.
let designer = Tween::new(0.0_f32, 1.0, Duration::from_millis(600))
    .ease(Ease::CubicBezier(0.34, 1.56, 0.64, 1.0));

// 4. The same Ease values plug into other curves — keyframes,
//    color tweens, FLIP transitions.
use wisp_animation::Track;
let track: Track<f32> = Track::new()
    .key(Duration::ZERO, 0.0)
    .key_eased(Duration::from_millis(500), 1.2, Ease::OutBack)
    .key_eased(Duration::from_millis(1_000), 1.0, Ease::InOutQuad);
```

```admonish note title="`Ease` is a flat enum, not a trait object"
Every named variant is a stack-allocated `Copy` value; the
compiler monomorphises `Tween::sample` per call site and the
inner match is branch-predictable. Custom easings live on
`Ease::Fn(fn(f32) -> f32)` — still `Copy`, still no
allocation. See [Tween + Ease](./tween.md) for the full type
breakdown.
```

## Picking the right ease

| Want | Reach for |
|---|---|
| Constant velocity / debug ramp | `Linear` |
| Smooth, organic in / out | `InSine` / `OutSine` / `InOutSine` |
| The "default UI" curve | `InOutCubic` |
| Sharper acceleration | `InQuart` → `InQuint` → `InExpo` |
| Slow into wall (parallax, scroll-end) | `OutCirc` |
| Anticipation / overshoot | `InBack` / `OutBack` / `InOutBack` |
| Springy overshoot rebound | `InElastic` / `OutElastic` / `InOutElastic` |
| Rubber-ball settle | `OutBounce` |
| Discrete reveal / typewriter beat | `Steps(n)` |
| Reveal then hide one-shot | `ThereAndBack` |
| Match a CSS or Figma curve | `CubicBezier(x1, y1, x2, y2)` |

```admonish important title="Where 'overshoot' eases leave the [0,1] box"
`Back` and `Elastic` deliberately produce values outside
`[0, 1]` during the curve. If you're tweening a value that
*must* stay in range (alpha, an unsigned scalar, a normalized
weight), clamp at the call site — the eases don't clamp for
you, and that's the point: the overshoot is what makes them
feel alive.
```

## Demo layout

Built in three layers on a single wgpu canvas:

- **Static** — a dark backdrop + 36 bordered cards + 64-segment
  polyline per card. Constructed once at `setup_animation` and
  never touched again. ~2,300 primitives total, all in one
  `Graphics` node so the SDF batcher submits the whole grid in a
  single draw call.
- **Dots** — 36 small squares, one per card, rebuilt every frame
  at the new progress value.
- **Text** — the 36 per-card labels are built once; the top-left
  `Progress: 0.00` readout is rebuilt every frame so the digits
  update.

The layout / curve / dot construction lives in
[`crates/wisp-chart-web/src/easing_grid.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-chart-web/src/easing_grid.rs)
behind small pure functions, so the native hero snapshot test
renders the *same* scene at a frozen `progress = 0.6` — what you
see above is what the live iframe shows every frame.
