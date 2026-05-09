# Milestone 1: Drop Zone + Video Player

> **Goal:** thinnest possible end-to-end Tauri+Leptos app — drop an MP4, play it back. No `wisp` integration, no capture, no encode. Just the shell.
>
> **Why this milestone:** validates the Tauri 2 + Leptos toolchain end-to-end, produces a runnable artifact, and gives us a foundation to slot the `wisp` crate into later (M2+). Zero new unknowns beyond "does the stack build and run."
>
> **Prerequisite:** M0 ships first. M0.1 (workspace conversion) is shared infrastructure; this doc assumes the workspace already exists.

---

## Acceptance criteria

- ✅ `cargo tauri dev` opens a window
- ✅ Empty state shows a styled drop zone with instruction text
- ✅ Dragging an MP4 over the window changes the drop zone visual (drag-over feedback)
- ✅ Dropping an MP4 transitions to player view
- ✅ Player shows the video with native HTML5 controls (play/pause/seek/volume)
- ✅ Dropping a non-MP4 shows an error and stays on drop zone
- ✅ A "Load another video" button returns to the drop zone empty state
- ✅ Builds cleanly on macOS (Windows/Linux deferred to milestone 2)

---

## Tech notes

- **File drop**: use Tauri 2's native `tauri://drag-drop` event, not HTML5 drag/drop. More reliable cross-platform; we get the file path directly. Configured via `dragDropEnabled: true` in `tauri.conf.json`.
- **File serving**: convert the dropped file path to an `asset://` URL via Tauri's `convertFileSrc()` (or build the URL in Rust and pass to Leptos). The `<video>` element loads from `asset://localhost/...`.
- **Asset protocol**: enable `app.security.assetProtocol.enable = true` in `tauri.conf.json` and add the file's parent directory to `assetProtocol.scope`.
- **Frontend build**: `trunk` is the lighter setup for Tauri+Leptos. (Alternative: `cargo-leptos`, more features but more config.)
- **Validation**: extension check (`.mp4`) + magic byte sniff (`ftyp` at offset 4) for paranoid mode. MVP just checks extension.

---

## Chunks

Each chunk is sized to be completable in under an hour. Numbered M1.x for milestone 1, phase x.

### Phase 1: Scaffold (3 chunks)

#### M1.1 — Add Tauri 2 + Leptos dependencies
- Add to `crates/app/Cargo.toml`: `tauri = "2"`, `tauri-build = "2"` (build dep), `leptos = "0.7"`, `leptos_meta`, `leptos_router`, `console_error_panic_hook`, `wasm-bindgen`
- Add `crates/app/build.rs` calling `tauri_build::build()`
- Add `crates/app/Trunk.toml` with WASM build config
- Add `crates/app/index.html` shell
- **Done when:** `cargo build -p screen-app` succeeds with new deps

#### M1.2 — Tauri config
- Create `crates/app/tauri.conf.json` with window config (1280×800, title "Screen"), `dragDropEnabled: true`, asset protocol enabled with scope `["**"]` for dev (tighten in v1)
- Create `crates/app/icons/` with a placeholder PNG/ICNS
- **Done when:** `tauri.conf.json` validates via `cargo tauri info`

#### M1.3 — Leptos hello-world inside Tauri
- `crates/app/src/main.rs` initializes Tauri runtime
- `crates/app/src/lib.rs` defines a Leptos `App` component rendering `<h1>"Screen"</h1>`
- Wire `index.html` to mount the WASM bundle from Trunk
- **Done when:** `cargo tauri dev` opens a window showing "Screen"

### Phase 2: Drop zone (3 chunks)

#### M2.1 — DropZone visual component
- Create `crates/app/src/components/drop_zone.rs`
- Renders a centered dashed-border box, "Drop an MP4 here" text, file-icon SVG
- CSS via inline `<style>` or a `style.css` file served by Trunk
- **Done when:** drop zone visible in the app, no logic yet

#### M2.2 — Tauri drag-drop event integration
- In Leptos, listen to Tauri's `tauri://drag-drop` event via `tauri-sys` or direct JS interop with `wasm-bindgen`
- Three event variants to handle: `drag-enter`, `drag-leave`, `drop` — each carries paths
- Store dropped paths in a Leptos `RwSignal<Option<Vec<String>>>`
- Log to browser console for verification
- **Done when:** dropping any file logs the path to dev console

#### M2.3 — MP4 filter + path storage
- Validate dropped file extension is `.mp4` (case-insensitive)
- If valid, store the path in a `RwSignal<Option<PathBuf>>` for the active video
- If invalid, set a `RwSignal<Option<String>>` for the error message
- **Done when:** valid drops update the path signal, invalid drops set the error signal

### Phase 3: Video player (3 chunks)

#### M3.1 — Path to asset URL
- Add a Tauri command `path_to_asset_url(path: String) -> String` that wraps the path in `asset://localhost/...` (or use the JS-side `convertFileSrc`)
- Call from Leptos when the path signal updates
- Resulting asset URL stored in a derived signal
- **Done when:** dropping an MP4 produces a logged asset URL

#### M3.2 — VideoPlayer component
- Create `crates/app/src/components/video_player.rs`
- Renders an HTML `<video controls>` element with `src` bound to the asset URL signal
- Auto-plays on load (`autoplay` attribute) — optional, can leave off
- Sized to fit the window
- **Done when:** assigning a valid asset URL plays the video

#### M3.3 — View switching
- Top-level `App` component branches: if path signal is `None`, render `DropZone`; else render `VideoPlayer`
- Use Leptos `Show` / signal-driven conditional
- **Done when:** dropping a video transitions from drop zone to player

### Phase 4: Polish (3 chunks)

#### M4.1 — Drag-over visual feedback
- On `drag-enter`, set a boolean signal `is_dragging`
- DropZone CSS reacts: thicker border, slight background tint, scale up 2%
- On `drag-leave` or `drop`, reset
- **Done when:** dragging a file over the window visibly changes the drop zone

#### M4.2 — Error message UI
- DropZone renders a red error chip below the instruction text when error signal is `Some`
- Error auto-clears after 3 seconds via a `set_timeout`-like pattern in Leptos
- **Done when:** dropping a `.txt` file shows "Only MP4 files are supported" and clears

#### M4.3 — Load another video
- VideoPlayer renders a small "← Load another" button in a corner
- Click resets path signal to `None` and returns to drop zone
- **Done when:** the cycle works: drop → play → reset → drop again

---

## Out of scope for this milestone

Explicitly not in M1 — handle in later milestones:

- Multiple video tracks
- Trimming / cutting / editing
- The `render` crate / wgpu / scene graph
- Capture (no recording yet)
- Encode / export
- Audio waveform display
- Cursor overlay
- Project files / autosave
- Cross-platform (macOS only for M1; Windows/Linux in M2)
- App icon polish
- Code signing / notarization
- Drag-drop of multiple files simultaneously (take only first valid MP4)
- Network / cloud upload
- Subtitle / caption support

---

## Tooling that gets set up along the way

By the end of M1 the project has:

- Cargo workspace with `crates/app`
- Tauri 2 dev/build toolchain working
- Leptos 0.7 hot-reload via Trunk
- WASM bundle output from Trunk consumed by Tauri
- Asset protocol configured for arbitrary local files (tightened later)
- A clear pattern for: signal-driven UI, Tauri command bridge, drag-drop event handling, conditional view rendering

These patterns extend directly to M2+ when we add the timeline, inspector, and render integration.

---

## Estimated effort

11 chunks × ~30–60 min each = **~5–11 hours of focused work** for someone comfortable with Rust and Tauri/Leptos. Add ~1× for reading docs / unforeseen friction on first attempt at this stack.

Tracked as tasks in the task list (M1.1 through M4.3). Workspace setup (formerly M1.1) is now M0.1.
