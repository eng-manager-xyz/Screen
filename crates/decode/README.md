# `decode` — video decode → BGRA frames

Codec-agnostic seam between video files and `wisp`. Defines the
`VideoStream` trait and ships two implementations:

- `MockVideoStream` — synthetic gradients, no system deps. Useful in
  tests + as a placeholder.
- `GstreamerPipeStream` — spawns `gst-launch-1.0` as a subprocess and
  reads BGRA frames off stdout. Decodes any file GStreamer can handle
  (MP4/H.264 today via `gstreamer1.0-libav`).

Pure library — no binaries, no examples. Consumers: `playback`,
`preview`, `wisp` (`playback_demo` example), `screen-app`.

## Run locally

There's nothing to run directly — drive it through the consumers:

```bash
# from: repo root (or anywhere inside the workspace)

# Headless decode → wisp render → PNGs.
cargo run -p playback --example play_file

# Native winit window decoding & playing the fixture MP4.
cargo run -p preview
```

## Test locally

```bash
# from: repo root (or anywhere inside the workspace)

# Unit + integration tests.
cargo nextest run -p decode

# Doctests.
cargo test -p decode --doc
```

The integration suite (`tests/gstreamer_integration.rs`) decodes the
committed fixture at `tests/fixtures/sample.mp4` (11 KB H.264). It
**requires GStreamer on `$PATH`**:

```bash
# from: anywhere — system package managers
brew install gstreamer
# or: apt install gstreamer1.0-tools gstreamer1.0-plugins-base \
#                  gstreamer1.0-plugins-good gstreamer1.0-libav
```

`gstreamer1.0-libav` is the one that carries the H.264 plugin on stock
Ubuntu — without it the fixture won't decode.
