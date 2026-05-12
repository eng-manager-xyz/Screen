# Quickstart

Three minutes from `cargo add` to a textured quad on screen.

## 1. Add the dependency

```toml
[dependencies]
wisp = "0.1"
```

## 2. The smallest scene that draws

```rust
use wisp::prelude::*;

fn build_scene() -> Stage {
    let mut stage = Stage::new(StageOptions {
        size: (1920, 1080),
        clear_color: [0.0, 0.0, 0.0, 1.0],
    });
    let id = stage.spawn(Sprite {
        position: vec2(960.0, 540.0),
        size: vec2(800.0, 450.0),
        anchor: vec2(0.5, 0.5),
        texture: TextureRef::Solid([0.4, 0.6, 1.0, 1.0]),
        ..Default::default()
    });
    stage.tick(0.0);
    stage
}
```

## 3. Drive the frame loop

Pick a host:

- **Native window** via `winit` — wisp's
  [`Application::from_wgpu`](./wisp/overview.md) accepts a
  `wgpu::Device` + `wgpu::Surface` and renders straight to it.
- **Headless** — render into a
  [`RenderTexture`](./wisp/overview.md) and call `read_pixels`
  for PNG dumps or push the BGRA bytes into GStreamer
  `appsrc → vtenc_h264_hw → mp4mux → filesink` for MP4 output.

See [Headless export](./wisp/chunks/example-headless-export.md)
and [Recorder mock](./wisp/chunks/example-recorder-mock.md) for
both flows running end-to-end.

## 4. Add a filter

```rust
let id = stage.spawn(Container::default());
stage.attach_filter(id, BlurFilter::new(8.0));
stage.attach_filter(id, DropShadowFilter::new()
    .with_offset(vec2(8.0, 8.0))
    .with_blur(12.0));
```

Filters compose left-to-right. The [filter chain
example](./wisp/chunks/example-filter-chain.md) animates three
filters layered on one container.

## 5. Read the deep dive

- [Renderer overview](./wisp/overview.md) — `Container` / `Sprite`
  / `Graphics` / `Text`, the frame pump, `RenderTexture`.
- [Sprite batcher](./wisp/chunks/sprite-batcher.md) — how the
  renderer collapses N sprites into one draw call.
- [Filter chain](./wisp/chunks/example-filter-chain.md) — composing
  multiple post-process passes.
- [Text architecture](./wisp/text/architecture.md) — bitmap vs
  flexible text, glyph atlas, layout pipeline.
- [Mask system](./wisp/chunks/mask-texture.md) — dynamic mask
  textures, vector → mask bridging.

```admonish info title="Used in production"
wisp is the renderer behind [Screen Studio](/Screen/), a native
Rust screen recorder. If you want to see how an editor / capture
pipeline / encoder integrates with wisp, the screen project book
at [/Screen/](/Screen/) has the full integration story.
```
