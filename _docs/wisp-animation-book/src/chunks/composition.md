# Sequence + Parallel + Delay

Composition primitives. `Sequence` plays children back-to-back;
`Parallel` plays them simultaneously; `Delay` is a no-op spacer.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/composition-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=polar&amp;animate=storyline" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: polar plot driven by Sequence<f32>"></iframe>
</div>

The demo runs a 3-step alpha `Sequence` (fade-in → hold → fade-out)
while a parallel rotation `LinearRamp` rotates the polar plot a
quarter turn over the full 2-second cycle.

## `Sequence` — concat in time

```rust,ignore
use std::time::Duration;
use wisp_animation::{Sequence, Tween, Ease};

let alpha: Sequence<f32> = Sequence::new()
    .then(Tween::new(0.0_f32, 1.0, Duration::from_millis(700)).ease(Ease::OutCubic))
    .then(Tween::new(1.0_f32, 1.0, Duration::from_millis(600)))   // hold
    .then(Tween::new(1.0_f32, 0.0, Duration::from_millis(700)).ease(Ease::InCubic));

// Duration is the SUM of children: 2 s total.
// Sampling at any t finds the active child and dispatches.
```

## `Parallel` — simultaneous, multi-output

```rust,ignore
use wisp_animation::{Parallel, Tween};

let pulses: Parallel<f32> = Parallel::new()
    .with(Tween::new(0.0_f32, 1.0, Duration::from_millis(500)))
    .with(Tween::new(0.0_f32, 0.5, Duration::from_millis(800)));

// Duration is the MAX of children.
// `sample_all(t)` returns one value per child for fan-out.
```

## `Delay` — pad time

```rust,ignore
use wisp_animation::{Delay, Sequence, Tween};

let with_pause: Sequence<f32> = Sequence::new()
    .then(Tween::new(0.0_f32, 1.0, Duration::from_millis(300)))
    // ... Delay::new only outputs (), so use a held-value Tween
    // when you need to keep `f32` output flowing during the pause.
    .then(Tween::new(1.0_f32, 1.0, Duration::from_millis(400)))
    .then(Tween::new(1.0_f32, 0.0, Duration::from_millis(300)));
```

## Why boxed children

```admonish important title="Children erase their concrete type"
`Sequence<O>` and `Parallel<O>` store children as
`Box<dyn Animation<Output = O>>`. That lets you mix a
`Tween<f32>` and a `LinearRamp` in the same sequence — the
common output type is what unifies them.

The dyn-call lives on the cold path (one virtual call per child
per sample), and `Box<dyn Animation>` keeps the value `'static`
so it can be stored in a `Driver`. There's no per-frame allocation
once construction is done.
```

## Test invariants

- `Sequence::duration()` = sum of child durations.
- Sampling a Sequence at `t` dispatches to the child whose
  cumulative window contains `t`; sampling past the end returns
  the last child's terminal value.
- `Parallel::duration()` = max of child durations.
- `Parallel::sample_all(t)` returns one value per child in
  declaration order.
- An empty `Sequence<f32>` returns `0.0` and `Duration::ZERO`.

Full source: [`crates/wisp-animation/src/composition.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/composition.rs).
