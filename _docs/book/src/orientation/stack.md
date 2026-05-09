# Stack

| Layer | Choice |
|---|---|
| Shell | Tauri 2 (multi-window) |
| UI | Leptos 0.7 (Rust → WASM) inside the Tauri webview |
| Renderer | `wisp` (this repo) — wgpu + WGSL |
| Editor preview | native `winit` sibling window rendered by `wisp` |
| Capture | `objc2`/ScreenCaptureKit (macOS), `windows-rs` (Windows), `pipewire-rs` (Linux) |
| Encode | `ffmpeg-next` for MVP; VideoToolbox / Media Foundation HW paths in v2 |

Locked 2026-05-09. Stack changes require an entry in `_docs/ISSUES.md`.

## Rust toolchain

Nightly (see `rust-toolchain.toml`). Edition 2024.

## Workspace layout

```
screen/
├─ crates/
│  ├─ wisp/                 # the renderer
│  ├─ wisp-storybook/       # wgpu story gallery (eframe)
│  ├─ ui-storybook/         # Leptos UI gallery (SSR + Trunk)
│  └─ app/                  # Tauri 2 shell
├─ _docs/
│  └─ book/                 # mdBook prose site
├─ Justfile                 # all QA recipes
└─ deny.toml                # supply-chain policy
```
