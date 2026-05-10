# Summary

[Introduction](./intro.md)

# Project orientation

- [What this is](./orientation/what.md)
- [Stack](./orientation/stack.md)
- [Theatre metaphor](./orientation/metaphor.md)

# Conventions

- [Workflow](./conventions/workflow.md)
- [Testing](./conventions/testing.md)
- [Documentation gate](./conventions/docs.md)
- [Story / screenshot pipeline](./conventions/screenshots.md)

# `wisp` — the wgpu renderer

- [Overview](./wisp/overview.md)
- [Stories](./wisp/stories.md)
  - [Textured quad — M0.6](./wisp/chunks/hello-quad.md)
  - [Nested transforms — M0.7 / M0.8](./wisp/chunks/transform-nesting.md)
  - [Sprite batcher — M0.9](./wisp/chunks/sprite-batcher.md)
  - [Rounded rect with stroke — M0.12 / M0.13](./wisp/chunks/graphics-rounded.md)
  - [Animated click ripple — M0.13](./wisp/chunks/graphics-ellipse.md)
  - [Gradient fills — M0.14](./wisp/chunks/graphics-gradients.md)
  - [Bitmap text — M0.15](./wisp/chunks/text-bitmap.md)
  - [Blur filter — M0.16](./wisp/chunks/filter-blur.md)
  - [Drop shadow — M0.17](./wisp/chunks/filter-drop-shadow.md)
  - [Motion blur — M0.18](./wisp/chunks/filter-motion-blur.md)
  - [Color matrix — M0.18](./wisp/chunks/filter-color-matrix.md)
  - [Perspective rotation — M0.19](./wisp/chunks/mesh-perspective.md)

# `decode` — video decode

- [Overview](./decode/overview.md)

# `playback` — player state machine

- [Overview](./playback/overview.md)
  - [Real MP4 → wisp playback (M-DEC.2)](./playback/play-file.md)

# `preview` — native window

- [Overview](./preview/overview.md)
  - [Native winit window (M-PREVIEW.1)](./preview/chunks/preview-window.md)

# `app-ui` — recorder shell

- [Overview](./app-ui/overview.md)
  - [Tauri ↔ Leptos integration (M-INT.2)](./app-ui/integration.md)
  - [Player IPC (M-PLAY.2)](./app-ui/player-ipc.md)
  - [Testing tiers (M-TEST.1 / .2)](./app-ui/testing.md)

# `ui-storybook` — Leptos UI

- [Overview](./ui/overview.md)
- [Components](./ui/components.md)
  - [Button — variants](./ui/chunks/button-variants.md)
  - [Button — sizes](./ui/chunks/button-sizes.md)
  - [Card — header + body](./ui/chunks/card-basic.md)
  - [Drop zone — idle](./ui/chunks/drop-zone-idle.md)
  - [Drop zone — active](./ui/chunks/drop-zone-active.md)
  - [Player — paused](./ui/chunks/player-controls-paused.md)
  - [Player — playing](./ui/chunks/player-controls-playing.md)
  - [Player — near end](./ui/chunks/player-controls-near-end.md)
  - [Recording toolbar — idle](./ui/chunks/recording-toolbar-idle.md)
  - [Recording toolbar — recording](./ui/chunks/recording-toolbar-recording.md)
  - [Recording toolbar — paused](./ui/chunks/recording-toolbar-paused.md)
  - [Status bar — ready](./ui/chunks/status-bar-ready.md)
  - [Status bar — encoding](./ui/chunks/status-bar-busy.md)
  - [Status bar — error](./ui/chunks/status-bar-error.md)
- [Dope sheet](./ui/dope-sheet.md)
  - [Multi-track](./ui/chunks/dope-sheet-basic.md)
  - [Dense keyframes](./ui/chunks/dope-sheet-dense.md)
  - [Editor panel composition](./ui/chunks/card-with-dope-sheet.md)
  - [Editor mock — full composition](./ui/chunks/editor-mock.md)

# Milestones

- [M0 — wisp renderer](./milestones/m0.md)
- [M1 — Tauri drop-zone](./milestones/m1.md)

# API reference

- [Rustdoc index](./api.md)
