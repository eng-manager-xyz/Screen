# Headless export — M0.21

![headless export · frame 30 of 60 at 1080p](../../assets/wisp/example-headless-export.png)

The M0.21 closing proof point: **60 frames at 1920×1080**, fully
headless, dumped as PNGs. This is the path the M-EXPORT pipeline
inherits — render the project file's scene graph for each frame's
timeline tick, dump pixels, push BGRA into GStreamer's
`appsrc → vtenc_h264_hw → mp4mux → filesink` (or its
`mfh264enc` / `vaapih264enc` / `x264enc` platform variants).

The example renders the same recorder-mock-shaped scene but animates it:

- Recording quad rotates ±8° (one cycle over 60 frames).
- Cursor sprite oscillates horizontally on a Lissajous-style path.
- Text label scale pulses 1.0 → 1.15 → 1.0.

```bash
cargo run -p wisp --example headless_export
# Outputs:
#   target/headless_export/frame_00.png … frame_59.png   (60 frames)
#   _docs/book/src/assets/wisp/example-headless-export.png  (highlight)
```

The highlight above is frame 30 — mid-animation peak. Sixty 1080p PNGs
total ~30 MB on disk; compositing them into a real MP4 is M2+ scope
when the encode crate lands.

## What this rules out (and rules in)

- **Rules in:** wisp can render a non-trivial scene (4 layers, 60+ glyphs)
  at 1080p in milliseconds per frame. The export pipeline isn't
  GPU-bound for typical recording scenes.
- **Rules in:** the `RenderTexture` → `read_pixels` path is allocation-
  free per frame after warmup (the texture pool persists across frames).
- **Rules out (deferred to M-EXPORT):** real per-frame timeline data
  (currently parameters are computed from `frame / 60.0`); the export
  format negotiation (we write PNG-per-frame; production pipeline
  pushes raw BGRA into GStreamer's `appsrc` and converts to `yuv420p`
  inline with `videoconvert ! video/x-raw,format=I420`).

## GStreamer is the single media stack

The M0.21 spec originally called for `examples/video_texture.rs` to
"loop an MP4 decoded via `ffmpeg-next`." During M-DEC the project
locked in GStreamer as the only media library (see [AUT-144](https://linear.app/harwood/issue/AUT-144)
and the [stack reference](../../orientation/stack.md#stack)); the
equivalent decode path now lives at
[`crates/playback/examples/play_file.rs`](../../playback/play-file.md).
The headless side that M0.21 cared about — render-loop-to-PNG — is
exactly what `headless_export` ships, and the eventual encode side
will push the same BGRA frames into a GStreamer `appsrc` pipeline.

[`Renderer::render_stage`](../../api/wisp/render/struct.Renderer.html#method.render_stage) ·
[`RenderTexture::read_pixels`](../../api/wisp/struct.RenderTexture.html#method.read_pixels) ·
[Playback overview](../../playback/overview.md)
