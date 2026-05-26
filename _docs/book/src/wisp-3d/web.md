# `wisp-3d-web` — Trunk-built wasm32 bundle

`crates/wisp-3d-web/` packages `wisp-3d` for the browser via wgpu's `BROWSER_WEBGPU` backend. Trunk drives the wasm-bindgen build; the output is a self-contained `dist/{index.html, *.js, *.wasm}` artefact.

## What it ships

A `#[wasm_bindgen(start)]` entry point that:

1. Picks the canvas via `document.querySelector("canvas[data-404-stage]")` — the same selector the engmanager.xyz `not-found.js` already renders into.
2. Sizes the canvas to its layout box × `device_pixel_ratio` (clamped to 2× to keep wasm fill rate reasonable).
3. Boots a `wgpu::Instance` with `Backends::BROWSER_WEBGPU` (no WebGL fallback — WebGPU only).
4. Wraps the instance via `wisp::Application::from_wgpu` so the `wisp-3d` render pipelines see a normal `Application`.
5. Builds `Mesh3D::pyramid(1.34, 1.25)` + `EdgesMesh::from_mesh(8°)`.
6. Composes one frame: `PaletteRampMaterial` on the pyramid + wireframe overlay.
7. `surface.present()`.

## Runbook

```bash
# Local dev (port 8082)
cd crates/wisp-3d-web && trunk serve

# Release build (the artefact the engmanager.xyz integration consumes)
cd crates/wisp-3d-web && trunk build --release
ls dist/
```

## Browser support matrix

| Browser | WebGPU support | Notes |
|---|---|---|
| Chrome / Edge ≥ 113 | yes | default on Win/macOS/Linux |
| Safari ≥ 18 | yes | macOS 14.2+ / iOS 17.4+ |
| Firefox | nightly | enable `dom.webgpu.enabled` in `about:config` |
| Older / no-WebGPU | no | host page must keep a Canvas2D fallback (engmanager.xyz does) |

## Bundle weight

The `data-wasm-opt="z"` Trunk attribute runs `wasm-opt -Oz` so the published `.wasm` lands in the ~600–800 KB range (brotli-compressed). That's competitive with the 580 KB minified `three.module.min.js` the engmanager.xyz page currently fetches from jsDelivr — and same-origin, so no third-party DNS / TLS handshake.

```admonish note title="The animation loop is host-page's job"
This bundle ships a single static draw. The full `requestAnimationFrame` loop + reduced-motion check lives in the engmanager.xyz host page (`not-found.js`) so the wasm bundle stays simple and the host page owns the per-frame lifecycle. See AUT-302 for the integration ticket.
```
