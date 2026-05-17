//! JS-bridge bindings for the M-MIC.1 / AUT-278 microphone commands
//! (M-MIC.2 / AUT-279).
//!
//! Mirrors [`crate::camera_ipc`] for the audio path. Each function
//! invokes a `__screen*` helper declared in `index.html`'s inline
//! script, which wraps `window.__TAURI__.core.invoke(...)`.
//!
//! Returned values are deserialised from `JsValue` into typed Rust
//! structs via `serde_wasm_bindgen` — same pattern as the camera +
//! player IPC modules.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

use crate::camera_ipc::CameraPermission;

/// Mirror of `crates/app/src/commands.rs::MicrophoneView` (M-MIC.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicrophoneView {
    /// Stable device id (`mic-…`).
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// `true` for the OS-default mic.
    pub is_default: bool,
    /// Native channel count from the gst caps line. `0` = unknown.
    pub channels: u8,
    /// Native sample rate from the gst caps line. `0` = unknown.
    pub sample_rate_hz: u32,
}

/// Mirror of `crates/app/src/audio/mod.rs::MicLifecycle`. Tagged
/// representation matches the Rust enum's default serde shape (one
/// of `"Idle"` / `"Starting"` / `"Running"` / `"Stopping"`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MicLifecycle {
    /// No worker running.
    #[default]
    Idle,
    /// Worker spawned; awaiting first chunk.
    Starting,
    /// Worker is producing chunks.
    Running,
    /// Worker is being torn down.
    Stopping,
}

#[wasm_bindgen]
extern "C" {
    /// `__screenListMicrophones()` — returns `Promise<MicrophoneView[]>`.
    #[wasm_bindgen(js_name = __screenListMicrophones, catch)]
    pub async fn list_microphones_js() -> Result<JsValue, JsValue>;

    /// `__screenStartMicCapture(micId)` — returns `Promise<void>`.
    #[wasm_bindgen(js_name = __screenStartMicCapture, catch)]
    pub async fn start_mic_capture_js(mic_id: String) -> Result<JsValue, JsValue>;

    /// `__screenStopMicCapture()` — returns `Promise<void>`.
    #[wasm_bindgen(js_name = __screenStopMicCapture, catch)]
    pub async fn stop_mic_capture_js() -> Result<JsValue, JsValue>;

    /// `__screenMicStatus()` — returns `Promise<MicLifecycle>`.
    #[wasm_bindgen(js_name = __screenMicStatus, catch)]
    pub async fn mic_status_js() -> Result<JsValue, JsValue>;

    /// `__screenMicrophonePermissionStatus()` — returns
    /// `Promise<CameraPermission>` (same three-variant shape).
    #[wasm_bindgen(js_name = __screenMicrophonePermissionStatus, catch)]
    pub async fn microphone_permission_status_js() -> Result<JsValue, JsValue>;

    /// `__screenOpenSettingsMicrophone()` (M-RECP.8 / AUT-286) —
    /// shells out to open System Settings → Privacy & Security →
    /// Microphone.
    #[wasm_bindgen(js_name = __screenOpenSettingsMicrophone, catch)]
    pub async fn open_settings_microphone_js() -> Result<JsValue, JsValue>;
}

/// Async helper: list every microphone the OS exposes.
///
/// Returns an empty `Vec` when running outside Tauri (plain
/// `trunk serve` browser preview), or when no mics are attached.
pub async fn list_microphones() -> Vec<MicrophoneView> {
    match list_microphones_js().await {
        Ok(value) => serde_wasm_bindgen::from_value(value).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Async helper: kick off `start_mic_capture` for the given mic id.
///
/// Failures are silently swallowed — the picker UX reads back the
/// effect via [`mic_status`] (and, in a follow-up, via the
/// `audio-levels` event when the meter wires up). The IPC layer's
/// re-entrant contract (M-MIC.1) means calling this with a new id
/// while an older session is running cleanly tears down the previous
/// pipeline.
pub async fn start_mic_capture(mic_id: String) {
    let _ = start_mic_capture_js(mic_id).await;
}

/// Async helper: tear down the active mic worker.
pub async fn stop_mic_capture() {
    let _ = stop_mic_capture_js().await;
}

/// Async helper: snapshot the worker lifecycle.
///
/// Returns `Idle` when running outside Tauri.
pub async fn mic_status() -> MicLifecycle {
    match mic_status_js().await {
        Ok(value) => serde_wasm_bindgen::from_value(value).unwrap_or_default(),
        Err(_) => MicLifecycle::Idle,
    }
}

/// Async helper: probe OS microphone permission state.
///
/// Returns `Granted` when running outside Tauri so the picker
/// renders normally during `trunk serve` dev.
pub async fn microphone_permission_status() -> CameraPermission {
    match microphone_permission_status_js().await {
        Ok(value) => serde_wasm_bindgen::from_value(value).unwrap_or(CameraPermission::Granted),
        Err(_) => CameraPermission::Granted,
    }
}

/// Async helper: shell out to open System Settings → Microphone
/// (M-RECP.8 / AUT-286). Same shape + silent-failure semantics as
/// [`crate::camera_ipc::open_settings_camera`].
pub async fn open_settings_microphone() {
    let _ = open_settings_microphone_js().await;
}
