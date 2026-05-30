//! Leptos-side bindings for the editor IPC (ED.5 / M-EDIT).
//!
//! `app-ui` is a WASM crate and can't depend on `screen-app` (Tauri-
//! native), but it *can* depend on the pure `edit` crate — so the
//! `open_in_editor` command's payload deserializes straight into
//! [`edit::EditProject`] with no hand-mirrored type.
//!
//! The flow mirrors the player IPC: a fire-and-forget invoke wrapper
//! ([`screen_open_in_editor`], bound to the `__screenOpenInEditor` helper
//! in `index.html`) plus an `editor-project` `CustomEvent` listener that
//! pushes the loaded project into a Leptos signal.

use edit::EditProject;
use leptos::prelude::{RwSignal, Set};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::CustomEvent;

#[wasm_bindgen]
extern "C" {
    /// Ask the shell to open `path` in the editor. Fire-and-forget: the
    /// loaded project arrives asynchronously as an `editor-project`
    /// browser `CustomEvent` (see [`install_editor_project_listener`]).
    /// `catch` so it degrades to a no-op outside Tauri.
    #[wasm_bindgen(js_namespace = window, js_name = "__screenOpenInEditor", catch)]
    pub fn screen_open_in_editor(path: &str) -> Result<JsValue, JsValue>;
}

/// Install an `editor-project` `CustomEvent` listener that deserializes
/// the loaded [`EditProject`] and pushes it into `project`.
///
/// The closure has app-lifetime, so it is leaked via `Closure::forget`.
pub fn install_editor_project_listener(project: RwSignal<Option<EditProject>>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if let Ok(custom) = event.dyn_into::<CustomEvent>()
            && let Ok(loaded) = serde_wasm_bindgen::from_value::<EditProject>(custom.detail())
        {
            project.set(Some(loaded));
        }
    }) as Box<dyn FnMut(_)>);
    let _ =
        window.add_event_listener_with_callback("editor-project", closure.as_ref().unchecked_ref());
    closure.forget();
}

/// IPC-stable mirror of `screen_app::editor_session::EditorStatusView`
/// (plain-text reference — `app-ui` can't depend on the Tauri-native
/// crate). Field names must match the Rust-side `Serialize` shape.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct EditorStatus {
    /// Current playhead frame.
    pub current_frame: u64,
    /// Total project length in frames.
    pub duration_frames: u64,
    /// Whether the clock is advancing.
    pub playing: bool,
    /// Project frame rate.
    #[serde(default)]
    pub fps: u32,
    /// Playback rate multiplier.
    #[serde(default)]
    pub rate: f32,
    /// In-point (inclusive).
    pub in_frame: u64,
    /// Out-point (exclusive).
    pub out_frame: u64,
    /// Whether looping is enabled.
    pub looping: bool,
}

/// Mirror of the backend `TransportAction` (serialize side). The JS bridge
/// forwards this object to the `editor_transport` command, which
/// re-deserializes it on the Rust side.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransportAction {
    /// Start advancing.
    Play,
    /// Stop advancing.
    Pause,
    /// Toggle play/pause.
    TogglePlay,
    /// Advance the clock by `dt_ms` (the UI's per-frame tick).
    Tick {
        /// Elapsed milliseconds.
        dt_ms: u32,
    },
    /// Seek to an exact frame.
    Seek {
        /// Target frame.
        frame: u64,
    },
    /// Step `delta` frames (negative = back) and pause.
    Step {
        /// Frame delta.
        delta: i64,
    },
    /// Set the playback rate.
    SetRate {
        /// New rate.
        rate: f32,
    },
    /// Set in/out points.
    SetInOut {
        /// One bound.
        a: u64,
        /// The other bound.
        b: u64,
    },
    /// Clear in/out points.
    ClearInOut,
    /// Enable/disable looping.
    SetLooping {
        /// Loop flag.
        looping: bool,
    },
    /// Update the project length after a duration-changing edit (ripple).
    SetDuration {
        /// New total project length in frames.
        frames: u64,
    },
    /// No-op — read the current status (mirrors the backend variant).
    Status,
}

#[wasm_bindgen]
extern "C" {
    /// Send a transport action to the backend editor session. `catch` so it
    /// degrades to a no-op outside Tauri.
    #[wasm_bindgen(js_namespace = window, js_name = "__screenEditorTransport", catch)]
    fn screen_editor_transport_js(action: JsValue) -> Result<JsValue, JsValue>;
}

/// Send a transport action. The resulting status arrives asynchronously as
/// an `editor-status` event (see [`install_editor_status_listener`]).
pub fn editor_transport(action: &TransportAction) {
    if let Ok(js) = serde_wasm_bindgen::to_value(action) {
        let _ = screen_editor_transport_js(js);
    }
}

/// Install an `editor-status` `CustomEvent` listener pushing the parsed
/// [`EditorStatus`] into `status`. App-lifetime; leaked via `forget`.
pub fn install_editor_status_listener(status: RwSignal<EditorStatus>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if let Ok(custom) = event.dyn_into::<CustomEvent>()
            && let Ok(parsed) = serde_wasm_bindgen::from_value::<EditorStatus>(custom.detail())
        {
            status.set(parsed);
        }
    }) as Box<dyn FnMut(_)>);
    let _ =
        window.add_event_listener_with_callback("editor-status", closure.as_ref().unchecked_ref());
    closure.forget();
}

// ── ED.22: export progress + cancel ─────────────────────────────────────

/// Progress payload from the backend `editor-export-progress` event.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct ExportProgress {
    /// Frames composed + encoded so far.
    pub done: u64,
    /// Total frames in the export.
    pub total: u64,
}

/// UI-facing export state, driven by the export event bridge.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ExportUiState {
    /// No export in flight.
    #[default]
    Idle,
    /// Export running: `done` of `total` frames.
    Running {
        /// Frames done.
        done: u64,
        /// Total frames.
        total: u64,
    },
    /// Export finished — output at `path`.
    Done {
        /// Output file path.
        path: String,
    },
    /// Export failed (or was cancelled).
    Error {
        /// Failure message.
        message: String,
    },
}

/// Progress as a whole percent `0..=100` (0 when `total` is 0).
#[must_use]
pub fn export_percent(done: u64, total: u64) -> u32 {
    if total == 0 {
        return 0;
    }
    u32::try_from(done.saturating_mul(100) / total)
        .unwrap_or(100)
        .min(100)
}

#[wasm_bindgen]
extern "C" {
    /// Start an edited export. Resolves to the output path (re-dispatched as
    /// `editor-export-done`); rejects as `editor-export-error`.
    #[wasm_bindgen(js_namespace = window, js_name = "__screenEditorExport", catch)]
    fn screen_editor_export_js(project: JsValue, format: JsValue) -> Result<JsValue, JsValue>;

    /// Request cancellation of the in-flight export.
    #[wasm_bindgen(js_namespace = window, js_name = "__screenEditorExportCancel", catch)]
    fn screen_editor_export_cancel_js() -> Result<JsValue, JsValue>;
}

/// Start exporting `project` to `format` (e.g. `"mp4"`). Progress + result
/// arrive as events (see [`install_editor_export_listeners`]).
pub fn editor_export(project: &EditProject, format: &str) {
    if let Ok(js) = serde_wasm_bindgen::to_value(project) {
        let _ = screen_editor_export_js(js, JsValue::from_str(format));
    }
}

/// Request cancellation of the in-flight export.
pub fn editor_export_cancel() {
    let _ = screen_editor_export_cancel_js();
}

fn on_custom_event(name: &str, mut handler: impl FnMut(CustomEvent) + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if let Ok(custom) = event.dyn_into::<CustomEvent>() {
            handler(custom);
        }
    }) as Box<dyn FnMut(_)>);
    let _ = window.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref());
    closure.forget();
}

/// Install the export event listeners (progress / done / error), each
/// pushing into `state`. App-lifetime; closures leaked via `forget`.
pub fn install_editor_export_listeners(state: RwSignal<ExportUiState>) {
    on_custom_event("editor-export-progress", move |custom| {
        if let Ok(p) = serde_wasm_bindgen::from_value::<ExportProgress>(custom.detail()) {
            state.set(ExportUiState::Running {
                done: p.done,
                total: p.total,
            });
        }
    });
    on_custom_event("editor-export-done", move |custom| {
        let path = custom.detail().as_string().unwrap_or_default();
        state.set(ExportUiState::Done { path });
    });
    on_custom_event("editor-export-error", move |custom| {
        let message = custom
            .detail()
            .as_string()
            .unwrap_or_else(|| "export failed".to_owned());
        state.set(ExportUiState::Error { message });
    });
}

#[wasm_bindgen]
extern "C" {
    /// Save the current project to its `.screenproj` (ED.23). The written
    /// path is re-dispatched as an `editor-saved` event.
    #[wasm_bindgen(js_namespace = window, js_name = "__screenEditorSaveProject", catch)]
    fn screen_editor_save_project_js(project: JsValue) -> Result<JsValue, JsValue>;
}

/// Save `project` to its `.screenproj`. The written path arrives as an
/// `editor-saved` event (see [`install_editor_saved_listener`]).
pub fn editor_save_project(project: &EditProject) {
    if let Ok(js) = serde_wasm_bindgen::to_value(project) {
        let _ = screen_editor_save_project_js(js);
    }
}

/// Install an `editor-saved` listener pushing the written path into `saved`.
pub fn install_editor_saved_listener(saved: RwSignal<Option<String>>) {
    on_custom_event("editor-saved", move |custom| {
        saved.set(custom.detail().as_string());
    });
}

#[cfg(test)]
mod export_tests {
    use super::export_percent;

    #[test]
    fn percent_is_clamped_and_zero_safe() {
        assert_eq!(export_percent(0, 0), 0);
        assert_eq!(export_percent(0, 200), 0);
        assert_eq!(export_percent(100, 200), 50);
        assert_eq!(export_percent(200, 200), 100);
        assert_eq!(export_percent(999, 200), 100); // clamped
    }
}
