# Testing the recorder shell — three tiers (M-TEST.1 / .2)

`screen-app` (Tauri shell) and `app-ui` (Leptos webview) sit at the integrated
top of the stack. Their regression surface is different in shape from the
library crates below — there's an OS process, a WebView, an IPC channel,
and a JS bridge between Rust and Rust. The tests that matter are
correspondingly stratified.

## The three tiers

| Tier | What runs | Catches | Cost | Lives in |
|---|---|---|---|---|
| **0. Chunk-level** | `cargo nextest` | Unit-level invariants in each crate (PlayerSession lifecycle, `aspect_fit_scale`, etc.) | <2s | every crate's `tests/` |
| **1. IPC harness** | `cargo nextest` (still in-process) | Tauri command registration, serde wire shapes, `State<T>` plumbing | ~1s | `crates/app/tests/commands.rs` |
| **2. WebDriver e2e** | `tauri-driver` + `fantoccini` | Real WebView + Leptos rendering + JS bridge + Rust round-trip | ~10–30s | `crates/app-e2e/` (Linux only) |

Adding a tier-N test does NOT replace tier-(N-1). They overlap deliberately —
tier 0 fires fast on every save; tier 2 fires once per CI run; in-between
tier 1 catches the wire-format and registration regressions that tier 0
can't reach (no IPC dispatch) and tier 2 is too expensive to keep paged in.

## Tier 0 — chunk-level (existing pattern)

Direct tests against the data types and functions in each crate. Already
the dominant test layer in the workspace. See `_docs/TESTING.md` for the
broader strategy. For `screen-app` specifically:

```rust
// crates/app/tests/player_session.rs
#[test]
fn play_pause_lifecycle() {
    let session = PlayerSession::new();
    session.open(Path::new(FIXTURE)).unwrap();
    session.play();
    assert_eq!(session.status().state, SessionState::Playing);
    session.pause();
    assert_eq!(session.status().state, SessionState::Paused);
}
```

These tests bypass Tauri entirely. If `PlayerSession` is correct, but the
Tauri registration layer has a typo, these still pass.

## Tier 1 — IPC harness (M-TEST.1)

Uses `tauri::test::mock_builder()` to spin up a Tauri runtime in-process.
No window, no WebView, no WASM. Commands are dispatched the way the Leptos
frontend dispatches them.

```rust
// crates/app/tests/commands.rs
fn build_app() -> tauri::App<MockRuntime> {
    mock_builder()
        .manage(PlayerSession::new())
        .invoke_handler(tauri::generate_handler![
            commands::player_open,
            commands::player_play,
            commands::player_pause,
            commands::player_status,
        ])
        .build(mock_context(noop_assets()))
        .expect("build mock app")
}

#[test]
fn player_open_then_play_then_pause_via_ipc() {
    let app = build_app();
    let webview = main_webview(&app);
    invoke(&webview, "player_open", InvokeBody::Json(json!({ "path": FIXTURE })));
    invoke(&webview, "player_play", InvokeBody::default());
    let value = invoke(&webview, "player_status", InvokeBody::default());
    let status: PlayerStatus = serde_json::from_value(value).unwrap();
    assert_eq!(status.state, SessionState::Playing);
}
```

What this catches that Tier 0 misses:

- A typo in `tauri::generate_handler![commands::playerr_play]` — compile
  fails; this test would never run.
- A missing `.manage(PlayerSession::new())` — Tier 0 still passes (it
  builds the session directly); Tier 1 fails at runtime when the command
  tries to read State.
- A `#[serde(rename_all = "lowercase")]` accidentally dropped from
  `SessionState` — Tier 0 doesn't serialize anything; Tier 1's
  `serde_shape_uses_lowercase_session_state` test fails because the
  payload now reads `"Empty"` instead of `"empty"`.

Cost: each test pays the same wisp `Application::new()` boot (~200 ms on
Apple Silicon) as Tier 0. Total Tier 1 runtime is ~1 s.

Setup required: `tauri = { version = "2", features = ["test"] }` in
`[dev-dependencies]`. Cargo unifies dep + dev-dep features, so the release
binary also gets the `test` module compiled in (a known cargo wart). The
footprint is small enough to accept.

## Tier 2 — WebDriver e2e (M-TEST.2)

The real thing: launches the built `screen-app` binary, drives it through
WebDriver via `tauri-driver`, asserts on rendered DOM state.

```rust
// crates/app-e2e/tests/golden_path.rs
#[tokio::test]
async fn open_play_pause_via_ui() {
    let app = E2eApp::start().await;
    let driver = app.client();

    // OS-level file drop can't be scripted via WebDriver, so the test
    // calls a debug-only Tauri command (`__test_drop_file`) that emits
    // the same `file-dropped` event the real drag-drop handler emits.
    driver.execute(
        "return window.__TAURI__.core.invoke('__test_drop_file', { path: arguments[0] })",
        vec![FIXTURE.into()],
    ).await?;

    driver.wait().for_element(Locator::Css(".player-controls")).await?;
    driver.find(Locator::Css(".player-toggle")).await?.click().await?;
    driver.wait_for(|d| async {
        d.find(Locator::Css(".player-toggle-playing")).await.is_ok()
    }).await?;
}
```

What this catches that Tier 1 misses:

- `index.html` JS bridge regressions (e.g. dropping `__screenPlay`
  globals, breaking the `__TAURI__.event.listen` re-emit).
- Leptos rendering / hydration issues (e.g. `<PlayerControls>` not
  re-rendering when `player_status` changes).
- CSS layout regressions that hide the play button off-screen.
- Cross-process timing — events arriving after the listener is wired,
  Promise resolution order on the JS side, etc.

### Platform support

`tauri-driver` works well on **Linux** (WebKitGTK has solid WebDriver
support via `webkit2gtk-driver`) and **Windows** (Edge WebView2 +
`msedgedriver`). **macOS** is the gap — Apple's WKWebView WebDriver
support is half-implemented and `tauri-driver` doesn't reliably drive
it. The community pattern is "Linux CI gates everything; mac is manual
smoke before tagging."

This workspace runs Tier 2 on `ubuntu-latest` only. The `just e2e`
recipe detects the host OS:

- **Linux:** runs `xvfb-run cargo nextest run -p app-e2e`.
- **macOS:** prints a clear "skipping — see [Player IPC chapter] for
  manual smoke procedure" message and exits 0.

### Local prerequisites (Linux)

```bash
# Cargo plugin:
cargo install --locked tauri-driver

# System packages (Debian/Ubuntu):
sudo apt-get install -y webkit2gtk-driver xvfb
```

Then:

```bash
just e2e
```

### File-drop simulation — the trick

WebDriver clients can't synthesize OS-level drag-drop events. Tauri 2's
`WindowEvent::DragDrop` only fires from real OS drops, not JS code. The
solution: a debug-only Tauri command that emits the same
`file-dropped` event the real handler emits.

```rust
#[cfg(debug_assertions)]
#[tauri::command]
pub fn __test_drop_file(app: tauri::AppHandle, path: String) -> Result<(), String> {
    app.emit("file-dropped", path).map_err(|e| e.to_string())
}
```

Gated by `#[cfg(debug_assertions)]` and a corresponding `#[cfg(...)]` in
the `generate_handler!` macro, so the test entry point is excluded from
release builds. Tests invoke it via
`window.__TAURI__.core.invoke('__test_drop_file', { path })`.

The real OS drag-drop path stays untouched. Tier 1 + Tier 2 cover it
collectively: Tier 1 verifies the `player_open` command (the *handler*
side of the file-drop chain), Tier 2 verifies the full UI flow assuming
a `file-dropped` event was emitted.

## When to add tests at each tier

- **New chunk with pure Rust logic** → Tier 0.
- **New `#[tauri::command]` exposed to the frontend** → add a Tier 1
  case with the exact JSON body shape the frontend will send.
- **New event emitted from Rust → received by Leptos** → add a Tier 2
  case asserting the rendered DOM responds to a synthesized event.
- **New cross-process timing concern (race conditions, ordering)** →
  Tier 2 only. Tier 1 doesn't have wall-clock semantics.

## What's still missing

- **Visual regression for the integrated shell.** The
  [ui-storybook SSR snapshots](../ui/components.md) cover individual
  components, but a "full editor mock under the live IPC" diff isn't
  captured today. Could be added under Tier 2 with `headless-screenshot`
  + `image::compare` once the e2e suite stabilizes.
- **macOS automation.** Tracked as future work; the broader Tauri
  community is still iterating on this.
