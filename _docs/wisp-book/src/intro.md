# wisp

A Pixi-shaped 2D scene graph + filter chain library on top of
[`wgpu`](https://wgpu.rs). Native Rust, no JavaScript, no DOM.

## What it gives you

- **Scene tree** — `Container` / `Sprite` / `Graphics` / `Text`,
  with transforms inherited from the parent.
- **Sprite batcher** — single draw call per atlas, scene-tree order
  preserved.
- **Filter chain** — composable post-process passes (blur, drop
  shadow, motion blur, color matrix), each with a `passes()` method
  so the renderer can allocate scratch `RenderTexture`s on demand.
- **Mask system** — rounded clip, privacy blur, solid redaction,
  spotlight, dim-outside, ellipse, freehand path. Works on raster
  AND vector primitives.
- **Text** — atlas-cached bitmap text for HUDs + a Cosmic-Text /
  Glyphon-backed `FlexibleText` for high-quality body copy.
- **Headless export** — render any scene to a `RenderTexture`,
  read pixels back as BGRA bytes, or push them straight into a
  GStreamer `appsrc` for video encode.

## What it costs you

- A wgpu adapter. Anything Metal / Vulkan / DX12 / GLES 3 capable.
- Rust 1.82+ (some examples use newer features).
- Awareness of wgpu's `Queue::submit` rhythm — wisp gives you the
  scene API but expects you to drive the frame loop.

## What it is not

- A game engine. There's no input handling, audio, networking,
  asset pipeline, or ECS.
- A web canvas library. wisp targets native; the web works via
  wgpu's WebGPU backend but is not a primary surface.
- A reactive UI framework. Wisp draws what you tell it to draw.

## Why it exists

It's the renderer behind [Screen Studio](/screen/) — a native Rust
screen recorder. The full project is in the `screen` monorepo; this
book is the renderer-only deep dive. If you want context on how the
recorder uses wisp, the project book at [/screen/](/screen/) has the
integration story.

[Get started](./quickstart.md) →
