//! Leptos-side bindings for the screen-app player IPC.
//!
//! Three responsibilities:
//!
//! 1. Mirror the Rust-side `screen_app::player_session::PlayerStatus`
//!    payload (kept in sync manually — they must match the
//!    `Serialize`/`Deserialize` shape on both ends). The link is
//!    deliberately plain text: `app-ui` is a WASM crate and can't depend
//!    on `screen-app` (Tauri-native), so the type can't be resolved by
//!    rustdoc.
//! 2. Expose `extern "C"` thin Rust wrappers around the JS helpers
//!    `__screenOpen` / `__screenPlay` / `__screenPause` declared in
//!    `index.html`. Returns are `Result<JsValue, JsValue>` (`catch`-style)
//!    so the calls degrade to no-ops when running outside Tauri.
//! 3. [`install_player_status_listener`] wires a `player-status`
//!    browser-`CustomEvent` listener and pushes the parsed payload
//!    into a Leptos `WriteSignal<PlayerStatus>`.

use leptos::prelude::{Set, WriteSignal};
use serde::Deserialize;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::CustomEvent;

/// IPC-stable mirror of `screen_app::player_session::SessionState`
/// (plain-text reference — see the crate-level docs for why).
/// `serde(rename_all = "lowercase")` matches the Rust-side serialization.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    /// No file is loaded.
    #[default]
    Empty,
    /// A file is loaded; the player is not advancing.
    Paused,
    /// A file is loaded; the player is advancing in real time.
    Playing,
    /// The stream reached EOF.
    Ended,
}

/// IPC-stable mirror of `screen_app::player_session::PlayerStatus`.
#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
pub struct PlayerStatus {
    /// Lifecycle state.
    pub state: SessionState,
    /// Wallclock-elapsed since `play` was first called, in milliseconds.
    pub elapsed_ms: u64,
    /// Total duration in milliseconds, when known.
    pub duration_ms: Option<u64>,
    /// Native frame width in pixels.
    pub width: u32,
    /// Native frame height in pixels.
    pub height: u32,
    /// Source frame rate (frames per second).
    pub fps: f32,
    /// Total frame count, when known.
    pub frame_count: Option<u64>,
}

#[wasm_bindgen]
extern "C" {
    /// Open a file. Bridge throws if `__TAURI__` is unavailable
    /// (browser-only `trunk serve` path) — we ignore the error.
    #[wasm_bindgen(js_namespace = window, js_name = "__screenOpen", catch)]
    pub fn screen_open(path: &str) -> Result<JsValue, JsValue>;

    /// Resume playback.
    #[wasm_bindgen(js_namespace = window, js_name = "__screenPlay", catch)]
    pub fn screen_play() -> Result<JsValue, JsValue>;

    /// Pause playback.
    #[wasm_bindgen(js_namespace = window, js_name = "__screenPause", catch)]
    pub fn screen_pause() -> Result<JsValue, JsValue>;
}

/// Install a `player-status` browser-`CustomEvent` listener.
///
/// The Tauri shell's JS bridge re-emits the Tauri-side `player-status`
/// events as browser `CustomEvent`s whose `.detail` is the [`PlayerStatus`]
/// payload as a JS object. We deserialize via `serde-wasm-bindgen` and
/// push into the supplied signal.
///
/// The closure is leaked via `Closure::forget` because the listener has
/// app-lifetime — it should never be removed.
pub fn install_player_status_listener(set_status: WriteSignal<PlayerStatus>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if let Ok(ce) = event.dyn_into::<CustomEvent>()
            && let Ok(status) = serde_wasm_bindgen::from_value::<PlayerStatus>(ce.detail())
        {
            set_status.set(status);
        }
    }) as Box<dyn FnMut(_)>);
    let _ =
        window.add_event_listener_with_callback("player-status", closure.as_ref().unchecked_ref());
    closure.forget();
}

/// Position fraction `0.0..=1.0` from elapsed/duration. Returns `0.0`
/// when duration is unknown or zero.
#[must_use]
pub fn position(status: &PlayerStatus) -> f32 {
    let Some(d) = status.duration_ms else {
        return 0.0;
    };
    if d == 0 {
        return 0.0;
    }
    fraction_ms(status.elapsed_ms, d)
}

/// Total duration in seconds. Falls back to `60.0` (matches the
/// `PlayerControls` component's own fallback) when not reported.
#[must_use]
pub fn duration_seconds(status: &PlayerStatus) -> f32 {
    let Some(d) = status.duration_ms else {
        return 60.0;
    };
    ms_to_seconds(d)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "scrub-bar UI; ms values fit comfortably in f64 and the f32 result is for layout only"
)]
#[allow(
    clippy::cast_possible_truncation,
    reason = "f64 fraction is bounded by [0, ~1.0], well within f32 range"
)]
fn fraction_ms(num_ms: u64, denom_ms: u64) -> f32 {
    let f = (num_ms as f64) / (denom_ms as f64);
    f as f32
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "duration display tolerates ~ms-rounded f32 (UI shows whole seconds anyway); ms / 1000 fits in f32 for any realistic video"
)]
fn ms_to_seconds(ms: u64) -> f32 {
    (ms as f64 / 1000.0) as f32
}
