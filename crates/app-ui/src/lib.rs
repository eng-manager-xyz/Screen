//! `app-ui` — the Leptos CSR app served by Trunk into the Tauri webview.
//!
//! Composes the workshopped components from
//! [`ui_storybook::components`] into the recorder's shell surface:
//!
//! - [`RecordingToolbar`](ui_storybook::components::RecordingToolbar) at top.
//! - [`DropZone`](ui_storybook::components::DropZone) when no recording is
//!   loaded.
//! - [`PlayerControls`](ui_storybook::components::PlayerControls) +
//!   placeholder preview when a recording is loaded.
//! - [`StatusBar`](ui_storybook::components::StatusBar) at the bottom.
//!
//! As of M-PLAY.2, the shell drives the screen-app player via Tauri IPC:
//! the file-drop event opens the file, the transport buttons toggle
//! play/pause, and a pushed `player-status` event keeps the UI in sync
//! with the Rust-side player. See [`player_ipc`] for the JS-bridge
//! `extern` declarations and the [`player_ipc::PlayerStatus`] mirror.

#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    reason = "Leptos `#[component]` macro rewrites these patterns; lints fire on generated code"
)]

pub mod app;
pub mod player_ipc;

use wasm_bindgen::prelude::*;

/// Trunk entry point — installs panic hooks and mounts the app to `<body>`.
///
/// Trunk wires this via the `data-trunk` `<link rel="rust">` tag in
/// `index.html`. When the WASM bundle loads, the browser calls this
/// function automatically.
#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
