# Spring physics

`Spring` is a 1-D damped harmonic oscillator implemented in
closed form: `sample(t)` is a pure analytical expression, not an
iterative integrator. Two flavours: critically-damped (fastest
settle, no overshoot — the UI default) and underdamped (springy
overshoot for affordance feedback).

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/spring-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=polar&amp;animate=spring" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: polar plot spring-scaling with overshoot"></iframe>
</div>

The demo uses `Spring::underdamped(70.0, 1.0, 0.4)` between
scales `0.4 → 1.0`, looping every 1.5s. The still frame catches
the overshoot peak at scale ≈ 1.08.

## API surface

```rust,ignore
use std::time::Duration;
use wisp_animation::{Animation, Spring};

// Critically-damped (no overshoot) — fastest settle.
let snap = Spring::critically_damped(170.0, 1.0).between(0.0, 1.0);

// Underdamped (with overshoot) — `ζ = 0.4` is moderately springy.
let bounce = Spring::underdamped(70.0, 1.0, 0.4).between(0.0, 1.0);

// UI-tuned default (k = 170, m = 1, critically damped).
let ui = Spring::ui_default();

// Sample at any t — Spring implements Animation<Output = f32>.
let v = snap.sample(Duration::from_millis(200));
```

## Closed-form vs integrator

```admonish important title="Pure function — no iterative state"
Damped springs have analytical solutions: critically-damped is
`(A + B·t) · e^(-omega_n · t)`; underdamped is a damped
sinusoid. Both are implemented directly. There's no `dt`
integrator, no per-frame internal velocity field, no time-step
sensitivity. Two callers sampling the same spring at the same
`t` get the same value — same architectural property as every
other `Animation` in this crate.
```

## Damping ratio + settle time

```admonish info title="Picking parameters"
- **Stiffness `k`** controls "speed of settle" — bigger `k`,
  faster motion.
- **Mass `m`** scales inertia — bigger `m`, slower motion.
- **Damping ratio `ζ = c / 2·sqrt(k·m)`** controls overshoot.
  `ζ ≥ 1` is critically/overdamped (no overshoot); `ζ < 1` is
  underdamped (overshoots).

`Spring::settling_duration()` returns the conservative
`5/ω_n` envelope (or `2·ln(5)/(ζ·ω_n)` underdamped). It's used
by `Animation::duration()` so drivers can ask "is this spring
done?".
```

## Test invariants

- `Spring::ui_default()` (k = 170, m = 1) reaches within 5% of
  the target within 2 seconds.
- `Spring::settling_duration()` is bounded and positive.
- `Spring::underdamped(...)` produces at least one sample value
  greater than the target endpoint (overshoot).
- `sample(Duration::ZERO)` equals `from` exactly.
- `Spring::critically_damped(k, m)` has `damping_ratio() == 1.0`.

Full source: [`crates/wisp-animation/src/spring.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/spring.rs).
