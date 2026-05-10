# Player IPC — Tauri commands + status events (M-PLAY.2)

The chunk that lifts `playback::Player` out of standalone-binary territory
and behind a Tauri IPC surface. The Leptos shell can now drive a real Rust
player from a button click, and the player can push state changes back
without polling.

## Data flow

```text
Leptos UI  (transport buttons)          Rust Tauri shell        playback crate
─────────                                ──────────────          ──────────────
                                                                  PlayerSession
PlayerControls on_toggle ──invoke──► __TAURI__.core ─►  player_play  ─► .play()
                                                        player_pause ─► .pause()
DropZone file path ─────────invoke──► __TAURI__.core ─► player_open  ─► .open()

                  player-status event ◄── emit ◄── tick thread ── .tick(dt)
                                                    (every 33 ms)   .status()
```

Every IPC hop is a one-liner. The bridge in `index.html` exposes three
top-level helpers (`__screenOpen` / `__screenPlay` / `__screenPause`) and
re-emits Tauri's `player-status` event as a browser `CustomEvent`. No
`tauri-sys` crate; the WASM bundle stays dependency-free of Tauri's JS API.

## Tauri commands (`crates/app/src/commands.rs`)

```rust
#[tauri::command]
pub fn player_open(state: State<'_, PlayerSession>, path: String) -> Result<PlayerStatus, String> {
    state.open(&PathBuf::from(path))
}

#[tauri::command]
pub fn player_play(state: State<'_, PlayerSession>) { state.play(); }

#[tauri::command]
pub fn player_pause(state: State<'_, PlayerSession>) { state.pause(); }

#[tauri::command]
pub fn player_status(state: State<'_, PlayerSession>) -> PlayerStatus { state.status() }
```

The four commands are thin wrappers around
[`PlayerSession`](../api/screen_app/player_session/struct.PlayerSession.html).
The session itself is pure Rust (no Tauri types), so its lifecycle is
testable end-to-end without booting Tauri — see
`crates/app/tests/player_session.rs` (6 tests covering empty/open/play/
pause/tick/error paths).

## Tick thread (`crates/app/src/main.rs`)

```rust
fn spawn_tick_thread(app_handle: tauri::AppHandle) {
    thread::spawn(move || {
        let mut last: Option<PlayerStatus> = None;
        loop {
            thread::sleep(TICK_INTERVAL);                  // 33 ms
            let session = app_handle.state::<PlayerSession>();
            session.tick();
            let status = session.status();
            if status_changed(last.as_ref(), &status) {
                let _ = app_handle.emit("player-status", &status);
                last = Some(status);
            }
        }
    });
}
```

`status_changed` throttles emits to:

- **every state transition** (Empty → Paused → Playing → Ended),
- **every 100 ms of `elapsed_ms` change while playing** (10 Hz UI updates).

A 33 ms tick × always-emit would be 30 events / sec hitting the webview;
the 10 Hz throttle keeps the IPC bandwidth flat at the cost of slightly
choppy timer animation. The play/pause UI flip is still instantaneous
because the state change emits immediately.

## JS bridge (`crates/app-ui/index.html`)

```html
<script>
  window.addEventListener("DOMContentLoaded", () => {
    if (window.__TAURI__?.event) {
      window.__TAURI__.event.listen("player-status", (event) => {
        window.dispatchEvent(new CustomEvent("player-status", {
          detail: event.payload
        }));
      });
    }
  });

  window.__screenOpen  = (path) => window.__TAURI__?.core?.invoke("player_open", { path });
  window.__screenPlay  = ()     => window.__TAURI__?.core?.invoke("player_play");
  window.__screenPause = ()     => window.__TAURI__?.core?.invoke("player_pause");
</script>
```

Outbound is a thin `core.invoke` wrapper; inbound is the same CustomEvent
re-emit pattern M-INT.2 introduced for `file-dropped`. Both directions
degrade to no-ops when `window.__TAURI__` is absent — a `trunk serve`
dev session against the standalone Leptos shell still flips the
drop-zone-to-player view via the demo affordance.

## Leptos side (`crates/app-ui/src/player_ipc.rs` + `app.rs`)

```rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = "__screenOpen", catch)]
    pub fn screen_open(path: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_namespace = window, js_name = "__screenPlay", catch)]
    pub fn screen_play() -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_namespace = window, js_name = "__screenPause", catch)]
    pub fn screen_pause() -> Result<JsValue, JsValue>;
}

pub fn install_player_status_listener(set_status: WriteSignal<PlayerStatus>) {
    /* CustomEvent listener — same shape as install_file_drop_listener */
    /* parses CE.detail() via serde-wasm-bindgen, calls set_status.set */
}
```

The `PlayerStatus` and `SessionState` types are mirrored on the
Leptos side (`Deserialize` matches the Rust-side `Serialize`'s
`rename_all = "lowercase"` form). The mirror lives in
`crates/app-ui/src/player_ipc.rs` and must stay in sync with
`crates/app/src/player_session.rs` — they're a contract pair.

The transport buttons in `<PlayerView>` are wrapped in a reactive
closure that re-renders `<PlayerControls>` whenever `player_status`
changes, with an `on_toggle: Callback<()>` that picks `screen_play`
or `screen_pause` based on the current state.

## Testable surface (`crates/app/tests/player_session.rs`)

- `empty_session_reports_empty` — fresh session, nothing loaded.
- `open_transitions_to_paused_with_metadata` — open the test fixture,
  assert width/height/fps come through.
- `play_pause_lifecycle` — round-trip play/pause/play.
- `tick_advances_elapsed_when_playing` — confirms wallclock pumping.
- `tick_is_noop_when_empty` — guard for the always-running tick thread.
- `open_with_invalid_path_errors` — error string flows out cleanly.

## How to run

```bash
# Dev — hot-reload via Trunk, Tauri webview on top.
cd crates/app && cargo tauri dev

# Drop the test fixture onto the window:
#   crates/decode/tests/fixtures/sample.mp4
# The status bar reflects the player's metadata; the play button toggles
# the Rust-side player; the timer ticks at 10 Hz.
```

## App-shell visual references

The player view, post-drop, with its transport bar wired to the IPC
commands. (Component-level layout is unchanged from M-INT.2 — what
changed is the wiring underneath, not the rendered HTML.)

<iframe src="../assets/ui/editor-mock.html" width="100%" height="540" frameborder="0"></iframe>

## Visible playback — `<video>` element bound to `convertFileSrc` (M-PLAY.3)

The IPC plumbing above tracks state, but on its own renders no pixels.
M-PLAY.3 wires the user-visible playback surface: an HTML5 `<video>`
element whose `src` is derived from the dropped path via Tauri 2's
[`convertFileSrc`](https://docs.rs/tauri/2/tauri/webview/struct.Webview.html)
JS helper.

```text
file dropped → loaded signal → video_src() ─┐
                                            ▼
                              window.__screenConvertFileSrc(path)
                                            │  (asset:// or http://asset.localhost)
                                            ▼
                              <video src=... node_ref=video_ref />

PlayerControls toggle click ─┬─ video.play() / video.pause() (sync, user-gesture)
                             └─ screen_play() / screen_pause()  (Tauri state)

player-status event ─→ Effect ─→ video.play()/pause() (catch-up, EOF, future seek)
```

### Why two paths to the `<video>` element

WebKit blocks programmatic `.play()` outside a user gesture. So:

- **Click handler** drives `<video>` synchronously inside the
  `Callback<()>`. The browser sees this as user-initiated and allows
  playback to start.
- **`Effect::new` over `player_status`** is the catch-up path for
  state changes that *aren't* user clicks — Tauri pushing `Ended` on
  EOF, future seek commands, etc. Idempotent: it only acts when
  `video.paused()` doesn't already match the target state, so it
  doesn't fight the click handler.

### Why HTML5 video for the playback surface (and not wisp)

The recorder's *editor preview* surface will eventually be a
winit-child window driven by wisp (so we can apply filters, transforms,
animation). But for the MVP "user dropped a file and wants to see it",
HTML5 `<video>` with the asset protocol is:

- one element, no decoder integration,
- hardware-accelerated by the WebView,
- scrub-bar/seek/audio for free.

The Tauri-side
[`PlayerSession`](../api/screen_app/player_session/struct.PlayerSession.html)
keeps running alongside — it owns the gstreamer-decoded
[`VideoTexture`](../api/wisp/struct.VideoTexture.html) that future
wisp-rendered surfaces will read. Two-source-of-truth is a deliberate
trade for shipping the playback MVP today.

### `tauri.conf.json` requirements

The `assetProtocol.scope` must include the dropped file's path. Our
config uses `["**"]` (any local file). For a production build we'd
tighten to user-selected directories.

```json
"security": {
  "assetProtocol": {
    "enable": true,
    "scope": ["**"]
  }
}
```

Without `"enable": true`, `convertFileSrc` returns the path unchanged
and the `<video>` element fails to load with a CSP / protocol error.

[`PlayerSession` API](../api/screen_app/player_session/struct.PlayerSession.html) ·
[Tauri commands](../api/screen_app/commands/index.html) ·
[`PlayerControls` component](../ui/chunks/player-controls-paused.md) ·
[Tauri ↔ Leptos integration (M-INT.2)](./integration.md)
