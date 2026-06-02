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

    /// `__screenSetBubbleVisibility(visible)` in `index.html` —
    /// returns `Promise<void>`. ISS-05 fix; explicit setter so the
    /// recorder's `camera_enabled` signal can drive the bubble's
    /// visibility without the toggle-drift the always-flip path had.
    #[wasm_bindgen(js_name = __screenSetBubbleVisibility, catch)]
    pub async fn set_webcam_bubble_visibility_js(visible: bool) -> Result<JsValue, JsValue>;

    /// `__screenSetBubbleClickthrough(enabled)` in `index.html` —
    /// returns `Promise<void>`. M-BUBBLE.1 v0 / AUT-274.
    #[wasm_bindgen(js_name = __screenSetBubbleClickthrough, catch)]
    pub async fn set_bubble_clickthrough_js(enabled: bool) -> Result<JsValue, JsValue>;

    /// `__screenSetBubbleSize(size)` in `index.html` — returns
    /// `Promise<void>`. `size` is `"small"` / `"medium"` / `"large"`. AUT-276.
    #[wasm_bindgen(js_name = __screenSetBubbleSize, catch)]
    pub async fn set_bubble_size_js(size: String) -> Result<JsValue, JsValue>;

    /// `__screenSnapBubble()` in `index.html` — returns `Promise<void>`.
    /// Snaps the bubble to the nearest monitor corner. AUT-276.
    #[wasm_bindgen(js_name = __screenSnapBubble, catch)]
    pub async fn snap_bubble_js() -> Result<JsValue, JsValue>;
}

/// Fire-and-forget resize (AUT-276). `size` = `"small"` / `"medium"` /
/// `"large"`. Failures log to the JS console; the Rust side warns via
/// `tracing`.
pub fn set_bubble_size(size: &str) {
    let size = size.to_owned();
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(err) = set_bubble_size_js(size).await {
            web_sys::console::warn_2(
                &JsValue::from_str("[bubble_ipc] set_bubble_size failed:"),
                &err,
            );
        }
    });
}

/// Fire-and-forget snap-to-nearest-corner (AUT-276) — the double-click UX.
pub fn snap_bubble_to_corner() {
    wasm_bindgen_futures::spawn_local(async {
        if let Err(err) = snap_bubble_js().await {
            web_sys::console::warn_2(
                &JsValue::from_str("[bubble_ipc] snap_bubble_to_corner failed:"),
                &err,
            );
        }
    });
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

/// Fire-and-forget setter. Use when the UI owns the source of truth
/// for the bubble's desired state (e.g. the recorder's
/// `camera_enabled` signal). Idempotent on the Rust side, so safe to
/// call on every toggle click + on page mount for initial alignment.
/// ISS-05.
pub fn set_webcam_bubble_visibility(visible: bool) {
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(err) = set_webcam_bubble_visibility_js(visible).await {
            web_sys::console::warn_2(
                &JsValue::from_str("[bubble_ipc] set_webcam_bubble_visibility failed:"),
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
