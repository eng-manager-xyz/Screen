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
