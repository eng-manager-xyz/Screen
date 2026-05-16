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

    /// `__screenSetBubbleClickthrough(enabled)` in `index.html` —
    /// returns `Promise<void>`. M-BUBBLE.1 v0 / AUT-274.
    #[wasm_bindgen(js_name = __screenSetBubbleClickthrough, catch)]
    pub async fn set_bubble_clickthrough_js(enabled: bool) -> Result<JsValue, JsValue>;
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

/// Fire-and-forget click-through toggle. `enabled = true` makes the
/// bubble pass mouse events through; `false` restores normal
/// interaction (drag works again). Failures log to the JS console;
/// the Rust side also logs via `tracing::warn!`. M-BUBBLE.1 v0 /
/// AUT-274.
pub fn set_bubble_clickthrough(enabled: bool) {
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(err) = set_bubble_clickthrough_js(enabled).await {
            web_sys::console::warn_2(
                &JsValue::from_str("[bubble_ipc] set_bubble_clickthrough failed:"),
                &err,
            );
        }
    });
}
