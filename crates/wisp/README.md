# `wisp` — 2D scene graph + filter chain on wgpu

Pixi-shaped public API: `Stage`, `Container`, `Sprite`, `Graphics`, `Text`,
`Mesh`, `Transform`, `RenderTexture`, `VideoTexture`, plus `BlurFilter`,
`DropShadowFilter`, `MotionBlurFilter`, `ColorMatrixFilter`.

This crate is the renderer. It owns no window — embedders supply a wgpu
device and surface (or use `Application::headless` for offscreen).

## Run locally

`wisp` is a library, but ships **8 examples** that exercise the public API
end-to-end.

```bash
# from: repo root (or anywhere inside the workspace — `-p` resolves the crate)
cargo run -p wisp --example hello_triangle
cargo run -p wisp --example hello_quad
cargo run -p wisp --example hello_sprite
cargo run -p wisp --example video_texture
cargo run -p wisp --example playback_demo      # decode → VideoTexture → render
cargo run -p wisp --example recorder_mock
cargo run -p wisp --example headless_export
cargo run -p wisp --example adapter_info       # prints the chosen wgpu adapter
```

Most examples are headless (write a PNG to `/tmp/` or print to stdout).
For the interactive feature gallery, see the `wisp-storybook` crate
(`just storybook`).

## Test locally

```bash
# from: repo root (or anywhere inside the workspace)

# Unit + integration tests (lib + tests/).
cargo nextest run -p wisp

# Doctests (run separately — nextest doesn't pick them up).
cargo test -p wisp --doc

# Lint + format check, scoped to this crate.
cargo clippy -p wisp --all-targets -- -D warnings
cargo fmt -p wisp -- --check
```

The full workspace gate (`just gate` from the repo root) runs the same
five steps across every crate.

## Notes

- **Linux software-Vulkan (lavapipe) limitation:** filter pipelines with
  multiple bind groups lose the device on lavapipe. CI sets
  `WISP_SKIP_GPU_FILTER_TESTS=1` to skip the affected tests; real
  hardware (Metal, hardware Vulkan, D3D) runs them all. Set the env var
  locally only if you're hitting the same software-adapter path.
- **`insta` snapshot first run:** the first run for a new snapshot writes
  `*.snap.new` and fails. Accept with `cargo insta accept` (or `mv`).
