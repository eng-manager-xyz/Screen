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
