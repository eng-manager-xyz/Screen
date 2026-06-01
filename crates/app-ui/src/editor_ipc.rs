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

    /// Open / update the live editor preview (AUT-510) for `project` — builds
    /// the compose pipeline (and re-applies edits) so `editor_preview_frame`
    /// composes the current edit. Fire-and-forget.
    #[wasm_bindgen(js_namespace = window, js_name = "__screenEditorPreviewOpen", catch)]
    fn screen_editor_preview_open_js(project: JsValue) -> Result<JsValue, JsValue>;
}

/// Open / update the live editor preview for `project` (AUT-510). Call on the
/// initial load and whenever an edit mutates the project, so the next composed
/// frame reflects it. The composed frames themselves come from the canvas poll
/// (`__screenEditorPreviewFrame`).
pub fn editor_preview_open(project: &EditProject) {
    if let Ok(js) = serde_wasm_bindgen::to_value(project) {
        let _ = screen_editor_preview_open_js(js);
    }
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

// ── ED.24: recordings library ───────────────────────────────────────────

/// Library mirror of the backend `RecordingEntry` (deserialize side).
#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize)]
pub struct RecordingEntry {
    /// Absolute path to the `.mp4`.
    pub path: String,
    /// Display name (file stem).
    pub name: String,
    /// Whether a saved `.screenproj` sits beside it.
    pub has_project: bool,
}

#[wasm_bindgen]
extern "C" {
    /// List recordings; results arrive as a `recordings-listed` event.
    #[wasm_bindgen(js_namespace = window, js_name = "__screenListRecordings", catch)]
    fn screen_list_recordings_js() -> Result<JsValue, JsValue>;
}

/// Ask the shell to list recordings (results via `recordings-listed`).
pub fn list_recordings() {
    let _ = screen_list_recordings_js();
}

/// Install a `recordings-listed` listener pushing the entries into `entries`.
pub fn install_recordings_listener(entries: RwSignal<Vec<RecordingEntry>>) {
    on_custom_event("recordings-listed", move |custom| {
        if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<RecordingEntry>>(custom.detail()) {
            entries.set(list);
        }
    });
}

// ── Drop-to-edit ─────────────────────────────────────────────────────────

/// Whether `path` has a recognized video extension. The editor drop
/// listener ignores non-video drops so dropping an unrelated file is a
/// no-op rather than a failed `gst-discoverer` probe.
#[must_use]
pub(crate) fn looks_like_video(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [".mp4", ".mov", ".m4v", ".mkv", ".webm", ".avi"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

/// Install a `file-dropped` listener that opens a dropped **video** in the
/// editor: it calls [`screen_open_in_editor`], whose `editor-project` reply
/// loads the project and (via the app-root effect) switches to the Editor
/// tab. Dropping a video anywhere in the app surface opens it for editing.
/// App-lifetime; the closure is leaked via `forget`.
pub fn install_file_drop_to_editor_listener() {
    on_custom_event("file-dropped", move |custom| {
        if let Some(path) = custom.detail().as_string()
            && looks_like_video(&path)
        {
            let _ = screen_open_in_editor(&path);
        }
    });
}

/// Install `file-drag-enter` / `file-drag-leave` listeners that drive
/// `active` — the editor drop zone reads it to show a drag-over highlight.
/// App-lifetime; closures leaked via `forget`.
pub fn install_drag_active_listeners(active: RwSignal<bool>) {
    on_custom_event("file-drag-enter", move |_| active.set(true));
    on_custom_event("file-drag-leave", move |_| active.set(false));
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

#[cfg(test)]
mod drop_tests {
    use super::looks_like_video;

    #[test]
    fn recognizes_video_extensions_case_insensitively() {
        assert!(looks_like_video("/recordings/Screen-2026-05-31.mp4"));
        assert!(looks_like_video("/x/clip.MOV"));
        assert!(looks_like_video("/x/a.WebM"));
        assert!(looks_like_video("/x/b.mkv"));
        // Non-video / extension-less drops are ignored.
        assert!(!looks_like_video("/x/notes.txt"));
        assert!(!looks_like_video("/x/image.png"));
        assert!(!looks_like_video("/x/screenshot.mp4.txt"));
        assert!(!looks_like_video("/x/noext"));
    }
}
