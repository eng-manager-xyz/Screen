# `decode` — overview

The decoder side of the recorder. Turns video sources (MP4 files,
ScreenCaptureKit captures, …) into a stream of BGRA frames that wisp's
`VideoTexture::upload_bgra` consumes directly.

## Why a trait

The recorder uses **GStreamer** as the single media stack (see
[`stack`](../orientation/stack.md) — and AUT-144 for the
"no ffmpeg-next" decision). Multiple implementations still hide
behind the trait:

- **`gstreamer_pipe`** — CLI-subprocess via `gst-launch-1.0`. Ships
  today; powers both the decode integration tests and the playback
  player.
- **`gstreamer-rs` Rust bindings** (future, encode-side) — needed
  for the `appsrc`-fed encoder pipeline where wisp pushes BGRA
  frames into the encoder in-process.
- **`MockVideoStream`** — deterministic synthesized frames; no
  external deps. Used by `playback_demo` and the wisp story
  harnesses.

Each backend is a non-trivial integration, but the *consumer* —
wisp's per-frame upload path — is uniform: it wants `Vec<u8>` BGRA
at known dimensions, ticked at known timestamps.
[`VideoStream`](../api/decode/trait.VideoStream.html) is that
uniform contract.

## Current state

- ✅ **M-DEC.1** — trait + `MockVideoStream` (synthesizes scrolling
  gradient, no external deps, drives the `playback_demo` example).
- ✅ **M-DEC.2** — real MP4 decode via GStreamer CLI-subprocess
  (`gstreamer_pipe`).

## End-to-end proof point

The `playback_demo` example pulls 8 frames from `MockVideoStream`,
uploads each through `VideoTexture::upload_bgra`, renders via wisp's
`Sprite` pipeline, and writes the result to disk.

| Frame | Asset |
|---|---|
| 0 | ![](../assets/decode/frame_00.png) |
| 1 | ![](../assets/decode/frame_01.png) |
| 2 | ![](../assets/decode/frame_02.png) |
| 3 | ![](../assets/decode/frame_03.png) |
| 4 | ![](../assets/decode/frame_04.png) |
| 5 | ![](../assets/decode/frame_05.png) |
| 6 | ![](../assets/decode/frame_06.png) |
| 7 | ![](../assets/decode/frame_07.png) |

The motion is the gradient phase-shifting frame to frame — proves the
per-frame upload path actually replaces the texture each tick (a static
output would be a regression).

Run with:

```
cargo run -p wisp --example playback_demo
```

[Decode API ref](../api/decode/index.html)
