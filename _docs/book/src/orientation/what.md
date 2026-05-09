# What this is

A native screen recorder in the Screen Studio / OpenScreen lineage, built as
an all-Rust stack. Two parallel deliverables:

- **`wisp`** — a Pixi-equivalent 2D scene graph + filter chain library on
  `wgpu`. Pixi-shaped public API, scoped to power the recorder.
- **`screen-app`** — the Tauri 2 + Leptos recorder application that consumes
  `wisp`.

Library is means; the app is the goal.

## Why the Pixi shape

The recorder is a compositor. Compositor-shaped problems map naturally to a
scene-graph API: a `Stage` with `Container` children, sprites for video and
cursor, `Filter` chains for shadows / blur / color grading, `RenderTexture`
for captured frames. Pixi's API has been refined over a decade of compositor
use; copying its shape (translated to Rust + wgpu) skips a lot of trial.

`wisp` is a focused subset, not a port — only the parts the recorder needs.
