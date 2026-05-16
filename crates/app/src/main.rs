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

use std::thread;
use std::time::Duration;

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{DragDropEvent, Emitter, Manager, WindowEvent};

use screen_app::commands::{self, TrayState};
use screen_app::player_session::{PlayerSession, PlayerStatus, SessionState};

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

fn main() {
    tauri::Builder::default()
        .manage(PlayerSession::new())
        .manage(TrayState::default())
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
                ]
            }
        })
        .on_window_event(|window, event| {
            // Three event flavors flow to the webview:
            //   - file-drag-enter: drop-zone shows the active visual.
            //   - file-drag-leave: drop-zone reverts. Also emitted after
            //     a successful drop so the active visual doesn't stick.
            //   - file-dropped:    payload is the dropped path.
            //
            // Diagnostic eprintln on every drag-related event so a
            // misbehaving dev environment shows up in stderr — once
            // the chain is verified end-to-end the prints can drop to
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
        })
        .setup(|app| {
            spawn_tick_thread(app.handle().clone());
            register_tray_icon(app)?;

            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
                ..
            } = event
            {
                let app = tray.app_handle();
                let state = app.state::<TrayState>();
                commands::toggle_tray_popover(app, &state);
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
