# Performance + RAIL budget

[Google's RAIL model](https://web.dev/articles/rail) gives
animations **~10 ms / frame** of CPU work to stay at 60 fps with
headroom for the renderer and the host. `wisp-animation` is
built so hitting that budget at 1000+ active tweens is
unremarkable.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/many-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?animate=many" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: 144-ellipse grid each with its own Tween"></iframe>
</div>

The demo runs **144 active tweens** on a 12×12 ellipse grid,
each with a different phase. The hero is a static snapshot of
the wave pattern; the iframe is the live demo.

## What we protect against

```admonish important title="Three invariants give RAIL almost for free"
- **`sample(t)` is pure.** No clock reads, no I/O, no allocs.
  Asserted by `sample_allocates_nothing` in the unit tests.
- **`Driver::tick` is alloc-free.** A heap-counting global
  allocator measures the delta across 1000 ticks and asserts
  zero growth.
- **`BatchDriver::tick_scalars` reuses its staging buffer.**
  After the first tick, the `Vec<(NodeId, Property, f32)>`
  capacity covers any future tick at the same animation count.
  Same heap-counter test as above.
```

## Measured cost — `tests/perf.rs`

The perf microbench builds a stage with 1000 nodes, registers
1000 `Tween<f32>` against their alpha properties, warms up,
then ticks 10 frames and measures wall clock.

| Build | Per-frame cost (1000 tweens) | Budget |
|---|---|---|
| `cargo nextest run -p wisp-animation --test perf` (debug) | ~66 µs | 10 ms |
| Release (estimated, release is typically ~10× faster) | ~10 µs | 10 ms |

> The original `BatchDriver` had an O(N²) back-to-front dedup that
> ran at ~4.4 ms in debug on a fast Mac and **failed the budget at
> ~22 ms on a slower CI runner**. Switching the dedup to
> `sort_unstable_by` with an index tiebreaker (in-place pdqsort,
> alloc-free) was a ~67× win and restored both the budget and the
> no-alloc invariant. Algorithm details in [multi-animation](./multi-animation.md).

```admonish info title="Override the budget locally"
The default budget is 10 ms, sized for slow CI hardware. On
your laptop the bench should fly. To tighten:

```bash
WISP_ANIM_PERF_BUDGET_MS=2 cargo nextest run -p wisp-animation --test perf
```

If it ever trips in CI without code changes, file a regression
rather than relax the budget — perf regressions matter.
```

## What we *don't* do (yet)

```admonish warning title="Not implemented (deliberate)"
- **Multi-threaded sampling.** wasm32 is single-threaded, and
  per-frame cost is already <1 % of a frame budget at our
  target scale. Not worth the complexity.
- **GPU-side interpolation.** Sampling is cheap CPU lerp math.
  GPU offload would be a complexity multiplier for no measured
  win until scenes balloon past ~10k animated nodes.
- **Frame-time telemetry / drop-frame detection.** We assume
  the renderer + host loop produces deterministic frames. If
  you need per-frame timing, instrument the host's rAF
  callback — it's outside this crate's scope.
- **Per-render dirty-region invalidation.** `wisp::Renderer`
  redraws the whole stage every frame today. Fine at chart
  scale (<500 nodes); a future ticket if scene graphs grow
  larger.
```

## Profiling tips

```admonish tip title="When something feels slow"
1. Build with `--release`. Debug `cargo nextest` is ~10× slower
   than the host app's release build.
2. Drop into `dhat` or `tracy` — `wisp-animation` makes no
   syscalls, no thread sync; the bottleneck is always lerp
   math or the renderer.
3. Halve animation count to see if the cost scales linearly
   (it should). Anything super-linear is a bug in batching.
4. If a single animation costs more than 1 µs, the problem is
   probably in your `Animation::sample` (`O(N)` per-call inner
   loop?). The built-in primitives are all `O(1)` per sample.
```

## Test invariants

- `tests/perf.rs::one_thousand_tweens_fit_in_budget` — 1000
  tweens × 10 frames within the configured budget.
- `tests/perf.rs::batch_tick_allocates_nothing_across_1000_frames`
  — heap counter delta is zero.
- `tests/determinism.rs` — composite animations sample-equal
  across two identical runs.

Full source: [`crates/wisp-animation/tests/perf.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/tests/perf.rs).
