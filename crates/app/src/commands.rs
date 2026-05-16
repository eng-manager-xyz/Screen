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

use tauri::{LogicalPosition, Manager, State};

use crate::player_session::{PlayerSession, PlayerStatus};
use crate::preview::{CameraError, PreviewLifecycle, PreviewState};
use crate::recp::tray_positioning::{MonitorBounds, pick_monitor, position_window_below_click};
use crate::tray::bubble_toggle::{BubbleAction, BubbleVisibility};
use crate::tray::toggle::{Action, TrayPopoverState};

/// Tauri-managed wrapper around the tray-popover toggle state machine
/// (M-TRAY.0 / AUT-249). Held in `tauri::State` so the click handler in
/// `main.rs` and the `tray_toggle_popover` command share one source of
/// truth. `Mutex` rather than `parking_lot::Mutex` to avoid adding a new
/// workspace dep just for the tray; contention is non-existent (only
/// the click handler ever touches it).
#[derive(Default)]
pub struct TrayState(pub Mutex<TrayPopoverState>);

/// Tauri-managed wrapper around the webcam-bubble visibility state
/// machine (M-BUBBLE.0 / AUT-273). Mirrors [`TrayState`]'s shape; the
/// "Show webcam bubble" button in the Recorder surface invokes
/// `toggle_webcam_bubble` which mutates this state.
#[derive(Default)]
pub struct BubbleState(pub Mutex<BubbleVisibility>);

/// Webcam-bubble toggle command (M-BUBBLE.0 / AUT-273).
///
/// Resolves the bound bubble window by its `tauri.conf.json` label
/// (`webcam-bubble`), advances the state machine, then performs the
/// corresponding `show()`/`hide()` on the window. Returns `()` to
/// match the existing tray-toggle command shape — failures log via
/// `tracing::warn!` so the click is never user-facing silent.
///
/// Notably does NOT call `set_focus()` on show — the bubble is a
/// peripheral overlay and shouldn't steal focus from the `AppShell`.
#[tauri::command]
pub fn toggle_webcam_bubble(app: tauri::AppHandle, state: State<'_, BubbleState>) {
    let Some(window) = app.get_webview_window("webcam-bubble") else {
        tracing::warn!("webcam-bubble window not found; tauri.conf.json may be missing it");
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
        BubbleAction::Show => {
            if let Err(err) = window.show() {
                tracing::warn!(?err, "failed to show webcam-bubble window");
            }
        }
        BubbleAction::Hide => {
            if let Err(err) = window.hide() {
                tracing::warn!(?err, "failed to hide webcam-bubble window");
            }
        }
    }
}

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
/// command. Calls [`toggle_tray_popover_at`] with no click position so
/// the window opens at its previous position (or the OS-default
/// position on first show). Used by the IPC bus and the no-position
/// fallback for synthetic clicks in tests.
pub fn toggle_tray_popover(app: &tauri::AppHandle, state: &TrayState) {
    toggle_tray_popover_at(app, state, None);
}

/// Like [`toggle_tray_popover`] but anchors the popover under
/// `click_position` (M-RECP.1 / AUT-262 wiring). When the state
/// machine resolves to `Action::Show` AND `click_position` is set,
/// we look up the monitor the click happened on, compute the
/// below-click anchor, and `set_position` BEFORE showing the window.
/// Without the explicit `set_position` Tauri restores the last-known
/// position (or the OS default), which is the source of the "popover
/// doesn't follow the tray icon" bug.
pub fn toggle_tray_popover_at(
    app: &tauri::AppHandle,
    state: &TrayState,
    click_position: Option<(i32, i32)>,
) {
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
            if let Some((click_x, click_y)) = click_position {
                anchor_window_to_click(app, &window, click_x, click_y);
            }
            if let Err(err) = window.show() {
                tracing::warn!(?err, "failed to show tray-popover window");
                return;
            }
            if let Err(err) = window.set_focus() {
                tracing::warn!(?err, "failed to focus tray-popover window");
            }
            // Debug builds: surface the webview console so we can
            // diagnose blank-page / wasm-panic regressions without a
            // bundled-app context-menu DevTools toggle.
            #[cfg(debug_assertions)]
            window.open_devtools();
        }
        Action::Hide => {
            if let Err(err) = window.hide() {
                tracing::warn!(?err, "failed to hide tray-popover window");
            }
        }
    }
}

/// Pick the right monitor for `(click_x, click_y)` and place the
/// `tray-popover` window's top-left below the click. Logs and bails
/// out without setting a position when monitors can't be queried —
/// the window will still `show()` at its last-known position so the
/// user doesn't lose access to the recorder.
fn anchor_window_to_click(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    click_x: i32,
    click_y: i32,
) {
    let monitors = match app.available_monitors() {
        Ok(list) => list,
        Err(err) => {
            tracing::warn!(
                ?err,
                "available_monitors failed; popover stays at last position"
            );
            return;
        }
    };
    let bounds: Vec<MonitorBounds> = monitors
        .iter()
        .map(|m| MonitorBounds {
            x: m.position().x,
            y: m.position().y,
            width: i32::try_from(m.size().width).unwrap_or(i32::MAX),
            height: i32::try_from(m.size().height).unwrap_or(i32::MAX),
        })
        .collect();
    // `inner_size()` gives the size of the webview content rect.
    // Falling through on lookup failure uses the conf-declared size
    // as a crude fallback so we still get a sensible anchor.
    let (window_w, window_h) = window.inner_size().map_or((1200, 720), |size| {
        (
            i32::try_from(size.width).unwrap_or(1200),
            i32::try_from(size.height).unwrap_or(720),
        )
    });
    let Some((target_x, target_y)) =
        compute_popover_anchor(click_x, click_y, window_w, window_h, &bounds)
    else {
        tracing::warn!("no monitors reported; popover stays at last position");
        return;
    };
    if let Err(err) = window.set_position(LogicalPosition::new(
        f64::from(target_x),
        f64::from(target_y),
    )) {
        tracing::warn!(?err, "set_position on tray-popover failed");
    }
}

/// Pure compute step shared by [`anchor_window_to_click`] (runtime)
/// and the unit tests (no Tauri). Returns the popover's target
/// top-left position in screen coordinates, or `None` if the
/// monitor list is empty.
///
/// Splitting this out exists so the click → monitor-pick →
/// below-click clamp pipeline is verifiable without spinning up a
/// Tauri mock app — see the unit tests below.
fn compute_popover_anchor(
    click_x: i32,
    click_y: i32,
    window_w: i32,
    window_h: i32,
    monitors: &[MonitorBounds],
) -> Option<(i32, i32)> {
    let monitor = pick_monitor(click_x, click_y, monitors)?;
    Some(position_window_below_click(
        click_x, click_y, window_w, window_h, monitor,
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(x: i32, y: i32, w: i32, h: i32) -> MonitorBounds {
        MonitorBounds {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn anchor_returns_none_when_no_monitors() {
        assert_eq!(compute_popover_anchor(500, 12, 800, 600, &[]), None);
    }

    #[test]
    fn anchor_aligns_popover_right_edge_with_click() {
        let monitors = vec![mon(0, 0, 1920, 1080)];
        // Typical menubar click near the top-right of a 1920-wide screen.
        let (x, y) = compute_popover_anchor(1820, 12, 800, 600, &monitors).expect("Some(_)");
        // Right-anchored: window's right edge at click_x (1820), so
        // top-left x = 1820 - 800 = 1020. Below the click by 4px: y = 16.
        assert_eq!(x, 1020);
        assert_eq!(y, 16);
    }

    #[test]
    fn anchor_picks_secondary_monitor_for_a_click_on_it() {
        // Two side-by-side 1920×1080 monitors. A click at x=3820 lives
        // in the second monitor's top-right; the popover anchors there.
        let monitors = vec![mon(0, 0, 1920, 1080), mon(1920, 0, 1920, 1080)];
        let (x, _) = compute_popover_anchor(3820, 12, 800, 600, &monitors).expect("Some(_)");
        // raw_x = 3820 - 800 = 3020; monitor 2 spans [1920..3840];
        // max_x = 3840 - 800 = 3040; 3020 is within [1920, 3040].
        assert_eq!(x, 3020);
    }

    #[test]
    fn anchor_clamps_left_when_click_is_near_origin() {
        let monitors = vec![mon(0, 0, 1920, 1080)];
        // Click near x=0 (e.g. dev menubar arrangement) — raw_x would
        // be negative; clamp pulls left edge to monitor.x.
        let (x, _) = compute_popover_anchor(50, 12, 800, 600, &monitors).expect("Some(_)");
        assert_eq!(x, 0);
    }
}
