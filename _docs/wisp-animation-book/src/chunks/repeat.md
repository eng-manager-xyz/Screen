# Repeat + Reverse + Yoyo

`Repeat<A>` wraps any animation to cycle: finite `N` times,
infinitely, or until a wall-clock deadline. `RepeatStrategy`
picks the direction — restart from `0` each cycle (`Loop`) or
alternate forward/backward (`MirroredRepeat`, "yoyo").

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/repeat-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=polar&amp;animate=yoyo" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: polar plot scale-yoyo via infinite MirroredRepeat"></iframe>
</div>

The demo is a `Tween<f32>::new(0.6, 1.0, 600ms).ease(Ease::InOutCubic)`
wrapped with `repeat_with(Infinite, MirroredRepeat)`. The chart
oscillates between scales 0.6 and 1.0 forever.

## API surface

```rust,ignore
use std::time::Duration;
use wisp_animation::{Tween, Ease, AnimationRepeatExt, RepeatCount, RepeatStrategy};

let pulse = Tween::new(0.6_f32, 1.0, Duration::from_millis(600))
    .ease(Ease::InOutCubic)
    .repeat_with(RepeatCount::Infinite, RepeatStrategy::MirroredRepeat);

// Or just `.repeat(RepeatCount::Finite(3))` for default Loop strategy.
```

## Count + strategy

```admonish info
- `RepeatCount::Finite(n)` runs the animation `n + 1` times in
  total. `Finite(0)` is "play once, no repeat". Past the end,
  `sample` returns the animation's terminal value.
- `RepeatCount::Infinite` runs forever. `Repeat::duration()`
  reports `Duration::MAX` so drivers know there's no end.
- `RepeatCount::ForDuration(d)` runs for at most `d` of wall
  clock, regardless of where in the cycle that lands.
```

```admonish important title="Yoyo via MirroredRepeat"
`MirroredRepeat` plays the inner animation forward on even
cycles and backward on odd cycles. There's no "yoyo" knob — it's
just `RepeatStrategy::MirroredRepeat`, the name borrowed from
bevy_tweening because it's unambiguous about what's happening.
```

## Test invariants

- `Loop` resampling at `t = 1.5 × cycle` returns the inner
  animation at `0.5 × cycle` forward.
- `MirroredRepeat` resampling at `t = 1.5 × cycle` returns the
  inner animation at `0.5 × cycle` *backward* (i.e. inner.sample
  of `cycle - 0.5 × cycle`).
- `Finite(N)` clamps past-end samples to the terminal value;
  `Infinite` wraps; `ForDuration` reports the supplied cap.

Full source: [`crates/wisp-animation/src/repeat.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/repeat.rs).
