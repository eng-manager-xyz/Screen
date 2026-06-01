//! Live screen-capture preview canvas (AUT-269).
//!
//! Replaces the recorder's mock display-preview with the **real** screen being
//! captured. Mirrors [`crate::camera_preview`] / [`crate::editor_preview_canvas`]:
//! an [`Effect`] starts/stops the downscaled preview capture as the display
//! section opens/closes (or the selected source changes), and a `setInterval`
//! poll paints the latest frame (`latest_screen_frame_bgra`) into the canvas.
//!
//! The backend captures downscaled to 1280×720 (`screen_capture::PREVIEW_WIDTH`
//! / `PREVIEW_HEIGHT`) and excludes the recorder's own windows, so the poll is
//! cheap and the preview doesn't capture itself (the screen-of-its-own-screen
//! feedback loop).

use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Stable DOM id for the screen-preview `<canvas>` (found by the poll).
pub const SCREEN_PREVIEW_CANVAS_ID: &str = "screen-preview-canvas";

/// Preview frame width — must match the backend `screen_capture::PREVIEW_WIDTH`
/// so the polled BGRA bytes fit the canvas pixel-for-pixel (`putImageData`
/// does not scale). The two crates can't share a const (native vs wasm).
pub const SCREEN_PREVIEW_WIDTH: u32 = 1280;
/// Preview frame height — matches the backend `screen_capture::PREVIEW_HEIGHT`.
pub const SCREEN_PREVIEW_HEIGHT: u32 = 720;
/// ~15 fps poll interval (ms) — matches the camera/editor previews.
pub const SCREEN_PREVIEW_POLL_MS: i32 = 66;

/// Install the live screen-preview hooks (AUT-269): start/stop the downscaled
/// preview capture as `active` + `source` change, plus the repaint poll. Call
/// **once** from the recorder body (not a reactive block, or each rebuild
/// leaks another `setInterval`). `active` = "the display section is showing";
/// `source` = the selected display/window id (`None` = primary display).
#[cfg(target_arch = "wasm32")]
pub fn install_screen_preview(active: Memo<bool>, source: RwSignal<Option<String>>) {
    // Start / stop the downscaled preview capture as visibility + source
    // change. `start_screen_capture` drops any in-flight stream first, so a
    // source change while active cleanly re-targets; a permission denial just
    // leaves the canvas blank (best-effort).
    Effect::new(move |_| {
        let on = active.get();
        let src = source.get();
        spawn_local(async move {
            if on {
                let _ = crate::screen_ipc::start_screen_capture(src).await;
            } else {
                crate::screen_ipc::stop_screen_capture().await;
            }
        });
    });
    on_cleanup(|| {
        spawn_local(async {
            crate::screen_ipc::stop_screen_capture().await;
        });
    });

    let closure = Closure::wrap(Box::new(move || {
        spawn_local(async move {
            paint_one_screen_frame().await;
        });
    }) as Box<dyn FnMut()>);
    if let Some(window) = web_sys::window() {
        let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            SCREEN_PREVIEW_POLL_MS,
        );
    }
    closure.forget();
}

/// Native stub — the screen preview runs only in the wasm webview.
#[cfg(not(target_arch = "wasm32"))]
pub fn install_screen_preview(_active: Memo<bool>, _source: RwSignal<Option<String>>) {}

/// One paint tick: find the canvas (skip the IPC entirely when the preview
/// isn't shown — a cheap idle poll), request the latest downscaled frame, swap
/// BGRA→RGBA, and `putImageData` it. Best-effort — every failure path no-ops.
#[cfg(target_arch = "wasm32")]
async fn paint_one_screen_frame() {
    use js_sys::{Reflect, Uint8ClampedArray};
    use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

    let bytes_len = (SCREEN_PREVIEW_WIDTH as usize) * (SCREEN_PREVIEW_HEIGHT as usize) * 4;

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    // Canvas-first: when the display section is closed the canvas is absent, so
    // we skip the IPC call entirely and keep the idle poll cheap.
    let Some(canvas_el) = document.get_element_by_id(SCREEN_PREVIEW_CANVAS_ID) else {
        return;
    };
    let Ok(canvas) = canvas_el.dyn_into::<HtmlCanvasElement>() else {
        return;
    };

    let Ok(invoke_fn) = Reflect::get(&window, &JsValue::from_str("__screenLatestScreenFrameBgra"))
    else {
        return;
    };
    if !invoke_fn.is_function() {
        return;
    }
    let invoke: js_sys::Function = invoke_fn.unchecked_into();
    let Ok(promise) = invoke.call0(&JsValue::NULL) else {
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
    // Empty / no-frame (no open session, or before the first frame): skip.
    let len = bytes.length() as usize;
    if len < bytes_len {
        return;
    }

    let mut rgba = vec![0u8; bytes_len];
    bytes.copy_to(&mut rgba[..bytes_len]);
    for px in rgba.chunks_exact_mut(4) {
        // BGRA → RGBA (the captured frame is BGRA, like the camera frame).
        px.swap(0, 2);
    }

    let Ok(Some(ctx)) = canvas.get_context("2d") else {
        return;
    };
    let Ok(ctx) = ctx.dyn_into::<CanvasRenderingContext2d>() else {
        return;
    };
    let Ok(image_data) = ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&rgba[..]),
        SCREEN_PREVIEW_WIDTH,
        SCREEN_PREVIEW_HEIGHT,
    ) else {
        return;
    };
    let _ = ctx.put_image_data(&image_data, 0.0, 0.0);
}
