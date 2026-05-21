//! JS-bridge bindings for the M-RECORD.1 coordinated-recording
//! commands (start / stop / status) + the `recording-status` event.
//!
//! Mirror of [`crate::screen_ipc`] / [`crate::mic_ipc`] / etc. The
//! `__screenStartRecording` / `__screenStopRecording` /
//! `__screenRecordingStatus` helpers in `index.html` wrap
//! `window.__TAURI__.core.invoke(...)`.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

/// Mirror of `screen_app::recording::SessionStreams`. Per-channel
/// flags chosen at session-start time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "Each bool maps to one of the four physical input channels (camera / screen / mic / system audio). Mirrors the Rust-side `SessionStreams` shape verbatim; a bitflag would diverge the IPC seam."
)]
pub struct SessionStreamsView {
    /// Include the camera channel.
    pub camera: bool,
    /// Include the screen-capture channel.
    pub screen: bool,
    /// Include the microphone channel.
    pub microphone: bool,
    /// Include the system / per-app audio channel.
    pub system_audio: bool,
}

impl SessionStreamsView {
    /// `true` if at least one channel is enabled.
    #[must_use]
    pub fn any_enabled(self) -> bool {
        self.camera || self.screen || self.microphone || self.system_audio
    }
}

/// Mirror of `screen_app::recording::RecordingConfig`. Sent to
/// `start_recording`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RecordingConfigView {
    /// Which physical channels to coordinate.
    pub streams: SessionStreamsView,
    /// Camera picker selection (FNV-1a id, empty = OS default).
    pub camera_id: String,
    /// Microphone picker selection (FNV-1a id, empty = OS default).
    pub microphone_id: String,
    /// Screen-source picker selection (`"display-<id>"` /
    /// `"window-<id>"`, `None` = primary display).
    pub screen_source_id: Option<String>,
    /// Output file path. `None` means "use the default location"
    /// (M-EXPORT.4 owns the default).
    pub output_path: Option<String>,
    /// Output container/codec format slug.
    pub format: Option<String>,
}

/// Mirror of `screen_app::recording::RecordingStatusView`. Returned
/// by `recording_status` and pushed as the `recording-status` event
/// every 500 ms while a session is active.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RecordingStatusViewIpc {
    /// `None` when no session is active.
    pub session_id: Option<u64>,
    /// One of `"Idle"` / `"Starting"` / `"Running"` / `"Stopping"`.
    pub state: String,
    /// Elapsed time in milliseconds.
    pub elapsed_ms: u64,
    /// One entry per enabled stream.
    pub streams: Vec<StreamHealthView>,
}

/// Mirror of `screen_app::recording::StreamHealth`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StreamHealthView {
    /// `"Camera"` / `"Screen"` / `"Microphone"` / `"SystemAudio"`.
    pub kind: String,
    /// Per-channel lifecycle (free-form string from the per-channel
    /// enum's `Debug` repr — `"Idle"` / `"Starting"` / `"Running"` /
    /// `"Stopping"`).
    pub lifecycle: String,
    /// Cumulative frame / chunk count since session start.
    pub frame_count: u64,
    /// Milliseconds since the most recent frame, if any.
    pub last_frame_ms_ago: Option<u64>,
}

/// Mirror of `screen_app::recording::RecordingSummary`. Returned by
/// `stop_recording`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RecordingSummaryView {
    /// The session id that just stopped.
    pub session_id: u64,
    /// Total session duration in milliseconds.
    pub elapsed_ms: u64,
    /// Final per-stream tally.
    pub streams: Vec<StreamHealthView>,
    /// Output file path, if M-EXPORT wrote one.
    pub output_path: Option<String>,
}

impl RecordingStatusViewIpc {
    /// Empty / no-session snapshot.
    #[must_use]
    pub fn idle() -> Self {
        Self {
            session_id: None,
            state: "Idle".to_string(),
            elapsed_ms: 0,
            streams: Vec::new(),
        }
    }

    /// `true` when the master session is `Running` (i.e. at least
    /// one stream produced its first frame). Used by M-RECORD.3 to
    /// lock the per-channel pickers.
    #[must_use]
    pub fn is_recording(&self) -> bool {
        matches!(self.state.as_str(), "Starting" | "Running" | "Stopping")
    }
}

#[wasm_bindgen]
extern "C" {
    /// `__screenStartRecording(config)` —
    /// `Promise<u64>` (session id) or string error.
    #[wasm_bindgen(js_name = __screenStartRecording, catch)]
    pub async fn start_recording_js(config: JsValue) -> Result<JsValue, JsValue>;

    /// `__screenStopRecording()` — `Promise<RecordingSummary>`.
    #[wasm_bindgen(js_name = __screenStopRecording, catch)]
    pub async fn stop_recording_js() -> Result<JsValue, JsValue>;

    /// `__screenRecordingStatus()` — `Promise<RecordingStatusView>`.
    #[wasm_bindgen(js_name = __screenRecordingStatus, catch)]
    pub async fn recording_status_js() -> Result<JsValue, JsValue>;

    /// `__screenDefaultRecordingOutputPath(formatSlug?: string)` —
    /// `Promise<string>` returning the auto-generated default path
    /// (M-EXPORT.4).
    #[wasm_bindgen(js_name = __screenDefaultRecordingOutputPath, catch)]
    pub async fn default_output_path_js(format_slug: JsValue) -> Result<JsValue, JsValue>;

    /// `__screenRevealRecordingInFileManager(path: string)` —
    /// `Promise<void>`. Opens the OS file manager focused on the
    /// given path (M-EXPORT.4).
    #[wasm_bindgen(js_name = __screenRevealRecordingInFileManager, catch)]
    pub async fn reveal_in_file_manager_js(path: String) -> Result<JsValue, JsValue>;
}

/// Resolve the default output path the recorder would write to if
/// the user started recording right now with the given format.
/// Returns empty string on IPC failure (caller treats as "use no
/// override").
pub async fn default_output_path(format_slug: Option<&str>) -> String {
    let arg = match format_slug {
        Some(s) => JsValue::from_str(s),
        None => JsValue::NULL,
    };
    match default_output_path_js(arg).await {
        Ok(value) => value.as_string().unwrap_or_default(),
        Err(_) => String::new(),
    }
}

/// Open the OS file manager focused on `path` (M-EXPORT.4).
pub async fn reveal_in_file_manager(path: &str) -> Result<(), String> {
    reveal_in_file_manager_js(path.to_string())
        .await
        .map(|_| ())
        .map_err(|err| js_error_string(&err))
}

/// Start a coordinated recording session. Returns the session id on
/// success.
pub async fn start_recording(config: RecordingConfigView) -> Result<u64, String> {
    let arg = serde_wasm_bindgen::to_value(&config)
        .map_err(|err| format!("encode config failed: {err}"))?;
    match start_recording_js(arg).await {
        Ok(value) => serde_wasm_bindgen::from_value(value)
            .map_err(|err| format!("decode session_id failed: {err}")),
        Err(err) => Err(js_error_string(&err)),
    }
}

/// Stop the active recording session.
pub async fn stop_recording() -> Result<RecordingSummaryView, String> {
    match stop_recording_js().await {
        Ok(value) => serde_wasm_bindgen::from_value(value)
            .map_err(|err| format!("decode summary failed: {err}")),
        Err(err) => Err(js_error_string(&err)),
    }
}

/// Synchronous mount-time snapshot of the recording status.
pub async fn recording_status() -> RecordingStatusViewIpc {
    match recording_status_js().await {
        Ok(value) => {
            serde_wasm_bindgen::from_value(value).unwrap_or_else(|_| RecordingStatusViewIpc::idle())
        }
        Err(_) => RecordingStatusViewIpc::idle(),
    }
}

fn js_error_string(err: &JsValue) -> String {
    err.as_string().unwrap_or_else(|| format!("{err:?}"))
}

// ---- M-RECORD.3 — shared "recording active" listener helper -----

/// Subscribe to the `recording-status` event and update `lock` to
/// match `RecordingStatusViewIpc::is_recording()`. Used by each of
/// the four per-channel pickers to disable their master toggle while
/// a session is `Running` / `Starting` / `Stopping` so the user
/// can't yank an input mid-record (M-RECORD.3).
///
/// Also fetches the synchronous initial status via
/// `recording_status` on mount so a picker remounted mid-session
/// (tray-popover → main window) starts in the locked state.
#[cfg(target_arch = "wasm32")]
pub fn install_recording_lock_listener(lock: leptos::prelude::RwSignal<bool>) {
    use js_sys::Reflect;
    use leptos::prelude::*;
    use leptos::task::spawn_local;

    // Initial poll.
    spawn_local(async move {
        let view = recording_status().await;
        lock.set(view.is_recording());
    });

    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(tauri_obj) = Reflect::get(&window, &JsValue::from_str("__TAURI__")) else {
        return;
    };
    let Ok(event_obj) = Reflect::get(&tauri_obj, &JsValue::from_str("event")) else {
        return;
    };
    let Ok(listen_fn) = Reflect::get(&event_obj, &JsValue::from_str("listen")) else {
        return;
    };
    if !listen_fn.is_function() {
        return;
    }
    let callback = wasm_bindgen::closure::Closure::wrap(Box::new(move |evt: JsValue| {
        let Ok(payload) = Reflect::get(&evt, &JsValue::from_str("payload")) else {
            return;
        };
        if let Ok(parsed) = serde_wasm_bindgen::from_value::<RecordingStatusViewIpc>(payload) {
            lock.set(parsed.is_recording());
        }
    }) as Box<dyn FnMut(JsValue)>);
    let listen_fn: js_sys::Function = wasm_bindgen::JsCast::unchecked_into(listen_fn);
    let _ = listen_fn.call2(
        event_obj.as_ref(),
        &JsValue::from_str("recording-status"),
        callback.as_ref().unchecked_ref(),
    );
    callback.forget();
}

/// Subscribe to the `recording-status` event and update `status` to
/// the full pushed snapshot. Companion to
/// [`install_recording_lock_listener`] — used when a component needs
/// the elapsed-ms / per-stream-health detail (e.g. the live
/// `RecorderPage` Start↔Stop cycle) rather than just the locked-or-not
/// boolean.
///
/// Also fires a one-shot synchronous poll via `recording_status` so
/// a remounted component starts with the current value rather than
/// the `RecordingStatusViewIpc::idle()` default.
#[cfg(target_arch = "wasm32")]
pub fn install_recording_status_listener(
    status: leptos::prelude::RwSignal<RecordingStatusViewIpc>,
) {
    use js_sys::Reflect;
    use leptos::prelude::*;
    use leptos::task::spawn_local;

    spawn_local(async move {
        let view = recording_status().await;
        status.set(view);
    });

    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(tauri_obj) = Reflect::get(&window, &JsValue::from_str("__TAURI__")) else {
        return;
    };
    let Ok(event_obj) = Reflect::get(&tauri_obj, &JsValue::from_str("event")) else {
        return;
    };
    let Ok(listen_fn) = Reflect::get(&event_obj, &JsValue::from_str("listen")) else {
        return;
    };
    if !listen_fn.is_function() {
        return;
    }
    let callback = wasm_bindgen::closure::Closure::wrap(Box::new(move |evt: JsValue| {
        let Ok(payload) = Reflect::get(&evt, &JsValue::from_str("payload")) else {
            return;
        };
        if let Ok(parsed) = serde_wasm_bindgen::from_value::<RecordingStatusViewIpc>(payload) {
            status.set(parsed);
        }
    }) as Box<dyn FnMut(JsValue)>);
    let listen_fn: js_sys::Function = wasm_bindgen::JsCast::unchecked_into(listen_fn);
    let _ = listen_fn.call2(
        event_obj.as_ref(),
        &JsValue::from_str("recording-status"),
        callback.as_ref().unchecked_ref(),
    );
    callback.forget();
}

/// Native (non-wasm) stub of [`install_recording_status_listener`].
#[cfg(not(target_arch = "wasm32"))]
pub fn install_recording_status_listener(
    _status: leptos::prelude::RwSignal<RecordingStatusViewIpc>,
) {
}

/// Native (non-wasm) stub of [`install_recording_lock_listener`].
/// Unit tests + non-browser builds get a no-op so the `RwSignal`
/// stays at its default `false`.
#[cfg(not(target_arch = "wasm32"))]
pub fn install_recording_lock_listener(_lock: leptos::prelude::RwSignal<bool>) {}
