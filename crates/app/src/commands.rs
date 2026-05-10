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

use tauri::State;

use crate::player_session::{PlayerSession, PlayerStatus};

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
