# Multi-animation batching (`BatchDriver`)

When two animations write to the same `(NodeId, Property)`, the
deterministic answer is **last-wins** — and the read/write
phases need to be cleanly separated so the answer doesn't
depend on registration order or scheduler luck.

`BatchDriver` is the formal version of what every per-frame
mutator in `wisp-chart-web` does informally:

1. **Read phase.** Sample every active animation. Stage the
   resulting `(NodeId, Property, value)` triples into a reused
   buffer. No stage mutations yet.
2. **Write phase.** Walk the staging buffer back-to-front,
   applying each triple. Skip entries whose
   `(NodeId, Property)` was already claimed by a later write.
3. **Render.** Caller invokes `Renderer::render_stage` exactly
   once. (Same shape as today.)

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/batched-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=polar&amp;animate=batched" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: BatchDriver writing alpha + rotation in one tick"></iframe>
</div>

The demo registers one alpha pulse + one rotation ramp on the
same polar plot through a single `BatchDriver`. Both land each
frame in one deterministic write phase; the hero captures the
chart mid-cycle at alpha ≈ 0.7, rotation ≈ 1.3 rad.

## API surface

```rust,ignore
use std::time::Duration;
use wisp_animation::{BatchDriver, BoundScalar, NodeProperty, Tween};

let mut driver = BatchDriver::realtime();
driver.play();

let alpha_fade = Tween::new(0.0_f32, 1.0, Duration::from_millis(500));
let rotation = LinearRamp::new(0.0, std::f32::consts::TAU, Duration::from_millis(2_000));

let mut anims = vec![
    BoundScalar::new(alpha_fade, NodeProperty::alpha(node)),
    BoundScalar::new(rotation, NodeProperty::rotation(node)),
];

// Each frame:
driver.tick_scalars(Duration::from_secs_f32(1.0 / 60.0), &mut anims, &mut stage);
```

## fastdom — the spiritual sibling

```admonish info title="Why this shape exists"
[fastdom](https://github.com/wilsonpage/fastdom) is a tiny JS
library that batches DOM reads and writes into separate
microtask queues to prevent forced synchronous layout
(`offsetWidth` after a style change triggers a full layout
flush; interleaving reads and writes is O(N²)).

wisp has no layout, so the specific perf cliff doesn't apply —
but the *architectural shape* is what gives us deterministic
last-wins semantics for free. Same pattern, different reason.
Once you batch reads, conflict resolution becomes a simple
back-to-front walk; no need to track "who wrote when."
```

## Last-wins semantics

```admonish important title="Order in `anims` decides the winner"
If you register two animations on `NodeProperty::alpha(node)`,
the **later one in the slice wins**. This is deterministic and
easy to override at registration time. If you want first-wins,
reverse the slice before passing it to `tick_scalars`.
```

## Per-frame cost

```admonish note
- One `Animation::sample` per registered animation (`O(N)` over
  active animations).
- One back-to-front walk with O(N²) inner dedup. In practice
  the inner walk early-exits because most animations target
  distinct properties. At 1000 active tweens with all distinct
  targets, the bench in [Performance](./performance.md) hits
  ~4 ms / frame on a debug-build CI runner — well inside RAIL's
  10 ms budget.
- Zero allocations after the first tick (staging buffer is a
  reused `Vec` owned by `BatchDriver`).
```

## Test invariants

- Two `BoundScalar` registered on the same `NodeProperty::alpha`
  with different sampled values → the **later registration**'s
  value lands in `Stage`.
- Different properties on the same node both land.
- Stale `NodeId` (post-destroy) writes silently no-op.
- 1000-frame batch tick allocates zero times (heap-counting
  global allocator in `tests/perf.rs`).

Full source: [`crates/wisp-animation/src/batch.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/batch.rs).
