# wisp-animation

`wisp-animation` is the authored-motion layer for the
[`wisp`](/Screen/wisp/) renderer and the
[`wisp-chart`](/Screen/wisp-chart/) composition library. It is a
pure-Rust crate, wasm-clean by construction, and built around one
load-bearing architectural decision:

```admonish important title="Animation is a value; Driver owns the clock"
An `Animation` is a *pure function from time to state*. You ask it
for its duration, then `sample(t)` at any instant. The animation
holds no playback state — no `Instant`, no cursor, no internal
mutation. Time advances through a separate `Driver`.

That separation is what lets the **same animation value** run in
real-time (your `winit` loop) and in deterministic offline export
(`wisp-export-animated` writes byte-identical MP4s across runs).
Same value. Same `t`. Same output. Always.
```

## What's in v0.1

Today the crate exposes the foundational primitives — the
`Animation` trait, the `Driver`, and a `LinearRamp` placeholder
animation good enough to drive a chart's rotation. The full
roadmap lives in
[M-ANIM](https://linear.app/harwood/project/screen-studio) on
Linear; we ship one ticket at a time, each with its own test
suite, mdBook chapter, and a WebGPU demo that animates a real
[`wisp-chart`](/Screen/wisp-chart/) chart.

## Who this book is for

You're either:

- Building a chart that needs an entrance, exit, or data-update
  transition — read [Animation trait + Driver](./chunks/animation-trait-driver.md)
  and then watch this book grow.
- Designing a new animation primitive for `wisp-animation` itself —
  the same chapter shows the trait + driver contract you'll be
  implementing against.

## How to read each chapter

Every chapter follows the same shape:

1. **What this feature is** in one paragraph.
2. **A live WebGPU demo** (iframe pointed at the deployed chart
   demo, with the relevant `?animate=…` URL flag).
3. **A still-frame PNG** behind the iframe as a graceful fallback
   for readers offline or on a browser without WebGPU.
4. **Minimal Rust code** that produces that exact animation.
5. **Admonish callouts** for non-obvious gotchas.

If you can't see the live demo, the still frame tells you what the
output looks like; the code below tells you how to produce it.
