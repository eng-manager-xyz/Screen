//! `screen-app` — Tauri shell library half.
//!
//! `main.rs` is the binary entry point that wires Tauri's event loop, the
//! window, the OS drag-drop forwarding, the player tick thread, and
//! registers the IPC commands. The library half holds the parts of that
//! plumbing that benefit from being testable on their own:
//!
//! - [`player_session::PlayerSession`] — owns a [`wisp::application::Application`]
//!   plus an `Option<Inner>` (built whenever a file is opened) holding the
//!   live [`playback::Player`]. Exposes `open` / `play` / `pause` / `tick`
//!   / `status`. Pure Rust; no Tauri types.
//! - [`commands`] — the four `#[tauri::command]` wrappers around
//!   `PlayerSession`. Thin — every method delegates straight through.

pub mod audio;
pub mod click_capture;
pub mod commands;
pub mod cursor_capture;
pub mod editor_command;
pub mod editor_export;
pub mod editor_preview;
pub mod editor_session;
pub mod player_session;
pub mod preview;
pub mod recorder_settings;
pub mod recording;
pub mod recording_compose;
pub mod recording_paths;
pub mod recp;
#[cfg(target_os = "macos")]
pub mod screen_capture;
#[cfg(target_os = "macos")]
pub mod system_audio;
pub mod tray;
