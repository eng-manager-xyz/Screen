//! JS-bridge bindings for the M-SCK.1 / .2 / .4 screen-capture
//! commands (AUT-268 / AUT-269 / AUT-271).
//!
//! Mirror of [`crate::camera_ipc`] / [`crate::mic_ipc`] /
//! [`crate::system_audio_ipc`] for the screen video path. Each
//! function invokes a `__screen*` helper in `index.html` which
//! wraps `window.__TAURI__.core.invoke(...)`.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

/// Mirror of `crates/app/src/commands.rs::DisplaySourceView`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplaySourceView {
    /// Stable id (`display-<displayID>`).
    pub id: String,
    /// Human-readable label (`"Display 1920×1080"`).
    pub label: String,
    /// Width in points.
    pub width: u32,
    /// Height in points.
    pub height: u32,
    /// `true` for the first display in the enumeration.
    pub is_primary: bool,
}

/// Mirror of `crates/app/src/commands.rs::WindowSourceView`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowSourceView {
    /// Stable id for the current session (`window-<windowID>`).
    pub id: String,
    /// Window title (or empty).
    pub label: String,
    /// Width in points.
    pub width: u32,
    /// Height in points.
    pub height: u32,
    /// Owning app bundle id.
    pub bundle_id: String,
    /// Owning app display name.
    pub display_name: String,
}

/// Tagged result so the picker can render a TCC-denied error
/// inline instead of swallowing it.
#[derive(Debug)]
pub enum ScreenSourcesResult<T> {
    /// Successful enumeration.
    Ok(Vec<T>),
    /// SCK refused (often: TCC permission denied).
    Err(String),
}

#[wasm_bindgen]
extern "C" {
    /// `__screenListScreenDisplays()` — `Promise<DisplaySourceView[]>`.
    #[wasm_bindgen(js_name = __screenListScreenDisplays, catch)]
    pub async fn list_screen_displays_js() -> Result<JsValue, JsValue>;

    /// `__screenListScreenWindows()` — `Promise<WindowSourceView[]>`.
    #[wasm_bindgen(js_name = __screenListScreenWindows, catch)]
    pub async fn list_screen_windows_js() -> Result<JsValue, JsValue>;

    /// `__screenStartScreenCapture(sourceId?: string)` —
    /// `Promise<void>` (or string error). `sourceId` is
    /// `"display-<id>"` / `"window-<id>"` / `null` for primary
    /// display (M-SCK.0.1 / AUT-291).
    #[wasm_bindgen(js_name = __screenStartScreenCapture, catch)]
    pub async fn start_screen_capture_js(source_id: JsValue) -> Result<JsValue, JsValue>;

    /// `__screenStopScreenCapture()` — `Promise<void>`.
    #[wasm_bindgen(js_name = __screenStopScreenCapture, catch)]
    pub async fn stop_screen_capture_js() -> Result<JsValue, JsValue>;

    /// `__screenScreenCaptureStatus()` — `Promise<boolean>`.
    #[wasm_bindgen(js_name = __screenScreenCaptureStatus, catch)]
    pub async fn screen_capture_status_js() -> Result<JsValue, JsValue>;

    /// `__screenScreenCaptureFrameCount()` — `Promise<number>`.
    #[wasm_bindgen(js_name = __screenScreenCaptureFrameCount, catch)]
    pub async fn screen_capture_frame_count_js() -> Result<JsValue, JsValue>;
}

/// Enumerate displays. Empty `Vec` outside Tauri.
pub async fn list_screen_displays() -> ScreenSourcesResult<DisplaySourceView> {
    match list_screen_displays_js().await {
        Ok(value) => match serde_wasm_bindgen::from_value(value) {
            Ok(v) => ScreenSourcesResult::Ok(v),
            Err(err) => ScreenSourcesResult::Err(format!("decode failed: {err}")),
        },
        Err(err) => ScreenSourcesResult::Err(js_error_string(&err)),
    }
}

/// Enumerate visible windows.
pub async fn list_screen_windows() -> ScreenSourcesResult<WindowSourceView> {
    match list_screen_windows_js().await {
        Ok(value) => match serde_wasm_bindgen::from_value(value) {
            Ok(v) => ScreenSourcesResult::Ok(v),
            Err(err) => ScreenSourcesResult::Err(format!("decode failed: {err}")),
        },
        Err(err) => ScreenSourcesResult::Err(js_error_string(&err)),
    }
}

/// Start screen capture. `source_id` of `None` captures the primary
/// display (M-SCK.0 default); `Some("display-<id>")` / `Some("window-<id>")`
/// pin to a specific source (M-SCK.0.1 / AUT-291). Returns
/// `Ok(())` or `Err(message)`.
pub async fn start_screen_capture(source_id: Option<String>) -> Result<(), String> {
    let arg = match source_id {
        Some(id) => JsValue::from_str(&id),
        None => JsValue::NULL,
    };
    start_screen_capture_js(arg)
        .await
        .map(|_| ())
        .map_err(|err| js_error_string(&err))
}

/// Stop screen capture.
pub async fn stop_screen_capture() {
    let _ = stop_screen_capture_js().await;
}

/// `true` when a screen-capture session is active.
pub async fn screen_capture_status() -> bool {
    match screen_capture_status_js().await {
        Ok(value) => serde_wasm_bindgen::from_value(value).unwrap_or(false),
        Err(_) => false,
    }
}

/// Cumulative frame counter. Returns `0` outside Tauri.
pub async fn screen_capture_frame_count() -> u64 {
    match screen_capture_frame_count_js().await {
        Ok(value) => serde_wasm_bindgen::from_value(value).unwrap_or(0),
        Err(_) => 0,
    }
}

fn js_error_string(err: &JsValue) -> String {
    err.as_string().unwrap_or_else(|| format!("{err:?}"))
}
