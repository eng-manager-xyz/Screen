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

// The Recorder surface composes 4 picker components (Camera + Mic +
// SystemAudio + Screen) plus the preview canvas + diagnostics overlay
// + bubble toggles, all returning deeply-nested Leptos `view!{}`
// types. Type resolution overflows the default 128-frame recursion
// limit during `cargo test --no-run` (which monomorphises the
// nested `IntoAny::into_any::resolve<...>` futures). 256 is the
// compiler's suggested bump and has plenty of headroom for the next
// few picker additions before we'd need to revisit.
#![recursion_limit = "256"]
#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    reason = "Leptos `#[component]` macro rewrites these patterns; lints fire on generated code"
)]

pub mod app;
pub mod bubble;
pub mod bubble_ipc;
pub mod camera_diagnostics;
#[cfg(feature = "tray-appshell-preview")]
pub mod dev_appshell;
pub mod player_ipc;

use wasm_bindgen::prelude::*;

/// Trunk entry point — installs panic hooks and mounts the app to `<body>`.
///
/// **URL-based view selection (M-TRAY.3 / AUT-252, M-BUBBLE.0 / AUT-273):**
/// the dispatch routes on the page's query string via
/// [`routing::parse_mount_point`]:
///
/// * `?surface=<recorder|library|editor|cursor|prefs>` → full
///   ui-storybook `AppShell` rooted at the requested surface (the
///   tray-launched main window).
/// * `?mount=bubble` → the borderless `<BubbleRoot />` component for
///   the webcam-bubble window.
/// * Otherwise → existing M-INT.1 drop-zone shell (`<App />`) — keeps
///   `trunk serve` browser flow working for one-off Leptos iteration.
///
/// **`AppShell` CSR preview (M-TRAY.1 / AUT-250):** when built with
/// the `tray-appshell-preview` Cargo feature, `mount_default` (the
/// no-query branch) mounts [`dev_appshell::DevAppShellPreview`]
/// instead of `<App />`. This stays for now alongside the
/// query-routed path so the M-TRAY.1 audit smoke recipe
/// (`just dev-appshell`) keeps working — production builds without
/// the feature are unaffected.
#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    match mount_point_from_query() {
        routing::MountPoint::AppShell(section) => {
            leptos::mount::mount_to_body(move || {
                app_shell_mount::AppShellRoot(app_shell_mount::AppShellRootProps {
                    initial: section,
                })
            });
        }
        routing::MountPoint::Bubble => {
            leptos::mount::mount_to_body(bubble::BubbleRoot);
        }
        routing::MountPoint::DropZone => mount_default(),
    }
}

/// Pull the live page URL's query string, parse it via
/// [`routing::parse_mount_point`]. Falls back to
/// [`routing::MountPoint::DropZone`] outside a browser so the calling
/// site's match arm has a sensible default.
#[must_use]
pub fn mount_point_from_query() -> routing::MountPoint {
    let Some(window) = web_sys::window() else {
        return routing::MountPoint::DropZone;
    };
    let Ok(search) = window.location().search() else {
        return routing::MountPoint::DropZone;
    };
    routing::parse_mount_point(&search)
}

#[cfg(not(feature = "tray-appshell-preview"))]
fn mount_default() {
    leptos::mount::mount_to_body(app::App);
}

#[cfg(feature = "tray-appshell-preview")]
fn mount_default() {
    leptos::mount::mount_to_body(dev_appshell::DevAppShellPreview);
}

/// Parse the live page URL's `?surface=` query — thin wrapper around
/// [`routing::parse_surface`] that pulls the string from
/// `window.location.search()`. Returns `None` outside a browser
/// (so the calling site falls through to the default mount path).
#[must_use]
pub fn parse_surface_from_query() -> Option<ui_storybook::components::shell::AppSection> {
    let search = web_sys::window()?.location().search().ok()?;
    routing::parse_surface(&search)
}

/// Convert an [`AppSection`](ui_storybook::components::shell::AppSection)
/// to its URL slug — thin wrapper around [`routing::surface_slug`].
#[must_use]
pub fn surface_to_query(section: ui_storybook::components::shell::AppSection) -> &'static str {
    routing::surface_slug(section)
}

mod app_shell_mount;
pub mod camera_ipc;
pub mod camera_picker;
pub mod camera_preview;
pub mod editor_ipc;
pub mod editor_surface;
pub mod mic_ipc;
pub mod mic_picker;
pub mod recorder_controls;
pub mod recorder_page;
pub mod recording_ipc;
pub mod routing;
pub mod screen_ipc;
pub mod screen_picker;
pub mod settings_ipc;
pub mod system_audio_ipc;
pub mod system_audio_picker;
