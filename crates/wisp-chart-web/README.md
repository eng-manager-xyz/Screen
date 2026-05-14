# `wisp-chart-web` — WebGPU browser demo for `wisp-chart`

> Same `wisp-chart` crate. Different surface — a browser `<canvas>`
> via WebGPU.

This crate exists to prove that `wisp-chart` (and therefore `wisp`
itself) compiles for `wasm32-unknown-unknown` and renders into a
`<canvas>` via wgpu's `BROWSER_WEBGPU` backend.

## What it does today

Renders [`sample_gantt`](src/lib.rs) — a small Gantt fixture with
four milestone rows (M-VEC, M-DYN, M-TEXT, M-BOOL) across 2026 —
into the canvas via `wgpu::Backends::BROWSER_WEBGPU`. The page-load
flow: find `<canvas id="wisp-chart-canvas">`, build a `wgpu::Instance`
+ canvas-backed surface, hand the wgpu context to
`wisp::Application::from_wgpu`, build a `wisp::Renderer`, ask
`wisp_chart::Gantt::emit_graphics` for a `wisp::Graphics` subtree,
add it to the stage, and `render_stage` it onto the surface
texture. End-to-end exercise of `wisp-chart` → `wisp` →
BROWSER_WEBGPU. Text labels, grid lines, and the header band are
the next chunks.

![Gantt demo render](../../_docs/wisp-chart-book/src/assets/wisp-chart-web/gantt-demo.png)

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

- **`tests/render_gantt.rs` — native, always-on.** Runs in
  `just gate` on every OS. Builds the same fixture as the demo,
  runs the full `Application` + `Renderer` + `Gantt::emit_graphics`
  pipeline against an offscreen `Rgba8Unorm` `RenderTexture` on
  whichever wgpu backend the host exposes (Metal / Vulkan / DX12),
  and asserts:
  1. A header-region pixel reads as `theme.bg` (white) — the
     background primitive painted.
  2. Centre pixel of `bar[0]` reads as Matt's `#0072b2` ± 2 — the
     layout math landed the bar where expected AND the explicit
     `PersonMap` override resolved.
  3. Centre pixel of `bar[2]` reads as Alice's `#d55e00` ± 2.

  The test also regenerates
  `_docs/wisp-chart-book/src/assets/wisp-chart-web/gantt-demo.png`
  — committed alongside the test as the PR-visible proof that
  it asserted on real rendered pixels.
- **`tests/headless_webgpu.rs` — headless Chrome, local-only.**
  Validates the BROWSER_WEBGPU **surface-presentation** path —
  the one a native readback cannot exercise. Same render path,
  same fixture, same `bar[0]` centre-pixel assertion (format-aware
  Bgra ↔ Rgba byte swap). Not wired into `just gate`; run locally
  before shipping any change that touches `web.rs`, surface
  configuration, or the canvas init:

  ```bash
  brew install chromedriver  # one-time
  # Chrome 124+ recommended for headless WebGPU on macOS.
  WASM_BINDGEN_TEST_TIMEOUT=60 \
    cargo test --target wasm32-unknown-unknown -p wisp-chart-web
  ```

  Failure modes the test catches that `cargo check` doesn't:
  surface configure validation errors, `CompositeAlphaMode`
  resolving to a non-opaque mode, surface format / clear-colour
  mismatches, `get_current_texture` returning a stale view, and
  any future regression in the `wisp::Renderer` pipelines under
  BROWSER_WEBGPU.

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
