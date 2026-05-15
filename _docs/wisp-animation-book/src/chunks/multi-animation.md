# Multi-animation batching (`BatchDriver`)

When two animations write to the same `(NodeId, Property)`, the
deterministic answer is **last-wins** — and the read/write
phases need to be cleanly separated so the answer doesn't
depend on registration order or scheduler luck.

`BatchDriver` is the formal version of what every per-frame
mutator in `wisp-chart-web` does informally:

1. **Read phase.** Sample every active animation. Stage the
   resulting `(NodeId, Property, value, index)` tuples into a
   reused buffer. No stage mutations yet.
2. **Sort phase.** `sort_unstable_by` orders the buffer by
   `(NodeId, Property)` with the original registration index as
   a *descending* tiebreaker so the last-registered entry for
   each `(NodeId, Property)` pair ends up first in its run.
   pdqsort is in-place — no allocation.
3. **Write phase.** Walk the sorted buffer forward; emit the
   first entry of each `(NodeId, Property)` run; skip duplicates.
   One stage write per distinct property per frame.
4. **Render.** Caller invokes `Renderer::render_stage` exactly
   once.

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
Once you batch reads, conflict resolution becomes a `O(N log N)`
sort plus a linear walk; no need to track "who wrote when".
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
- One in-place `sort_unstable_by` over the staging buffer
  (`O(N log N)`, no auxiliary allocation — `sort_by` would
  allocate `O(N)` scratch and is deliberately avoided).
- One forward walk emitting the first entry of each
  `(NodeId, Property)` run (`O(N)`).
- At 1000 active tweens with all distinct targets the bench in
  [Performance](./performance.md) runs at **~66 µs / frame** in
  debug on a fast Mac — far inside RAIL's 10 ms budget.
- Zero allocations after the first tick (staging buffer is a
  reused `Vec` owned by `BatchDriver`).
```

```admonish info title="Why `sort_unstable_by` instead of `sort_by`"
Rust's stable `slice::sort_by` allocates `O(N)` scratch memory
every call — that would break the `batch_tick_allocates_nothing`
invariant. `sort_unstable_by` (pdqsort) is in-place. To recover
the stable-sort semantics needed for "last-wins by registration
order", `BatchDriver` packs the original index into each staged
tuple and uses it as a *descending* tiebreaker. After the sort,
the latest-registered entry per `(NodeId, Property)` pair sorts
first in its run; the linear walk takes that one.
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
