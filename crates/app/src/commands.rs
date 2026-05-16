//! Tauri `#[command]` wrappers around [`PlayerSession`].
//!
//! Every command is a one-liner — the heavy lifting is in
//! [`super::player_session`]. Splitting them out keeps the IPC surface
//! easy to audit (one file, four functions) and isolates the Tauri
//! framework dep from the testable `PlayerSession`.

#![allow(
    clippy::needless_pass_by_value,
    reason = "tauri::command requires State<T> by value (the macro's signature inspection rejects &State<T>)"
)]

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{Manager, State};

use crate::player_session::{PlayerSession, PlayerStatus};
use crate::preview::{CameraError, PreviewLifecycle, PreviewState};
use crate::tray::toggle::{Action, TrayPopoverState};

/// Tauri-managed wrapper around the tray-popover toggle state machine
/// (M-TRAY.0 / AUT-249). Held in `tauri::State` so the click handler in
/// `main.rs` and the `tray_toggle_popover` command share one source of
/// truth. `Mutex` rather than `parking_lot::Mutex` to avoid adding a new
/// workspace dep just for the tray; contention is non-existent (only
/// the click handler ever touches it).
#[derive(Default)]
pub struct TrayState(pub Mutex<TrayPopoverState>);

/// Tray-popover toggle command (M-TRAY.0 / AUT-249).
///
/// Resolves the bound popover window by its `tauri.conf.json` label
/// (`tray-popover`), advances the state machine, then performs the
/// corresponding `show()`/`hide()` on the window. Returning `()` rather
/// than `Result` matches the existing player_* commands' shape — failure
/// paths log via `tracing::warn!` so the click is never user-facing
/// silent.
#[tauri::command]
pub fn tray_toggle_popover(app: tauri::AppHandle, state: State<'_, TrayState>) {
    toggle_tray_popover(&app, &state);
}

/// Pure function variant of [`tray_toggle_popover`] — not a Tauri
/// command. The click handler in `main.rs` calls this directly so it
/// doesn't have to round-trip through the IPC command bus.
pub fn toggle_tray_popover(app: &tauri::AppHandle, state: &TrayState) {
    let Some(window) = app.get_webview_window("tray-popover") else {
        tracing::warn!("tray-popover window not found; tauri.conf.json may be missing it");
        return;
    };
    let action = {
        let mut guard = state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.on_click()
    };
    match action {
        Action::Show => {
            if let Err(err) = window.show() {
                tracing::warn!(?err, "failed to show tray-popover window");
                return;
            }
            if let Err(err) = window.set_focus() {
                tracing::warn!(?err, "failed to focus tray-popover window");
            }
        }
        Action::Hide => {
            if let Err(err) = window.hide() {
                tracing::warn!(?err, "failed to hide tray-popover window");
            }
        }
    }
}

/// Open a video file and start it paused at frame 0.
#[tauri::command]
pub fn player_open(state: State<'_, PlayerSession>, path: String) -> Result<PlayerStatus, String> {
    state.open(&PathBuf::from(path))
}

/// Resume playback. No-op when nothing is loaded.
#[tauri::command]
pub fn player_play(state: State<'_, PlayerSession>) {
    state.play();
}

/// Pause playback. No-op when nothing is loaded.
#[tauri::command]
pub fn player_pause(state: State<'_, PlayerSession>) {
    state.pause();
}

/// Snapshot the current status. The shell normally subscribes to the
/// pushed `player-status` events instead of polling, but this command
/// is useful on initial mount to seed the UI before the first event.
#[tauri::command]
#[must_use]
pub fn player_status(state: State<'_, PlayerSession>) -> PlayerStatus {
    state.status()
}

/// View-model shape for the camera-list IPC command (M-CAM.2 /
/// AUT-256). Mirrors `media::CameraDevice` but lives in
/// `crates/app/` so the IPC schema is owned by the shell crate
/// rather than the media crate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraView {
    /// Stable device id.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// First in the enumeration order.
    pub is_default: bool,
}

impl From<media::CameraDevice> for CameraView {
    fn from(value: media::CameraDevice) -> Self {
        Self {
            id: value.id,
            label: value.label,
            is_default: value.is_default,
        }
    }
}

/// Camera permission probe (M-CAM.2 / AUT-256). Stub-returns
/// `Granted` on every platform today; full macOS implementation via
/// `AVCaptureDevice.authorizationStatus(for:)` is M-RECP.0 territory.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum CameraPermission {
    /// User has granted camera access.
    Granted,
    /// macOS has not yet asked the user — the next capture call
    /// will trigger the OS-level prompt.
    NotDetermined,
    /// User has explicitly denied access.
    Denied,
}

/// Enumerate attached cameras (M-CAM.2 / AUT-256).
///
/// Wraps `media::list_cameras()` and converts each `CameraDevice`
/// to a `CameraView`. Returns an empty `Vec` (not an error) if
/// `gst-device-monitor-1.0` isn't on `PATH` or no cameras are
/// attached — Leptos consumers should runtime-skip in that case.
#[tauri::command]
#[must_use]
pub fn list_cameras() -> Vec<CameraView> {
    media::list_cameras()
        .into_iter()
        .map(CameraView::from)
        .collect()
}

/// Probe the OS for camera permission (M-CAM.2 / AUT-256).
///
/// Today: returns `Granted` everywhere. The real macOS
/// implementation lands in M-RECP.0 (AUT-261) via `objc2` calls
/// into `AVCaptureDevice.authorizationStatus(for: .video)`.
#[tauri::command]
#[must_use]
pub fn camera_permission_status() -> CameraPermission {
    CameraPermission::Granted
}

/// Start the camera preview pipeline (M-CAM.2 / AUT-256).
///
/// Today: pure state-machine transition. M-CAM.3 (AUT-257) fills in
/// the actual gst → wisp → readback → frame-channel pipeline behind
/// this transition.
///
/// # Errors
///
/// Returns [`CameraError`] when the state machine refuses (already
/// running) or — once M-CAM.3 lands — when the gst pipeline fails
/// to produce frames.
#[tauri::command]
pub fn start_preview(state: State<'_, PreviewState>, camera_id: String) -> Result<(), CameraError> {
    let mut guard = state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let new_state = guard.try_start();
    if new_state == *guard {
        // Already starting / running / stopping — nothing to do.
        return Ok(());
    }
    *guard = new_state;
    tracing::info!(camera_id = %camera_id, "preview Starting (state-only stub; M-CAM.3 wires the pipeline)");
    // M-CAM.3 will set state to Running after the first frame
    // arrives. For now mark Running immediately so single-process
    // smoke tests can observe a stable state.
    *guard = guard.mark_running();
    Ok(())
}

/// Stop the camera preview pipeline (M-CAM.2 / AUT-256).
///
/// State-machine only today; M-CAM.3 wires the actual gst child
/// kill + wisp Stage drop here.
#[tauri::command]
pub fn stop_preview(state: State<'_, PreviewState>) {
    let mut guard = state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = guard.try_stop();
    *guard = guard.finish_stop();
    tracing::info!("preview Stopped (state-only stub; M-CAM.3 wires the teardown)");
}

/// Snapshot the current preview lifecycle (M-CAM.2 / AUT-256).
///
/// Useful for Leptos to seed its `RecorderPreviewState` enum on
/// first mount before the pushed frame events drive it.
#[tauri::command]
#[must_use]
pub fn preview_status(state: State<'_, PreviewState>) -> PreviewLifecycle {
    *state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Test-only entry point for `WebDriver` e2e suites. Emits a
/// `file-dropped` event with the same shape as the real OS drag-drop
/// handler in `main.rs`. Gated on `debug_assertions` so it's stripped
/// from release builds; `main.rs` likewise registers it conditionally
/// in `generate_handler!`.
///
/// Why this exists: `WebDriver` clients can't synthesize OS-level
/// drag-drop events. Without this command, the e2e tests would have
/// to use platform-specific tools (`xdotool` on Linux, etc.) which are
/// fragile and don't help the rest of the test suite.
///
/// # Errors
///
/// Returns the underlying [`tauri::Error`] message string if the event
/// emit fails (no listeners is not an error — `Emitter::emit` returns
/// `Ok(())` regardless).
#[cfg(debug_assertions)]
#[tauri::command]
pub fn __test_drop_file(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri::Emitter;
    app.emit("file-dropped", path).map_err(|e| e.to_string())
}

/// Test-only entry point: synthesize a `DragDropEvent::Enter` for the
/// `WebDriver` e2e suite. Emits the same `file-drag-enter` event as the
/// real OS drag-enter handler. Debug-only, parallel to [`__test_drop_file`].
///
/// # Errors
///
/// Returns the underlying [`tauri::Error`] message if the event emit
/// fails.
#[cfg(debug_assertions)]
#[tauri::command]
pub fn __test_drag_enter(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    app.emit("file-drag-enter", ()).map_err(|e| e.to_string())
}

/// Test-only entry point: synthesize a `DragDropEvent::Leave`.
/// Pair with [`__test_drag_enter`].
///
/// # Errors
///
/// Returns the underlying [`tauri::Error`] message if the event emit
/// fails.
#[cfg(debug_assertions)]
#[tauri::command]
pub fn __test_drag_leave(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    app.emit("file-drag-leave", ()).map_err(|e| e.to_string())
}
