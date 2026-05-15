# Deterministic export

The whole crate hangs on one architectural property: **same
animation value, same time `t`, same output bytes — always.**
That makes offline MP4 export through `wisp-export-animated` a
single function call rather than a "hope the scheduler doesn't
interfere" prayer.

This chapter is the contract page; the load-bearing tests live
at [`crates/wisp-animation/tests/determinism.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/tests/determinism.rs).

## The two halves of determinism

```admonish important title="Pure samples + driver-owned clock"
- `Animation::sample(&self, t: Duration)` is a **pure
  function**. No `Instant`. No hidden state. No allocation.
- `Driver` is the **only** place time accumulates. In
  `DriverMode::Fixed { dt }`, it advances by exactly `dt` per
  `tick`, ignoring the caller's argument — so garbage host dt
  can't pollute the integral.

Together: a fixed-mode driver seeded identically will produce
the same `elapsed` sequence, sampled at the same animation, the
same values. Forever.
```

## What the tests assert

```rust,ignore
// determinism.rs:
//   fixed_driver_is_deterministic_across_600_frames_with_complex_animation
//
// Two Driver::fixed(dt) instances, each ticked 600 frames with
// the SAME dt (but caller-arg dt fed garbage in one and the real
// value in the other to prove it's ignored). The composite
// animation is:
//
//   (Sequence::then::then).repeat_with(Infinite, MirroredRepeat)
//   + Spring::critically_damped(120, 1).
//
// Sampled into a Vec<f32> of length 600 in each run. The two
// vectors must be equal — strict equality, no epsilon. Verified
// across every commit that touches wisp-animation.
```

## How `wisp-export-animated` consumes this

```admonish info title="Frame-by-frame, fps-locked"
The export binary will (post-M-ANIM.18 future implementation)
construct a `Driver::fixed(Duration::from_secs_f32(1.0 / fps))`,
tick it once per frame, render the stage, push the texture
bytes into `gst-launch-1.0`'s `appsrc`. The fps is the same the
GStreamer pipeline expects; the driver's `tick` math is the
same regardless of platform; the resulting MP4 bytes are
identical (modulo the encoder itself, which we hash-pin in CI).
```

## Test invariants

- **Fixed-mode determinism** — two `Driver::fixed(dt)` drivers,
  ticked with garbage callers' dt, produce equal sample vectors
  across 600 frames of a complex composite animation.
- **Realtime determinism (when dt is constant)** — two
  `Driver::realtime()` drivers fed the *same* per-tick dt
  produce equal elapsed + equal samples. Realtime only diverges
  when the host dt diverges.
- **Curve determinism** — sampling the same `s` returns the
  same `Vec2`, regardless of call order.
- **Track determinism** — re-sampling at the same probe times
  yields the same values.

Full source: [`crates/wisp-animation/tests/determinism.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/tests/determinism.rs).
