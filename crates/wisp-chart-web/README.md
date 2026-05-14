# `wisp-chart-web` — WebGPU browser demo for `wisp-chart`

> Same `wisp-chart` crate. Different surface — a browser `<canvas>`
> via WebGPU.

This crate exists to prove that `wisp-chart` (and therefore `wisp`
itself) compiles for `wasm32-unknown-unknown` and renders into a
`<canvas>` via wgpu's `BROWSER_WEBGPU` backend.

## What it does today

The bring-up demo: find a `<canvas id="wisp-chart-canvas">`, build
a `wgpu::Instance` with `BROWSER_WEBGPU`, configure the surface,
clear the canvas to a deliberate vivid purple. The purple is the
smoke signal — anyone reloading the page can tell at a glance that
WebGPU committed pixels (grey = bug, purple = working). That
proves the WebGPU path is wired up.

The real chart rendering arrives when M-CHART.0 chunk 3 lands the
`Gantt::render(&Theme) -> SceneNode` API and `wisp`'s renderer
module gets a wasm-clean draw entrypoint.

## Build / run

```bash
# Local dev with live reload.
just dev-wisp-chart-demo

# Release artefact in ./dist/.
cargo install --locked trunk
cd crates/wisp-chart-web
trunk build --release
```

CI builds the same artefact with `--public-url /Screen/wisp-chart/demo/`
and mounts it at the deployed Pages site.

## Browser support

- ✅ **Chrome / Edge 113+** — WebGPU on by default.
- ✅ **Firefox 121+** — WebGPU on by default on Linux / macOS / Windows.
- ⚠️ **Safari** — WebGPU shipping pending; Technology Preview only.

WebGL fallback is deliberately NOT enabled — this demo is WebGPU-only.

## Tests

Two layered tests guard the WebGPU render path; the chunk-3 commit
only proved `cargo check` and that's how the initial grey-canvas
regression slipped past review.

- **`tests/clear_pass.rs` — native, always-on.** Runs in
  `just gate` on every OS. Calls `clear_to_white` against an
  offscreen `RenderTexture` on whichever backend wgpu picks (Metal,
  Vulkan / lavapipe, DX12) and asserts every pixel is opaque white.
  Catches "the render code wrote wrong pixels" regressions.
- **`tests/headless_webgpu.rs` — headless Chrome, local-only.**
  Validates the BROWSER_WEBGPU **surface-presentation** path —
  the one a native readback cannot exercise. Not wired into
  `just gate`; run it locally before shipping any change that
  touches `web.rs`, surface configuration, or the canvas init:

  ```bash
  brew install chromedriver  # one-time
  # Chrome 124+ recommended for headless WebGPU on macOS.
  WASM_BINDGEN_TEST_TIMEOUT=60 \
    cargo test --target wasm32-unknown-unknown -p wisp-chart-web
  ```

  The test creates a `<canvas>`, configures a wgpu surface with
  `RENDER_ATTACHMENT | COPY_SRC`, runs `clear_to_white`, copies the
  surface texture to a buffer, and asserts the centre pixel is
  `[255, 255, 255, 255]`. Failure modes the test catches that
  `cargo check` doesn't: surface configure validation errors,
  `CompositeAlphaMode` resolving to a non-opaque mode, surface
  format / clear-colour mismatches, `get_current_texture`
  returning a stale view.

## Why a separate crate

`wisp-chart` is the library; `wisp-chart-web` is the
browser-flavoured consumer. Keeping them separate means:

1. **No wasm-bindgen leakage.** `wisp-chart`'s Cargo graph stays
   clean. Native callers don't pull in `js-sys` / `web-sys`.
2. **`crate-type = ["cdylib", "rlib"]` here only.** The library
   stays a plain rlib.
3. **Trunk configuration is co-located** with the HTML it
   bundles. No conditional config in `wisp-chart`.

## License

MIT.
