# Tween + Ease

`Tween<V>` is the headline primitive: interpolate a value from
`from` to `to` over `duration`, shaped by a named [`Ease`]. The
ease palette mirrors anime.js v4 exactly so curve names paste
cleanly between codebases.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/tween-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=polar&amp;animate=tween" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: polar plot scale-pulsing via Tween<f32> + Ease::OutBack"></iframe>
</div>

The demo grows the polar plot from `0` to `1` with `Ease::OutBack`
(overshoots past `1.0` then settles), holds, then shrinks back
with `Ease::InCubic`. The still frame catches the overshoot peak
at scale ≈ 1.05.

## Public surface

```rust,ignore
use std::time::Duration;
use wisp_animation::{Tween, Ease};

let pulse = Tween::new(0.0_f32, 1.0, Duration::from_millis(700))
    .ease(Ease::OutBack);

// Sample at any t:
let mid = wisp_animation::Animation::sample(&pulse, Duration::from_millis(350));
```

Every named Penner ease is on the enum: `Linear`, `In{Quad, Cubic,
Expo, Back, Elastic, Bounce}`, `Out{...}`, `InOut{...}`. Plus:

- `Ease::CubicBezier(x1, y1, x2, y2)` — CSS-style parametric ease.
- `Ease::Steps(n)` — `n`-plateau staircase (`@keyframes step` in CSS).
- `Ease::ThereAndBack` — triangle `0 → 1 → 0` for reveal-and-hide.
- `Ease::Fn(fn(f32) -> f32)` — custom rate function for full control.

## Why match an enum

```admonish important title="No `dyn` for ease dispatch"
`Ease` is a flat enum, not a `Box<dyn Easing>`. The compiler
monomorphises `Tween::<V>::sample` per call site; the inner
`match` is branch-predictable; no allocation; no virtual
dispatch. Custom eases live on `Ease::Fn(fn ptr)` which keeps
the value `Copy` and stack-allocated.
```

## Cubic-bezier maths

```admonish note
`CubicBezier(x1, y1, x2, y2)` evaluates via Newton-Raphson with
a bisection fallback — the same approach CSS uses. The unit-test
asserts `cubic-bezier(0.25, 0.1, 0.25, 1.0)` (the CSS `ease`
default) front-loads correctly: `eval(0.5) > 0.5`.
```

## Test invariants

- **Endpoint invariant** — `ease.eval(0.0) == 0.0 && ease.eval(1.0) == 1.0`
  is asserted for **every** named variant.
- **CubicBezier reference** — the CSS-`ease` curve passes through
  five reference points.
- **Steps plateaus** — `Steps(5)` returns `0.0` for `t ∈ [0, 0.2)`,
  `0.2` for `t ∈ [0.2, 0.4)`, …, `0.8` for `t ∈ [0.8, 1.0)`.
- **Custom `Ease::Fn`** dispatches through correctly.

Full source: [`crates/wisp-animation/src/ease.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/ease.rs)
+ [`tween.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/tween.rs).
