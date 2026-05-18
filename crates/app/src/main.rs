//! Tauri 2 shell for the screen recorder.
//!
//! The shell wires three things into Tauri's event loop:
//!
//! 1. **OS file drops** — `on_window_event` listens for `WindowEvent::DragDrop`
//!    and emits a `file-dropped` Tauri event carrying the dropped path.
//! 2. **Player IPC** — registers [`screen_app::player_session::PlayerSession`]
//!    via `.manage()` and exposes the four `player_*` commands.
//! 3. **Tick thread** — a single OS thread ticks the player every ~33 ms
//!    and emits `player-status` events whenever the state changes (or
//!    every 100 ms of elapsed change while playing).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// macOS Info.plist embedding is handled by `tauri::generate_context!()`
// — tauri-codegen 2.6.1 auto-embeds `crates/app/Info.plist` into the
// binary's `__TEXT,__info_plist` section for every debug build on
// macOS (see `tauri-codegen::context::context_codegen`, branch
// `target == Target::MacOS && dev && !running_tests`). The manual
// `embed_plist::embed_info_plist!` call we previously had emitted
// the SAME `_EMBED_INFO_PLIST` symbol, which made every integration
// test fail at link time with `symbol _EMBED_INFO_PLIST is already
// defined`. Bundled `.app` builds (when `bundle.active = true`) pick
// up the same `Info.plist` via `bundle.macOS.infoPlist` in
// `tauri.conf.json`.

use std::thread;
use std::time::Duration;

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{DragDropEvent, Emitter, Manager, WindowEvent};

use screen_app::audio::{MicCaptureHandle, MicCaptureState};
use screen_app::commands::{self, BubbleState, TrayState};
use screen_app::player_session::{PlayerSession, PlayerStatus, SessionState};
use screen_app::preview::{CameraPipelineHandle, PreviewDiagnostics, PreviewState};
use screen_app::recording::RecordingState;
#[cfg(target_os = "macos")]
use screen_app::screen_capture::ScreenCaptureState;
#[cfg(target_os = "macos")]
use screen_app::system_audio::SystemAudioCaptureState;

/// Embedded tray-icon bytes (M-TRAY.0 / AUT-249). `include_bytes!`
/// resolves at compile time so the bundled binary doesn't need
/// filesystem access to render the icon at runtime. Regenerated from
/// `icons/tray.svg` by `cargo run -p screen-app --example
/// regen-tray-icons`.
const TRAY_ICON_PNG: &[u8] = include_bytes!("../icons/tray.png");

const TICK_INTERVAL: Duration = Duration::from_millis(33);

/// Granularity at which `elapsed_ms` change alone triggers a status emit
/// (every 100 ms = 10 Hz UI updates while playing). Without this throttle
/// every tick would emit, hammering the webview event bridge.
const ELAPSED_EMIT_GRANULARITY_MS: u64 = 100;

#[allow(
    clippy::too_many_lines,
    reason = "fn main() is a Tauri builder chain — the bulk is two cfg-gated generate_handler! arms listing every IPC command (now 30+). Splitting them into helpers doesn't reduce complexity; the lints fire on growth, not nesting."
)]
fn main() {
    let builder = tauri::Builder::default()
        .manage(PlayerSession::new())
        .manage(TrayState::default())
        .manage(BubbleState::default())
        .manage(PreviewState::default())
        .manage(PreviewDiagnostics::default())
        .manage(CameraPipelineHandle::default())
        .manage(MicCaptureState::default())
        .manage(MicCaptureHandle::default())
        .manage(RecordingState::default());
    // System-audio + screen-capture states are macOS-only — both
    // depend on ScreenCaptureKit.
    #[cfg(target_os = "macos")]
    let builder = builder
        .manage(SystemAudioCaptureState::default())
        .manage(ScreenCaptureState::default());
    builder
        .invoke_handler({
            // Debug builds expose `__test_drop_file` for WebDriver e2e
            // tests (M-TEST.2). Release builds omit it entirely — the
            // command is `#[cfg(debug_assertions)]` and so is its
            // registration here.
            #[cfg(debug_assertions)]
            {
                tauri::generate_handler![
                    commands::player_open,
                    commands::player_play,
                    commands::player_pause,
                    commands::player_status,
                    commands::tray_toggle_popover,
                    commands::toggle_webcam_bubble,
                    commands::set_bubble_clickthrough,
                    commands::list_cameras,
                    commands::camera_permission_status,
                    commands::start_preview,
                    commands::stop_preview,
                    commands::preview_status,
                    commands::preview_diagnostics,
                    commands::list_microphones,
                    commands::start_mic_capture,
                    commands::stop_mic_capture,
                    commands::mic_status,
                    commands::microphone_permission_status,
                    commands::open_settings_camera,
                    commands::open_settings_microphone,
                    commands::open_settings_screen_recording,
                    commands::list_audio_apps,
                    commands::start_system_audio_capture,
                    commands::stop_system_audio_capture,
                    commands::set_system_audio_filter,
                    commands::system_audio_status,
                    commands::list_screen_displays,
                    commands::list_screen_windows,
                    commands::start_screen_capture,
                    commands::stop_screen_capture,
                    commands::screen_capture_status,
                    commands::screen_capture_frame_count,
                    commands::start_recording,
                    commands::stop_recording,
                    commands::recording_status,
                    commands::default_recording_output_path,
                    commands::reveal_recording_in_file_manager,
                    commands::latest_camera_frame_bgra,
                    commands::request_all_permissions,
                    commands::__test_drop_file,
                    commands::__test_drag_enter,
                    commands::__test_drag_leave,
                ]
            }
            #[cfg(not(debug_assertions))]
            {
                tauri::generate_handler![
                    commands::player_open,
                    commands::player_play,
                    commands::player_pause,
                    commands::player_status,
                    commands::tray_toggle_popover,
                    commands::toggle_webcam_bubble,
                    commands::set_bubble_clickthrough,
                    commands::list_cameras,
                    commands::camera_permission_status,
                    commands::start_preview,
                    commands::stop_preview,
                    commands::preview_status,
                    commands::preview_diagnostics,
                    commands::list_microphones,
                    commands::start_mic_capture,
                    commands::stop_mic_capture,
                    commands::mic_status,
                    commands::microphone_permission_status,
                    commands::open_settings_camera,
                    commands::open_settings_microphone,
                    commands::open_settings_screen_recording,
                    commands::list_audio_apps,
                    commands::start_system_audio_capture,
                    commands::stop_system_audio_capture,
                    commands::set_system_audio_filter,
                    commands::system_audio_status,
                    commands::list_screen_displays,
                    commands::list_screen_windows,
                    commands::start_screen_capture,
                    commands::stop_screen_capture,
                    commands::screen_capture_status,
                    commands::screen_capture_frame_count,
                    commands::start_recording,
                    commands::stop_recording,
                    commands::recording_status,
                    commands::default_recording_output_path,
                    commands::reveal_recording_in_file_manager,
                    commands::latest_camera_frame_bgra,
                    commands::request_all_permissions,
                ]
            }
        })
        .on_window_event(handle_window_event)
        .setup(|app| {
            spawn_tick_thread(app.handle().clone());
            register_tray_icon(app)?;
            // The legacy `main` window stays declared in tauri.conf.json
            // because the M1 player IPC commands target it by label, but
            // it boots hidden (`"visible": false`) and stays hidden — the
            // recorder UX is tray-only. Showing it explicitly here was
            // the source of the "I see a blank window on launch" bug.
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Per-window event dispatcher. Extracted from `main()`'s closure so
/// the file's top-level builder stays inside clippy's
/// `too_many_lines` threshold once the mic + audio command surface
/// lands (M-MIC.1 / AUT-278). Behaviour is unchanged: bubble drag
/// position cache (M-BUBBLE.3) + drag-drop event forwarding.
fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    // M-BUBBLE.3 / AUT-276: keep the webcam-bubble's in-memory
    // position cache in sync while the user drags. Disk persistence
    // happens on Hide (see `toggle_webcam_bubble`), not here —
    // `WindowEvent::Moved` fires per-frame during a macOS drag and
    // writing to disk at that rate would burn I/O. The in-memory
    // write is a single mutex acquire + i32 pair copy, cheap enough
    // for the high-frequency path.
    if let WindowEvent::Moved(physical) = event
        && window.label() == "webcam-bubble"
    {
        let state = window.app_handle().state::<commands::BubbleState>();
        commands::update_bubble_position_from_event(&state, physical.x, physical.y);
    }
    // Three event flavors flow to the webview:
    //   - file-drag-enter: drop-zone shows the active visual.
    //   - file-drag-leave: drop-zone reverts. Also emitted after a
    //     successful drop so the active visual doesn't stick.
    //   - file-dropped:    payload is the dropped path.
    //
    // Diagnostic eprintln on every drag-related event so a
    // misbehaving dev environment shows up in stderr — once the
    // chain is verified end-to-end the prints can drop to
    // `tracing::trace!` (or come out entirely).
    if let WindowEvent::DragDrop(drag) = event {
        match drag {
            DragDropEvent::Enter { paths, .. } => {
                eprintln!(
                    "tauri: DragDropEvent::Enter ({} path(s)) — emitting file-drag-enter",
                    paths.len()
                );
                if let Err(err) = window.emit("file-drag-enter", ()) {
                    eprintln!("failed to emit file-drag-enter event: {err}");
                }
            }
            DragDropEvent::Leave => {
                eprintln!("tauri: DragDropEvent::Leave — emitting file-drag-leave");
                if let Err(err) = window.emit("file-drag-leave", ()) {
                    eprintln!("failed to emit file-drag-leave event: {err}");
                }
            }
            DragDropEvent::Drop { paths, .. } => {
                eprintln!("tauri: DragDropEvent::Drop ({} path(s))", paths.len());
                if let Some(path) = paths.first() {
                    let payload = path.to_string_lossy().into_owned();
                    eprintln!("tauri: emitting file-dropped → {payload}");
                    if let Err(err) = window.emit("file-dropped", payload) {
                        eprintln!("failed to emit file-dropped event: {err}");
                    }
                }
                // Reset the drag visual after a successful drop.
                if let Err(err) = window.emit("file-drag-leave", ()) {
                    eprintln!("failed to emit file-drag-leave event: {err}");
                }
            }
            _ => {} // DragDropEvent is non_exhaustive (Over, future variants).
        }
    }
}

/// Register the menubar / tray icon (M-TRAY.0 / AUT-249).
///
/// Left-click toggles the `tray-popover` window via the state machine
/// at [`screen_app::tray::toggle::TrayPopoverState`]. The icon is
/// rendered as a macOS template image so the OS handles light/dark
/// menubar tinting automatically; on Windows + Linux the raw PNG is
/// used as-is (template semantics are macOS-only).
///
/// Errors propagate up to Tauri's `setup` so the app fails to boot
/// rather than silently launching without a tray icon — the icon IS
/// the user's only entry point to the popover today.
fn register_tray_icon(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // `Image::from_bytes` requires Tauri's `image-png` feature — see
    // `Cargo.toml`. It decodes the PNG to raw RGBA at load time.
    let icon = tauri::image::Image::from_bytes(TRAY_ICON_PNG)?;
    let tray = TrayIconBuilder::with_id("tray-popover-icon")
        .icon(icon)
        .icon_as_template(true)
        .on_tray_icon_event(|tray, event| {
            // Filter to left-mouse-up only — ignore right-click
            // (future M-TRAY.5+ context menu), button-down events,
            // hover-enter/leave, and the future motion variants
            // that `TrayIconEvent` is `#[non_exhaustive]` for.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                position,
                ..
            } = event
            {
                // `position` is the icon's physical screen coords
                // (PhysicalPosition<f64>). We hand it to
                // `toggle_tray_popover_at` which anchors the popover
                // below the click before show().
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "monitor coordinates fit comfortably in i32 — we'd need a >2.1Bpx screen layout to overflow"
                )]
                let click = (position.x as i32, position.y as i32);
                let app = tray.app_handle();
                let state = app.state::<TrayState>();
                commands::toggle_tray_popover_at(app, &state, Some(click));
            }
        })
        .build(app)?;
    // Persist via Tauri-managed state so the icon survives `setup` return.
    app.manage(tray);
    tracing::info!("tray icon registered");
    Ok(())
}

/// Spin up the tick thread. Lives for the lifetime of the process.
fn spawn_tick_thread(app_handle: tauri::AppHandle) {
    thread::spawn(move || {
        let mut last: Option<PlayerStatus> = None;
        loop {
            thread::sleep(TICK_INTERVAL);

            let session = app_handle.state::<PlayerSession>();
            session.tick();
            let status = session.status();

            if status_changed(last.as_ref(), &status) {
                if let Err(err) = app_handle.emit("player-status", &status) {
                    tracing::warn!(?err, "failed to emit player-status event");
                }
                last = Some(status);
            }
        }
    });
}

/// `true` iff this status differs from the last-emitted one in a way the
/// webview cares about: lifecycle change, dimension/fps change after open,
/// or `elapsed_ms` change crossing a 100 ms boundary while playing.
fn status_changed(last: Option<&PlayerStatus>, current: &PlayerStatus) -> bool {
    let Some(last) = last else { return true };
    if last.state != current.state
        || last.width != current.width
        || last.height != current.height
        || last.duration_ms != current.duration_ms
    {
        return true;
    }
    if current.state == SessionState::Playing
        && last.elapsed_ms / ELAPSED_EMIT_GRANULARITY_MS
            != current.elapsed_ms / ELAPSED_EMIT_GRANULARITY_MS
    {
        return true;
    }
    false
}
