# Decay (inertia glide)

`Decay` is the "fling-and-settle" primitive. Given a starting
position and an initial velocity, it produces an exponential
approach to an asymptote: position grows fast, then slowly,
then asymptotes near `from + velocity · τ`. Useful for kinetic
scroll, drag-throw, momentum-based snap.

## API surface

```rust,ignore
use std::time::Duration;
use wisp_animation::{Animation, Decay};

let glide = Decay::new(
    /* from */ 0.0,
    /* initial_velocity (units/sec) */ 200.0,
    /* time_constant (seconds) */ 0.4,
);

let asymptote = glide.predict_target(); // 200 * 0.4 = 80.0
let mid = glide.sample(Duration::from_millis(200));
```

## Closed-form, like Spring

```admonish info title="Pure function — `from + v·τ·(1 - e^{-t/τ})`"
Decay shares the architectural shape of `Spring`: closed-form
analytical sample, no integrator, no per-frame hidden state.
Two callers sampling the same `Decay` at the same `t` get the
same value. Same composition guarantees, same export
determinism.
```

## Picking τ

```admonish tip
- **Short τ (≈100 ms)** — snappy, like dismissing a sheet.
- **Medium τ (≈400 ms)** — kinetic scroll feel.
- **Long τ (≈1 s)** — slow, considered glide.

`Decay::duration()` returns `5·τ` — past that point the
position is within 1% of the asymptote.
```

## Test invariants

- `sample(Duration::ZERO) == from`.
- `predict_target` matches `sample(5·τ)` within 1%.
- `initial_velocity = 0` is constant (no motion).
- `duration()` returns `5·τ`.

Full source: [`crates/wisp-animation/src/decay.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/decay.rs).
