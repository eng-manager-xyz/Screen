//! JS bridge for the webcam-bubble toggle command (M-BUBBLE.0 / AUT-273).
//!
//! One-function module mirroring [`crate::camera_ipc`]'s shape — wraps
//! a `__screen*` helper declared inline in `index.html`, which in turn
//! calls `window.__TAURI__.core.invoke("toggle_webcam_bubble")`.
//!
//! Async + ignores the return value — toggling the bubble cannot fail
//! in any way the UI needs to react to. If the window can't be found
//! the Rust-side command logs via `tracing::warn!` (see
//! [`crate::commands::toggle_webcam_bubble`](../../app/src/commands.rs)
//! in the screen-app crate) and the click is a no-op.

use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// `__screenToggleBubble()` in `index.html` — returns `Promise<void>`.
    #[wasm_bindgen(js_name = __screenToggleBubble, catch)]
    pub async fn toggle_webcam_bubble_js() -> Result<JsValue, JsValue>;
}

/// Fire-and-forget toggle. Spawns the IPC call as a wasm-bindgen
/// future so the click handler returns immediately; failures are
/// logged to the JS console via `web_sys::console::warn_1` rather
/// than surfaced to the user (the Rust side warns via `tracing`).
pub fn toggle_webcam_bubble() {
    wasm_bindgen_futures::spawn_local(async {
        if let Err(err) = toggle_webcam_bubble_js().await {
            web_sys::console::warn_2(
                &JsValue::from_str("[bubble_ipc] toggle_webcam_bubble failed:"),
                &err,
            );
        }
    });
}
