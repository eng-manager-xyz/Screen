//! Live editor preview canvas wiring (AUT-510 / ED.6b).
//!
//! Replaces the old `"Preview renders here."` placeholder. Paints the composed
//! frame (zoom / crop / background / cursor) at the playhead by polling the
//! `editor_preview_frame` Tauri command — the **same** compose the export uses
//! — and `putImageData`-ing it into the editor canvas, mirroring
//! [`crate::camera_preview`]'s loop. Two hooks installed once per editor mount:
//!
//! - an [`Effect`] on the project signal that calls
//!   [`editor_ipc::editor_preview_open`](crate::editor_ipc::editor_preview_open)
//!   so the backend (re)builds the compose pipeline + sees each edit;
//! - a `setInterval` poll (~15 fps) that requests the composed frame at the
//!   current playhead and paints it.
//!
//! Unlike the fixed-size camera preview, the editor frame is the **clip's**
//! dimensions, so the canvas is sized from `project.source` and the poll
//! computes the expected byte length per clip (`putImageData` does not scale).

use edit::EditProject;
use leptos::prelude::*;

use crate::editor_ipc::EditorStatus;

#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Stable DOM id for the editor preview `<canvas>`. The poll finds it by id
/// (the canvas element itself is rendered by the editor surface).
pub const EDITOR_PREVIEW_CANVAS_ID: &str = "editor-preview-canvas";

/// Repaint cadence (ms) — ~15 fps, matching the camera preview poll.
pub const EDITOR_PREVIEW_POLL_MS: i32 = 66;

/// Install the live-preview hooks: re-open on project change + the repaint
/// poll. Call **once** from the editor surface body (not inside a reactive
/// block, or each rebuild would leak another `setInterval`).
#[cfg(target_arch = "wasm32")]
pub fn install_editor_preview(
    project: Option<RwSignal<Option<EditProject>>>,
    status: RwSignal<EditorStatus>,
) {
    // (Re)build the compose pipeline + push edits whenever the project changes
    // (initial load + every edit op). The backend re-opens the stream only
    // when the source clip changes; otherwise it just swaps the project.
    Effect::new(move |_| {
        if let Some(sig) = project
            && let Some(p) = sig.get()
        {
            crate::editor_ipc::editor_preview_open(&p);
        }
    });

    let closure = Closure::wrap(Box::new(move || {
        spawn_local(async move {
            paint_one_editor_frame(project, status).await;
        });
    }) as Box<dyn FnMut()>);
    if let Some(window) = web_sys::window() {
        let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            EDITOR_PREVIEW_POLL_MS,
        );
    }
    closure.forget();
}

/// Native stub — the editor preview runs only in the wasm webview.
#[cfg(not(target_arch = "wasm32"))]
pub fn install_editor_preview(
    _project: Option<RwSignal<Option<EditProject>>>,
    _status: RwSignal<EditorStatus>,
) {
}

/// One paint tick: request the composed frame at the current playhead, swap
/// BGRA→RGBA, and `putImageData` it into the editor canvas at the clip's
/// dimensions. Best-effort — every failure path silently no-ops so a missing
/// canvas / closed session can't crash the UI (mirrors the camera preview).
#[cfg(target_arch = "wasm32")]
#[allow(
    clippy::cast_precision_loss,
    reason = "the playhead frame index is well under 2^53, so u64→f64 is exact"
)]
async fn paint_one_editor_frame(
    project: Option<RwSignal<Option<EditProject>>>,
    status: RwSignal<EditorStatus>,
) {
    use js_sys::{Reflect, Uint8ClampedArray};
    use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

    // Clip dimensions (the composed frame size) come from the loaded project.
    let Some((width, height)) = project
        .and_then(|sig| sig.get_untracked())
        .map(|p| (p.source.width, p.source.height))
    else {
        return;
    };
    let bytes_len = (width as usize) * (height as usize) * 4;
    if bytes_len == 0 {
        return;
    }
    let frame = status.get_untracked().current_frame;

    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(invoke_fn) = Reflect::get(&window, &JsValue::from_str("__screenEditorPreviewFrame"))
    else {
        return;
    };
    if !invoke_fn.is_function() {
        return;
    }
    let invoke: js_sys::Function = invoke_fn.unchecked_into();
    let Ok(promise) = invoke.call1(&JsValue::NULL, &JsValue::from_f64(frame as f64)) else {
        return;
    };
    let promise: js_sys::Promise = match promise.dyn_into() {
        Ok(p) => p,
        Err(_) => return,
    };
    let Ok(buf) = wasm_bindgen_futures::JsFuture::from(promise).await else {
        return;
    };

    let bytes = if let Ok(array_buffer) = buf.clone().dyn_into::<js_sys::ArrayBuffer>() {
        Uint8ClampedArray::new(&array_buffer)
    } else if let Ok(typed_array) = buf.dyn_into::<Uint8ClampedArray>() {
        typed_array
    } else {
        return;
    };
    // Empty / no-frame (no open session, or past the end): skip.
    if (bytes.length() as usize) < bytes_len {
        return;
    }

    let mut rgba = vec![0u8; bytes_len];
    bytes.copy_to(&mut rgba[..bytes_len]);
    for px in rgba.chunks_exact_mut(4) {
        // BGRA → RGBA (the composed frame is BGRA, like the camera frame).
        px.swap(0, 2);
    }

    let Some(document) = window.document() else {
        return;
    };
    let Some(canvas_el) = document.get_element_by_id(EDITOR_PREVIEW_CANVAS_ID) else {
        return;
    };
    let Ok(canvas) = canvas_el.dyn_into::<HtmlCanvasElement>() else {
        return;
    };
    let Ok(Some(ctx)) = canvas.get_context("2d") else {
        return;
    };
    let Ok(ctx) = ctx.dyn_into::<CanvasRenderingContext2d>() else {
        return;
    };
    let Ok(image_data) = ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&rgba[..]),
        width,
        height,
    ) else {
        return;
    };
    let _ = ctx.put_image_data(&image_data, 0.0, 0.0);
}
