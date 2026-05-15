# Animation trait + Driver

The first thing this crate ships: the `Animation` trait, the
`Driver` clock, and a `LinearRamp` placeholder animation that's
enough to drive a real chart in the browser. Everything that
lands later (Tween, Spring, Sequence, Stagger, FLIP) is built on
this contract.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/animation-trait-driver-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=polar&amp;animate=spin" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: polar plot driven by a wisp-animation Driver"></iframe>
</div>

The demo above is a `wisp-chart` polar plot whose rotation is
driven each `requestAnimationFrame` by a `wisp_animation::Driver`
sampling a `LinearRamp` from `0.0` to `2π` over one second, set
to loop. The chart itself is unmodified — the animation reaches
in from outside and mutates its rotation.

## The contract

```rust,ignore
use std::time::Duration;

pub trait Animation {
    type Output;
    fn duration(&self) -> Duration;
    fn sample(&self, t: Duration) -> Self::Output;
}
```

Three method signatures, one unspoken rule: **`sample` is a pure
function**. Don't read a clock inside it. Don't allocate. Don't
cache. Two callers sampling the same value at the same `t` must
get the same output.

## The driver

```rust,ignore
use std::time::Duration;
use wisp_animation::{Driver, LinearRamp};

// 60 fps fixed step — same bytes across runs for offline export.
let mut driver = Driver::fixed(Duration::from_secs_f32(1.0 / 60.0));
driver.play();

let ramp = LinearRamp::new(0.0, std::f32::consts::TAU, Duration::from_secs(1));

// In your render loop:
driver.tick(Duration::ZERO);                  // advance the clock
let rotation_radians = driver.sample(&ramp);  // sample any animation
```

In real-time mode (`Driver::realtime()`) the caller passes the
wall-clock `dt` to `tick`. In fixed-step mode (`Driver::fixed(dt)`)
the `dt` argument is ignored and the driver advances by exactly
the stored step on every call.

## Why this shape

```admonish important title="Both modes call the same `sample(t)`"
The driver owns time; the animation does not. Two consequences:

- **Determinism is free.** Fixed-step + pure `sample` = same MP4
  bytes across runs and platforms.
- **Composition is free.** Multiple animations can read the same
  `Driver::elapsed` to stay in lockstep without sharing state.
```

```admonish warning title="Don't read `Instant::now()` inside `sample`"
The whole crate is wasm-clean *because* `Animation` values are
timeless. If `sample` peeks at wall-clock time, the value stops
being a function and the deterministic-export contract dies. The
host injects `dt` into the driver; the driver injects `t` into
`sample`; nothing else reads a clock.
```

```admonish tip title="Picking realtime vs fixed"
`Driver::realtime()` is for the interactive `winit` loop —
variable frame times, can be paused/scrubbed by user input.
`Driver::fixed(dt)` is for `wisp-export-animated` and for tests:
deterministic, byte-identical, and the caller's `dt` argument is
ignored on purpose so a malformed host clock can't pollute the
output.
```

## Driving a chart from an animation

The demo above is roughly this — `wisp-chart-web` ticks a driver
on every `requestAnimationFrame` and feeds its sample into the
chart's root-node rotation before re-rendering:

```rust,ignore
fn animation_frame(state: &mut WebState) {
    let dt = Duration::from_secs_f32(1.0 / 60.0);
    state.driver.tick(dt);
    let rotation = state.driver.sample(&state.spin_anim);
    // wrap into [0, TAU) so the loop is seamless when the ramp
    // exceeds its duration and clamps to the endpoint.
    state.stage.set_rotation(state.chart_root, rotation % std::f32::consts::TAU);
    state.render();
}
```

The chart code did not change. The animation code did not call
into the chart. Everything composes through `Stage` mutations
that the chart's renderer will pick up next frame — exactly the
boundary the [`Target` ticket (M-ANIM.5)](https://linear.app/harwood/project/screen-studio)
will formalise into a typed witness.

## Test invariants

Three load-bearing invariants are asserted in
[`crates/wisp-animation/src/tests.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/tests.rs)
— if any of these breaks, the whole architectural decision
breaks with it:

- **`fixed_driver_is_deterministic_across_1000_frames`** — two
  fixed drivers seeded identically produce equal samples for
  1000 frames, even when one is fed garbage `dt` and the other
  millisecond `dt`. Fixed mode ignores the caller's `dt` on
  purpose.
- **`pause_freezes_elapsed`** + **`time_scale_doubles_step`** +
  **`seek_jumps_clock_without_changing_playing_flag`** — every
  playback-state mutation does exactly what its docstring says.
- **`tick_allocates_nothing`** + **`sample_allocates_nothing`** —
  measured by a `#[global_allocator]` heap counter installed in
  the test module. Per-frame budgets matter; an allocation in
  `tick` would be a silent perf bug.
