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
#[cfg(feature = "tray-appshell-preview")]
pub mod dev_appshell;
pub mod player_ipc;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

/// Trunk entry point — installs panic hooks and mounts the app to `<body>`.
///
/// Trunk wires this via the `data-trunk` `<link rel="rust">` tag in
/// `index.html`. When the WASM bundle loads, the browser calls this
/// function automatically.
///
/// **URL-based view selection (M-TRAY.0 / AUT-249):** when the
/// `tray-popover` Tauri window opens this bundle with `?tray=stub` in
/// the URL, we render only an empty rectangle placeholder instead of
/// the full app.
///
/// **`AppShell` CSR preview (M-TRAY.1 / AUT-250):** when built with
/// the `tray-appshell-preview` Cargo feature, the else branch mounts
/// [`dev_appshell::DevAppShellPreview`] instead of `<App />`. This is
/// a developer affordance for the audit smoke; production builds are
/// unaffected. M-TRAY.3 (AUT-252) supersedes the feature-gate with
/// the real URL-routed mount.
#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    if is_tray_stub() {
        leptos::mount::mount_to_body(|| view! { <div class="tray-popover-stub" /> });
        return;
    }
    mount_default();
}

#[cfg(not(feature = "tray-appshell-preview"))]
fn mount_default() {
    leptos::mount::mount_to_body(app::App);
}

#[cfg(feature = "tray-appshell-preview")]
fn mount_default() {
    leptos::mount::mount_to_body(dev_appshell::DevAppShellPreview);
}

/// `true` when the current page URL carries `?tray=stub`. Used to
/// short-circuit `mount_to_body` to the M-TRAY.0 popover stub.
fn is_tray_stub() -> bool {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .is_some_and(|s| s.contains("tray=stub"))
}
