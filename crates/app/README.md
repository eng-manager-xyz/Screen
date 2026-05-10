# `screen-app` — Tauri 2 shell

The native binary. Wires three things into Tauri's event loop:

1. **OS file drops** — `WindowEvent::DragDrop` → `file-dropped` event
   carrying the dropped path.
2. **Player IPC** — registers `PlayerSession` via `.manage()` and
   exposes `player_open` / `player_play` / `player_pause` /
   `player_status` commands.
3. **Tick thread** — ticks the player every ~33 ms and emits
   `player-status` events when state or elapsed time crosses a 100 ms
   boundary.

Frontend is `app-ui` (Leptos CSR), built by Trunk and served into the
webview.

## Run locally

```bash
# from: crates/app/
# `cargo tauri dev` reads tauri.conf.json from cwd; running it from
# elsewhere errors with "tauri.conf.json not found".
cd crates/app
cargo tauri dev
```

Requires the Tauri 2 CLI:

```bash
# from: anywhere
cargo install --locked tauri-cli --version "^2.0"
```

A native window titled **Screen** opens (1280×800, drag-drop enabled).
Drop any `.mp4` and the drop-zone view swaps for the player surface.
Wisp-rendered playback inside that surface is M-PLAY.2 (pending) — the
underlying decode pipeline already works headlessly via `cargo run -p
playback --example play_file`.

For a release build:

```bash
# from: crates/app/
cd crates/app
cargo tauri build
```

(Bundling is gated off in `tauri.conf.json` — `tauri build` produces the
binary but no installer.)

## Test locally

```bash
# from: repo root (or anywhere inside the workspace)

# IPC harness + player_session unit tests. Runs cross-platform via
# tauri's `mock_builder` (no real webview), so this is part of the gate.
cargo nextest run -p screen-app
cargo test -p screen-app --doc
```

End-to-end WebDriver tests (real binary, real webview) live in the
`app-e2e` crate — see `crates/app-e2e/README.md`.

## Notes

- **`icons/icon.png`** must exist at compile time even with bundling
  disabled — `tauri::generate_context!()` embeds it.
- **`tauri` is in `[package.metadata.cargo-machete] ignored`** because
  the dep is consumed at proc-macro expansion, which machete's static
  analysis doesn't see.
- **Tauri 2 on Linux** needs the gtk-rs build toolchain at
  `cargo check` time (not just link time):
  `apt install pkg-config libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev build-essential`.
