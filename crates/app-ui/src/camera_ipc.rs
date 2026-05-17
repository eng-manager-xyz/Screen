//! JS-bridge bindings for the M-CAM.2 Tauri camera commands
//! (M-CAM.4 / AUT-258 + M-REC.1 / AUT-260).
//!
//! Mirrors `crate::player_ipc` for the camera surface. Each function
//! invokes a `__screen*` helper declared in `index.html`'s inline
//! script, which in turn wraps `window.__TAURI__.core.invoke(...)`.
//!
//! The returned values come back as `JsValue` and are deserialised
//! into typed Rust structs via `serde_wasm_bindgen` — the same
//! pattern the player IPC layer uses.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

/// Mirror of `crates/app/src/commands.rs::CameraView` (M-CAM.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraView {
    /// Stable device id.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// First in the enumeration order.
    pub is_default: bool,
}

/// Mirror of `crates/app/src/commands.rs::CameraPermission` (M-CAM.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraPermission {
    /// User has granted camera access.
    Granted,
    /// macOS has not yet asked the user.
    NotDetermined,
    /// User has explicitly denied access.
    Denied,
}

#[wasm_bindgen]
extern "C" {
    /// `__screenListCameras()` in `index.html` — returns `Promise<CameraView[]>`.
    #[wasm_bindgen(js_name = __screenListCameras, catch)]
    pub async fn list_cameras_js() -> Result<JsValue, JsValue>;

    /// `__screenStartPreview(cameraId)` in `index.html` — returns `Promise<void>`.
    #[wasm_bindgen(js_name = __screenStartPreview, catch)]
    pub async fn start_preview_js(camera_id: String) -> Result<JsValue, JsValue>;

    /// `__screenStopPreview()` in `index.html` — returns `Promise<void>`.
    #[wasm_bindgen(js_name = __screenStopPreview, catch)]
    pub async fn stop_preview_js() -> Result<JsValue, JsValue>;

    /// `__screenCameraPermissionStatus()` in `index.html` — returns
    /// `Promise<CameraPermission>`.
    #[wasm_bindgen(js_name = __screenCameraPermissionStatus, catch)]
    pub async fn camera_permission_status_js() -> Result<JsValue, JsValue>;

    /// `__screenOpenSettingsCamera()` (M-RECP.0 / AUT-261) — shells
    /// out to open System Settings → Privacy & Security → Camera.
    #[wasm_bindgen(js_name = __screenOpenSettingsCamera, catch)]
    pub async fn open_settings_camera_js() -> Result<JsValue, JsValue>;
}

/// Async helper: list every camera the OS exposes.
///
/// Returns an empty `Vec` when running outside Tauri (e.g. plain
/// `trunk serve` browser preview), or when the OS has no cameras.
pub async fn list_cameras() -> Vec<CameraView> {
    match list_cameras_js().await {
        Ok(value) => serde_wasm_bindgen::from_value(value).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Async helper: kick off `start_preview` for the given camera id.
///
/// Failures are silently swallowed today — the M-CAM.3 frame channel
/// hasn't landed yet, so observable behaviour is "state machine
/// transitions to Running". Once frames flow the Recorder
/// `<CameraPreview />` overlay will reflect any error via the
/// `RecorderPreviewState` enum.
pub async fn start_preview(camera_id: String) {
    let _ = start_preview_js(camera_id).await;
}

/// Async helper: tear down the active preview.
pub async fn stop_preview() {
    let _ = stop_preview_js().await;
}

/// Async helper: probe OS camera permission state.
///
/// Returns `Granted` when running outside Tauri so the picker
/// renders normally during `trunk serve` dev.
pub async fn camera_permission_status() -> CameraPermission {
    match camera_permission_status_js().await {
        Ok(value) => serde_wasm_bindgen::from_value(value).unwrap_or(CameraPermission::Granted),
        Err(_) => CameraPermission::Granted,
    }
}

/// Async helper: shell out to open System Settings → Camera
/// (M-RECP.0 / AUT-261). Silently no-ops outside Tauri (so
/// `trunk serve` dev doesn't error on click) and silently swallows
/// spawn errors — the user-facing "tell me if I can't open this"
/// case is rare enough that surfacing it adds more noise than value.
pub async fn open_settings_camera() {
    let _ = open_settings_camera_js().await;
}
