# `preview` — native winit + wgpu window driven by `wisp`

A standalone binary that opens a winit window, hands its wgpu surface to
`wisp::Application::from_wgpu`, and plays an MP4 end-to-end through the
`decode` → `playback` → `wisp` stack. Proves the embedding seam that
`screen-app` will use for its sibling preview window (M-PREVIEW.x).

## Run locally

```bash
# from: repo root (the no-arg form looks up the fixture at
#       crates/decode/tests/fixtures/sample.mp4 relative to cwd)

# No args — uses the committed fixture so it just works.
cargo run -p preview

# With a custom file (path resolved relative to cwd).
cargo run -p preview -- path/to/your.mp4

# Release build for smoother playback.
cargo run -p preview --release -- path/to/your.mp4
```

Requires GStreamer on `$PATH` for any real MP4 (see the `decode`
README). The window aspect-fits the video into its current size; resize
to letterbox/pillarbox.

Press the OS window-close button to quit. There is no other UI yet.

## Test locally

```bash
# from: repo root (or anywhere inside the workspace)
cargo nextest run -p preview
cargo test -p preview --doc
```

`tests/render_smoke.rs` exercises the headless render path
(`examples/render_offscreen.rs`) so the surface-less branch keeps
working without needing a real display server in CI.

You can also run the headless example directly:

```bash
# from: repo root (or anywhere inside the workspace)
cargo run -p preview --example render_offscreen
```
