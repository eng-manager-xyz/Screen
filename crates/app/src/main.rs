//! Tauri 2 shell for the screen recorder.
//!
//! Listens for OS-level drag-drop events on the main window and forwards
//! the dropped file path to the Leptos webview via Tauri's event system.
//! The webview's JS bridge re-emits as a `CustomEvent("file-dropped")` so
//! Leptos can listen via plain `web_sys`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{DragDropEvent, Emitter, Manager, WindowEvent};

fn main() {
    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event
                && let Some(path) = paths.first()
            {
                let payload = path.to_string_lossy().into_owned();
                if let Err(err) = window.emit("file-dropped", payload) {
                    eprintln!("failed to emit file-dropped event: {err}");
                }
            }
        })
        .setup(|app| {
            // Make sure the main window is visible on launch.
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
