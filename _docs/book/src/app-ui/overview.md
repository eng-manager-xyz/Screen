# `app-ui` — overview

The Leptos CSR app that becomes the recorder's HTML/UI layer. This is the
chunk where the components workshopped in
[`ui-storybook`](../ui/overview.md) leave their gallery and live in a real
shell.

## Architecture

```text
crates/app/                         crates/app-ui/                crates/ui-storybook/
─ Tauri shell (Rust binary)         ─ Leptos CSR app (WASM)       ─ Component library
─ tauri.conf.json                   ─ Trunk-built dist/             + SSR snapshot tests
─ frontendDist points at ──────►    ─ Mounts <App> to <body>        + visual gallery
  app-ui's dist/ (M-INT.2)          ─ Reuses ui-storybook's       (consumed as a normal
                                      DropZone, PlayerControls,    Cargo lib dep)
                                      RecordingToolbar, StatusBar,
                                      Card, DopeSheet
```

Three layers, three crates, one direction of dependency. Adding a new
component once means it becomes available in:

1. The storybook gallery (regression-tested via SSR + insta).
2. The shell app (consumed via `use ui_storybook::components::*`).
3. The mdBook chapter (assets regenerated via `just snapshots-ui`).

## Current state — M-INT.1

Pure UI composition, no Tauri IPC, no real file ingestion:

- `RecordingToolbar` at the top showing the idle state.
- `DropZone` (idle) as the main surface — clicking it flips a Leptos
  signal into the loaded view.
- `PlayerControls` (paused) + a placeholder gradient surface where the
  wisp render output will eventually go (M-PREVIEW.1).
- `StatusBar` (ready) at the bottom.

No real wiring yet — the click → loaded transition is a demo affordance
that lets reviewers exercise both views before the actual file-drop event
lands in M-INT.2.

## How to run

```bash
just app-ui          # dev server with hot reload, opens browser
just app-ui-build    # production build → crates/app-ui/dist/
```

`just app-ui-build` produces a static bundle that the Tauri shell will
serve verbatim once M-INT.2 wires `tauri.conf.json` to point at it.

## Roadmap

- ✅ **M-INT.1** — this chunk; Trunk builds, components render, gate green.
- ⏳ **M-INT.2** — Tauri serves the Trunk bundle; OS file-drop events
  flip the loaded signal.
- ⏳ **M-PREVIEW.1** — native winit sibling window with the wisp surface
  rendering the active video frame.
- ⏳ **M-PLAY.2** — Tauri↔player IPC for transport (load / play / pause /
  seek dispatched from Leptos signals to the native player loop).
