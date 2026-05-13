# `wisp-chart-web` — WebGPU browser demo for `wisp-chart`

> Same `wisp-chart` crate. Different surface — a browser `<canvas>`
> via WebGPU.

This crate exists to prove that `wisp-chart` (and therefore `wisp`
itself) compiles for `wasm32-unknown-unknown` and renders into a
`<canvas>` via wgpu's `BROWSER_WEBGPU` backend.

## What it does today

The bring-up demo: find a `<canvas id="wisp-chart-canvas">`, build
a `wgpu::Instance` with `BROWSER_WEBGPU`, configure the surface,
clear the canvas to white. That proves the WebGPU path is wired up.

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
