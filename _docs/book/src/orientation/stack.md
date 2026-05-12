# Stack

| Layer | Choice |
|---|---|
| Shell | Tauri 2 (multi-window) |
| UI | Leptos 0.8 (Rust → WASM) inside the Tauri webview |
| Renderer | `wisp` (this repo) — wgpu + WGSL |
| Editor preview | native `winit` sibling window rendered by `wisp` |
| Capture | `objc2`/ScreenCaptureKit (macOS), `windows-rs` (Windows), `pipewire-rs` (Linux) |
| Encode | `ffmpeg-next` for MVP; VideoToolbox / Media Foundation HW paths in v2 |

Locked 2026-05-09. Stack changes require an entry in `_docs/ISSUES.md`.

## Rust toolchain

Nightly (see `rust-toolchain.toml`). Edition 2024.

## Workspace layout

```text
screen/
├─ crates/                  # every workspace member lives here
│  ├─ wisp/                 # the renderer (wgpu + WGSL)
│  ├─ wisp-storybook/       # wgpu story gallery (eframe)
│  ├─ ui-storybook/         # Leptos UI gallery (SSR + Trunk)
│  ├─ app-ui/               # Leptos CSR shell (WASM)
│  ├─ app/                  # Tauri 2 binary (wraps app-ui)
│  ├─ media/                # GStreamer-backed audio + video
│  ├─ decode/               # BGRA frame contract + decoders
│  ├─ playback/             # Player state machine
│  └─ preview/              # native winit preview window
├─ _docs/
│  └─ book/                 # mdBook prose site (this site)
├─ Justfile                 # all QA recipes (`just gate`, etc.)
└─ deny.toml                # supply-chain policy
```

```admonish info title="Why so many crates"
Each crate is independently consumable. The renderer (`wisp`) is the
load-bearing one — every other crate either feeds it data (`media`,
`decode`, `playback`) or wraps it for a host (`app`, `preview`). An
embedder taking just `wisp` doesn't pull GStreamer; an embedder
taking just `media` doesn't pull `wgpu`.
```
