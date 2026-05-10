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

use tauri::{DragDropEvent, Emitter, Manager, WindowEvent};

use screen_app::commands;
use screen_app::player_session::{PlayerSession, PlayerStatus, SessionState};

const TICK_INTERVAL: Duration = Duration::from_millis(33);

/// Granularity at which `elapsed_ms` change alone triggers a status emit
/// (every 100 ms = 10 Hz UI updates while playing). Without this throttle
/// every tick would emit, hammering the webview event bridge.
const ELAPSED_EMIT_GRANULARITY_MS: u64 = 100;

fn main() {
    tauri::Builder::default()
        .manage(PlayerSession::new())
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
                ]
            }
        })
        .on_window_event(|window, event| {
            // Three event flavors flow to the webview:
            //   - file-drag-enter: drop-zone shows the active visual.
            //   - file-drag-leave: drop-zone reverts. Also emitted after
            //     a successful drop so the active visual doesn't stick.
            //   - file-dropped:    payload is the dropped path.
            if let WindowEvent::DragDrop(drag) = event {
                match drag {
                    DragDropEvent::Enter { .. } => {
                        if let Err(err) = window.emit("file-drag-enter", ()) {
                            eprintln!("failed to emit file-drag-enter event: {err}");
                        }
                    }
                    DragDropEvent::Leave => {
                        if let Err(err) = window.emit("file-drag-leave", ()) {
                            eprintln!("failed to emit file-drag-leave event: {err}");
                        }
                    }
                    DragDropEvent::Drop { paths, .. } => {
                        if let Some(path) = paths.first() {
                            let payload = path.to_string_lossy().into_owned();
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

            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
