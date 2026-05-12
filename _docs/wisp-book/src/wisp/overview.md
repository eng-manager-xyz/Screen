# `wisp` — overview

`wisp` is a Pixi-equivalent 2D scene graph + filter chain on `wgpu`.

It exists to power the recorder. Pixi's API shape (Stage → Container → Sprite,
filters as a per-node chain, render-to-texture targets) maps cleanly to the
recorder's compositor needs: a video sprite under cursor effects, drop shadows
on the recording quad, filter chains for color grading, etc.

Every renderable feature ships with a story; every story is screenshotted
into `assets/wisp/<id>.png` and shows up in [Stories](./stories.md).

## Core types

- `Stage` — root scene container; owns the slotmap of nodes.
- `Container` / `Sprite` / `Graphics` / `Text` / `Mesh` — node types.
- `Transform` — position / rotation / scale; nested via parent/child.
- `Filter` — `BlurFilter`, `DropShadowFilter`, `MotionBlurFilter`,
  `ColorMatrixFilter`. Composable, applied at render.
- `Renderer` — orchestrates the scene → texture path; stat counts come back
  in `RenderStats { draw_calls, sprites_drawn, … }`.
- `RenderTexture` / `VideoTexture` — render targets; the latter for per-frame
  uploads (the path the recorder uses for capture frames).

For the full API see the [rustdoc index](../api.md).
