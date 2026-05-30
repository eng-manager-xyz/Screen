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
