# `playback` — `Player` state machine

Owns a boxed `decode::VideoStream` plus a `wisp::VideoTexture`, advances
on `tick(dt)`, and uploads the next decoded frame to the GPU when the
wallclock catches up to its PTS. Idle / Loaded / Playing / Paused
lifecycle.

Library only — runnable via two examples that demonstrate the pipeline.

## Run locally

```bash
# from: repo root (the example writes its PNGs to a workspace-relative
#       path, so the cwd matters for `play_file`)

# Synthetic stream — no system deps. 7 PNGs of a moving gradient.
# Output: /tmp/timed_playback_NN.png
cargo run -p playback --example timed_playback

# Real MP4 → decode → upload to GPU → render through wisp → PNGs.
# Uses crates/decode/tests/fixtures/sample.mp4 (committed).
# Output: _docs/book/src/assets/playback/playfile_NN.png
cargo run -p playback --example play_file
```

`play_file` requires GStreamer on `$PATH` (see the `decode` README). The
`timed_playback` example uses `MockVideoStream` and runs anywhere.

## Test locally

```bash
# from: repo root (or anywhere inside the workspace)
cargo nextest run -p playback
cargo test -p playback --doc
```

The unit tests cover the state machine transitions and the per-tick
PTS-vs-wallclock decision; the integration test runs the full pipeline
against `MockVideoStream` so it stays hermetic.
