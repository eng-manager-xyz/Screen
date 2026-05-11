# `media` — architecture — M-MEDIA.0 / AUT-96

The `media` crate is the home for GStreamer-backed audio + video capture,
playback orchestration, and the data models that `wisp` (the renderer)
and `app` (the Tauri/Leptos shell) consume. It's the M-MEDIA track's
foundation — every subsequent ticket (M-MEDIA.1 through M-MEDIA.22)
adds to one of its modules.

[api](../api/media/index.html)

## Three-way responsibility split

Boundaries are load-bearing. Crossing them once would bloat every wisp
consumer (storybook, headless export, future plugins) with GStreamer's
build footprint + license obligations.

| Crate | Owns | Doesn't own |
|---|---|---|
| `media` | GStreamer capture, GStreamer playback, audio + video data models, `MediaClock` / `MediaTime`, audio histogram quantization, recording-session manifest, device enumeration | rendering, UI |
| `wisp` | visual composition — sprite + graphics + text + mask + filter + blend pipelines | media capture, timing |
| `app` (Tauri + Leptos) | UI orchestration — webview, IPC, file dialogs, recorder commands | direct GStreamer or wgpu calls |

**`wisp` must not depend on `media`'s GStreamer integration.** Wisp
receives `VideoFrame` / `Texture` handles, `WaveformBarRect` geometry,
cursor state, and timeline timestamps via typed structs, and renders
them through its existing sprite + graphics pipelines.

## Layering

```text
 ┌──────────────────────────────────────────────┐
 │ app  (Tauri shell + Leptos UI)              │
 │   - calls `media::commands::*`              │
 │   - feeds `wisp` typed data                 │
 └────────────┬──────────────────┬──────────────┘
              │                  │
    ┌─────────▼────────┐   ┌─────▼────────────┐
    │      media       │   │      wisp        │
    │  - GStreamer     │   │  - render        │
    │  - timing model  │   │  - sprite/graphics│
    │  - histogram     │   │  - text + mask   │
    │  - manifest      │   │  - filter + blend│
    └────────┬─────────┘   └──────────────────┘
             │      typed data (VideoFrame,
             │      WaveformBarRect, MediaTime, …)
             ▼
        (no direct dep on wisp)
```

## Build-on-`decode`

The `decode` crate already carries the BGRA-frame contract used
throughout the project (`VideoFrame`, `VideoStream`, and the existing
`GstreamerPipeStream` CLI-pipe pattern). `media` builds on top of it —
re-exports `VideoFrame` / `VideoStream` under [`video`](../api/media/video/index.html)
and consumes the CLI-pipe pattern in M-MEDIA.6 / .13 / .16 for video
capture and webcam intake.

## GStreamer integration choice — CLI-pipe

Spawn `gst-launch-1.0` as a child process and pipe raw bytes through
`fdsrc` / `fdsink`. **Not `gstreamer-rs`**.

- Zero compile-time dependency on libgstreamer. Works on any machine
  with `brew install gstreamer` / `apt install gstreamer1.0-tools`, no
  `gst-build` setup needed.
- The CLI pipeline doubles as runnable documentation — you can paste it
  into a terminal.
- Upgrading to `gstreamer-rs` later is a one-line swap at the call
  site, because the public surface (`VideoStream`, `AudioStream` trait +
  chunk types) hides the transport.

Lessons captured in CLAUDE.md and the GStreamer-integration project
memory: `fdsink fd=1` for stdout, `rawvideoparse` before `mp4mux` to
synthesize PTS, drop-kill the child on shutdown, skip-guard every
integration test, include `PATH` in spawn errors.

## Module index

| Module | Chunk | Status |
|---|---|---|
| [`gstreamer`](../api/media/gstreamer/index.html) | M-MEDIA.1 (AUT-97) | scaffolded |
| [`clock`](../api/media/clock/index.html) | M-MEDIA.2 (AUT-98) | scaffolded |
| [`audio`](../api/media/audio/index.html) | M-MEDIA.3 (AUT-99) | scaffolded |
| [`video`](../api/media/video/index.html) | re-export of `decode` | done |
| [`histogram`](../api/media/histogram/index.html) | M-MEDIA.8 (AUT-104) | scaffolded |
| [`manifest`](../api/media/manifest/index.html) | M-MEDIA.20 (AUT-116) | scaffolded |

Every cell marked "scaffolded" is a module that exists today, compiles,
and contains the planned-surface comment that the next chunk converts
into real types + tests + an mdBook chapter of its own.

## Track sequencing

The 23 M-MEDIA chunks land on the `m-media` branch as one big PR.
Order is numeric and follows the dependency chain:

- **P0** (`AUT-96..103`) — crate + probe + clock + audio model + mock
  sources + GStreamer capture (audio + video) + A/V sync harness.
- **P1** (`AUT-104..110`) — histogram → waveform → Wisp render → gst
  histogram → texture handoff → gst video → synced scene.
- **P2** (`AUT-111..117`) — live mic / webcam / playback harness /
  cursor / device enumeration / manifest / Leptos seam.
- **P3** (`AUT-118`) — end-to-end smoke.

## Done when

- [x] `crates/media` compiles.
- [x] Module-level docs explain the media/Wisp/UI split.
- [x] Builds on `decode` via re-export of `VideoFrame` + `VideoStream`.
- [x] mdBook chapter (this page).
- [x] PROGRESS entry.
- [x] `just gate` green.
