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

pub mod commands;
pub mod player_session;
pub mod tray;
