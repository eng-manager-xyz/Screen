# Lifecycle callbacks + EventReader

`WithCallbacks<A>` wraps any animation and fires user-supplied
closures on `Started` / `Completed`. The same events also land in
a shared `EventReader` queue — drain it each frame for sync
loops that can't borrow closures.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/lifecycle-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=polar&amp;animate=callbacks" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: polar with on_complete flash + console events"></iframe>
</div>

The demo wraps an infinite spin with `with_callbacks(AnimId(42))
.with_reader(reader)`. Each cycle, the reader queues a
`Started` and a `Completed` event; the rAF loop drains them,
logs to the browser console, and flashes the chart's alpha to
`0.55` for 120ms on Completed. Open devtools to watch the events
fire.

## Combinator API

```rust,ignore
use wisp_animation::{AnimId, AnimationLifecycleExt, EventReader, LinearRamp};

let reader = EventReader::default();
let anim = LinearRamp::new(0.0, 1.0, std::time::Duration::from_millis(500))
    .with_callbacks(AnimId(7))
    .on_start(|| println!("started"))
    .on_complete(|| println!("done"))
    .with_reader(reader.clone());

// Sample as usual; callbacks fire automatically; events queue.
let _ = anim.sample(std::time::Duration::from_millis(300));
let evs = reader.drain();
```

## Why two APIs

```admonish important title="Closures + events serve different hosts"
- **Closures** (`.on_start(|| …)`, `.on_complete(|| …)`) are
  great for interactive hosts where the callback can borrow
  ambient state via `move ||`.
- **EventReader** is great for sync export loops that can't
  borrow into closures (`wisp-export-animated` runs frames as
  a loop body — no per-frame closure to attach to). Drain the
  reader after each `Driver::tick`, dispatch from the loop body.

Both surfaces emit the same three events. Pick whichever shape
your host prefers; mix freely.
```

## Determinism

```admonish info title="Events fire on `sample`, not on wall clock"
A `WithCallbacks` wrapper only emits an event when the `t`
passed to `sample` crosses a threshold (0 → Started, ≥duration
→ Completed). Two `DriverMode::Fixed` drivers will emit the
same events at the same `t`. This is what makes the lifecycle
shape compatible with byte-identical MP4 export.
```

## Test invariants

- `Started` fires exactly once even when sampled many times.
- `Completed` fires exactly once at `t ≥ duration`.
- Drains return events in queue order.
- Closure callbacks and event reader fire from the same `sample`
  call.

Full source: [`crates/wisp-animation/src/lifecycle.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/lifecycle.rs).
