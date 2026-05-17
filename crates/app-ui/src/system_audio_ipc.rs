//! JS-bridge bindings for the M-AUDIO-SYS.2 / AUT-282 system-audio
//! commands.
//!
//! Mirrors [`crate::camera_ipc`] / [`crate::mic_ipc`] for the
//! per-app speaker-capture path. Each function invokes a `__screen*`
//! helper in `index.html`'s inline script, which wraps
//! `window.__TAURI__.core.invoke(...)`.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

/// Mirror of `crates/app/src/commands.rs::AudioAppView` (M-AUDIO-SYS.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioAppView {
    /// Process identifier observed at enumeration time. Use
    /// `bundle_id` for cross-restart persistence; PIDs drift.
    pub pid: u32,
    /// Bundle identifier (`"com.spotify.client"`).
    pub bundle_id: String,
    /// Human-readable name (`"Spotify"`).
    pub display_name: String,
    /// 32×32 PNG icon bytes. Empty in v0; the picker renders a
    /// placeholder when empty.
    pub icon_png_bytes: Vec<u8>,
}

/// Mirror of `crates/app/src/commands.rs::AudioAppFilterView`. The
/// tagged-enum serde shape lets the Tauri command pick one of the
/// three filter modes without an extra `mode: String` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioAppFilterView {
    /// Capture every app's audio.
    AllAudio,
    /// Capture only these apps (by bundle id).
    OnlyApps(Vec<String>),
    /// Capture everything except these apps.
    ExcludeApps(Vec<String>),
}

#[wasm_bindgen]
extern "C" {
    /// `__screenListAudioApps()` — returns `Promise<AudioAppView[]>`
    /// or rejects with a string error.
    #[wasm_bindgen(js_name = __screenListAudioApps, catch)]
    pub async fn list_audio_apps_js() -> Result<JsValue, JsValue>;

    /// `__screenStartSystemAudio()` — returns `Promise<void>` or
    /// rejects with a string error.
    #[wasm_bindgen(js_name = __screenStartSystemAudio, catch)]
    pub async fn start_system_audio_js() -> Result<JsValue, JsValue>;

    /// `__screenStopSystemAudio()` — returns `Promise<void>`.
    #[wasm_bindgen(js_name = __screenStopSystemAudio, catch)]
    pub async fn stop_system_audio_js() -> Result<JsValue, JsValue>;

    /// `__screenSetSystemAudioFilter(filter)` — returns
    /// `Promise<void>` or rejects with a string error.
    #[wasm_bindgen(js_name = __screenSetSystemAudioFilter, catch)]
    pub async fn set_system_audio_filter_js(filter: JsValue) -> Result<JsValue, JsValue>;

    /// `__screenSystemAudioStatus()` — returns `Promise<boolean>`.
    #[wasm_bindgen(js_name = __screenSystemAudioStatus, catch)]
    pub async fn system_audio_status_js() -> Result<JsValue, JsValue>;

    /// `__screenOpenSettingsScreenRecording()` (M-RECP.6 / AUT-272)
    /// — shells out to open System Settings → Privacy & Security →
    /// Screen Recording. macOS only; Linux/Windows no-op.
    #[wasm_bindgen(js_name = __screenOpenSettingsScreenRecording, catch)]
    pub async fn open_settings_screen_recording_js() -> Result<JsValue, JsValue>;
}

/// Result shape carrying either the typed list or a string error so
/// the picker can render the error inline.
#[derive(Debug)]
pub enum ListAudioAppsResult {
    /// Successful enumeration.
    Ok(Vec<AudioAppView>),
    /// SCK refused (often: TCC permission denied).
    Err(String),
}

/// Async helper: enumerate every running app SCK can see.
pub async fn list_audio_apps() -> ListAudioAppsResult {
    match list_audio_apps_js().await {
        Ok(value) => match serde_wasm_bindgen::from_value(value) {
            Ok(apps) => ListAudioAppsResult::Ok(apps),
            Err(err) => ListAudioAppsResult::Err(format!("decode failed: {err}")),
        },
        Err(err) => ListAudioAppsResult::Err(js_error_string(&err)),
    }
}

/// Async helper: start the system-audio session. Returns `Ok(())`
/// or `Err(message)` where the message is the SCK error description.
pub async fn start_system_audio_capture() -> Result<(), String> {
    start_system_audio_js()
        .await
        .map(|_| ())
        .map_err(|err| js_error_string(&err))
}

/// Async helper: stop the active session.
pub async fn stop_system_audio_capture() {
    let _ = stop_system_audio_js().await;
}

/// Async helper: apply a per-app filter.
pub async fn set_system_audio_filter(filter: AudioAppFilterView) -> Result<(), String> {
    let value = serde_wasm_bindgen::to_value(&filter)
        .map_err(|err| format!("serialise filter failed: {err}"))?;
    set_system_audio_filter_js(value)
        .await
        .map(|_| ())
        .map_err(|err| js_error_string(&err))
}

/// Async helper: status snapshot. `true` when a session is up.
pub async fn system_audio_status() -> bool {
    match system_audio_status_js().await {
        Ok(value) => serde_wasm_bindgen::from_value(value).unwrap_or(false),
        Err(_) => false,
    }
}

/// Async helper: shell out to open System Settings → Screen
/// Recording (M-RECP.6 / AUT-272). Silently no-ops outside Tauri
/// and silently swallows spawn errors — same shape as the other
/// deep-link helpers.
pub async fn open_settings_screen_recording() {
    let _ = open_settings_screen_recording_js().await;
}

/// Subscribe to the `system-audio-level` Tauri event
/// (M-AUDIO.METER / AUT-287) emitted by the SCK delegate at the
/// stream's natural buffer rate (~20-100 Hz). Handler receives an
/// EMA-smoothed master-stream RMS in `[0.0, ~1.0]`. Mirror of
/// [`crate::mic_ipc::subscribe_mic_level`].
pub fn subscribe_system_audio_level(handler: impl Fn(f32) + 'static) {
    use wasm_bindgen::closure::Closure;
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(tauri) = js_sys::Reflect::get(&window, &"__TAURI__".into()) else {
        return;
    };
    let Ok(event) = js_sys::Reflect::get(&tauri, &"event".into()) else {
        return;
    };
    let Ok(listen) = js_sys::Reflect::get(&event, &"listen".into()) else {
        return;
    };
    let Ok(listen_fn) = listen.dyn_into::<js_sys::Function>() else {
        return;
    };
    let cb = Closure::wrap(Box::new(move |payload: JsValue| {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "RMS in [0, 1] easily fits f32"
        )]
        let level = js_sys::Reflect::get(&payload, &"payload".into())
            .ok()
            .and_then(|v| v.as_f64())
            .map_or(0.0_f32, |v| v as f32);
        handler(level);
    }) as Box<dyn Fn(JsValue)>);
    let _ = listen_fn.call2(
        &event,
        &"system-audio-level".into(),
        cb.as_ref().unchecked_ref(),
    );
    cb.forget();
}

/// Best-effort: turn a `JsValue` error into a human-readable string
/// for inline display in the picker.
fn js_error_string(err: &JsValue) -> String {
    err.as_string().unwrap_or_else(|| format!("{err:?}"))
}
