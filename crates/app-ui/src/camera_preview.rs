//! `<CameraPreview />` — Leptos canvas painted by raw BGRA frames
//! pulled from the `CameraFrameSlot` (M-PIX.8 of M-RECORD-EXPORT-REAL-PIXELS).
//!
//! Owns the `RecorderPreviewState` enum that drives the four UX
//! states (`Initialising` / `AwaitingPermission` / `PermissionDenied`
//! / `Live`), and a `<canvas>` element painted at 15 fps by an
//! `setInterval` worker that polls the
//! `latest_camera_frame_bgra` Tauri command (raw `ArrayBuffer`
//! over `tauri::ipc::Response` — zero JSON overhead).
//!
//! ```admonish info title="BGRA → RGBA swap"
//! `CanvasRenderingContext2D::putImageData` expects RGBA in linear
//! byte order. The capture pipeline emits BGRA (the macOS /
//! GStreamer / SCK default for screen + camera). We swap the R/B
//! channels in-place in JS before constructing the `ImageData`.
//! ```
//!
//! ```admonish info title="Mask is CSS now"
//! M-PIX.8 paints the raw rectangular BGRA frame into the canvas
//! and uses `border-radius: 50%` to render it as a circle. The
//! wisp-baked mask (described in earlier comments) lives in the
//! ENCODER side: `RecordingScene` masks the cam sprite into a
//! circle in the .mp4 output. The UI preview uses CSS so we
//! don't need a separate wisp pipeline running in the webview
//! just for the preview.
//! ```

use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Four-state UX machine for the camera preview surface
/// (M-CAM.3 / AUT-257).
///
/// Transitions: `Initialising` is the initial render before
/// `start_preview` IPC resolves. `AwaitingPermission` is the
/// macOS-prompt-pending state. `PermissionDenied` is the post-prompt
/// reject. `Live` is the running pipeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecorderPreviewState {
    /// `start_preview` IPC has not yet resolved.
    #[default]
    Initialising,
    /// macOS permission prompt is up; the user hasn't clicked yet.
    AwaitingPermission,
    /// User has denied camera access.
    PermissionDenied,
    /// Pipeline is running + painting frames.
    Live,
}

impl RecorderPreviewState {
    /// Human-readable copy for the loading / error states. `Live`
    /// returns an empty string because the canvas paints over it.
    #[must_use]
    pub fn copy(self) -> &'static str {
        match self {
            Self::Initialising => "Starting camera…",
            Self::AwaitingPermission => "Waiting for camera permission…",
            Self::PermissionDenied => {
                "Camera access denied. Grant access in System Settings → Privacy & Security, then re-open this surface."
            }
            Self::Live => "",
        }
    }

    /// Kebab-case slug for CSS class hooks + data-attribute styling.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            Self::Initialising => "initialising",
            Self::AwaitingPermission => "awaiting-permission",
            Self::PermissionDenied => "permission-denied",
            Self::Live => "live",
        }
    }
}

/// Stable DOM id for the `<canvas>` element. M-CAM.3's Rust-side
/// pipeline finds the canvas by `document.getElementById(...)` and
/// `putImageData`s frames into its 2D context.
pub const CANVAS_DOM_ID: &str = "camera-preview-canvas";

/// Frame width the camera preview canvas paints at — must match the
/// capture size (`screen_app::preview::pipeline::PREVIEW_WIDTH`, 720
/// since M-QUAL.3) so the polled BGRA bytes fit the canvas
/// pixel-for-pixel (`putImageData` does not scale). The two crates
/// can't share a constant (native vs wasm), so keep them in lockstep.
pub const PREVIEW_CANVAS_WIDTH: u32 = 720;
/// Frame height (square: matches the preview pipeline).
pub const PREVIEW_CANVAS_HEIGHT: u32 = 720;
/// Bytes per frame (BGRA × width × height).
pub const PREVIEW_CANVAS_BYTES: usize =
    (PREVIEW_CANVAS_WIDTH as usize) * (PREVIEW_CANVAS_HEIGHT as usize) * 4;
/// 15 fps preview poll interval in milliseconds. Sufficient for a
/// "see your face" UX; IPC traffic ~31 MB/s at 720×720.
pub const PREVIEW_POLL_MS: i32 = 66;

/// `<CameraPreview />` — the recorder surface's webcam preview.
///
/// The component renders a 720×720 `<canvas>` plus an overlaid copy
/// element that displays the current `RecorderPreviewState`. M-PIX.8
/// installs a 15 fps poll that pulls the latest BGRA frame from
/// `CameraFrameSlot` via the `latest_camera_frame_bgra` Tauri
/// command + paints it into the canvas (with a BGRA→RGBA swap).
#[component]
pub fn CameraPreview() -> impl IntoView {
    let state = RwSignal::new(RecorderPreviewState::default());
    install_camera_frame_poll(state);
    view! {
        <section
            class="camera-preview-surface"
            data-state=move || state.get().slug()
        >
            <canvas
                id=CANVAS_DOM_ID
                class="camera-preview"
                width=PREVIEW_CANVAS_WIDTH
                height=PREVIEW_CANVAS_HEIGHT
                aria-label="Live webcam preview (circular)"
            />
            <Show
                when=move || !matches!(state.get(), RecorderPreviewState::Live)
                fallback=|| view! { <></> }
            >
                <div class="camera-preview-overlay">
                    {move || state.get().copy()}
                </div>
            </Show>
        </section>
    }
}

/// Set up the 15 fps poll → fetch BGRA → swap R/B → `putImageData`
/// loop. Flips `state` to `Live` on first painted frame.
///
/// `setInterval` lives for the lifetime of the page (the closure is
/// `forget()`-ed so the JS runtime keeps it). Matches the
/// `install_*_listener` pattern used by the file-drop +
/// recording-status hooks.
#[cfg(target_arch = "wasm32")]
fn install_camera_frame_poll(state: RwSignal<RecorderPreviewState>) {
    let closure = Closure::wrap(Box::new(move || {
        spawn_local(async move {
            paint_one_frame(state).await;
        });
    }) as Box<dyn FnMut()>);
    if let Some(window) = web_sys::window() {
        let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            PREVIEW_POLL_MS,
        );
    }
    closure.forget();
}

#[cfg(not(target_arch = "wasm32"))]
fn install_camera_frame_poll(_state: RwSignal<RecorderPreviewState>) {}

/// One paint tick: call the IPC, get an `ArrayBuffer`, copy into a
/// `Uint8ClampedArray`, swap R↔B per pixel, build `ImageData`, hand
/// to the canvas 2D context. Best-effort: every failure path silently
/// no-ops + a `console.trace` so the UI doesn't crash if the canvas
/// vanished between renders.
#[cfg(target_arch = "wasm32")]
async fn paint_one_frame(state: RwSignal<RecorderPreviewState>) {
    use js_sys::{Reflect, Uint8ClampedArray};
    use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(invoke_fn) = Reflect::get(&window, &JsValue::from_str("__screenLatestCameraFrameBgra"))
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
    let result = wasm_bindgen_futures::JsFuture::from(promise).await;
    let Ok(buf) = result else {
        return;
    };

    // Empty / no-frame: skip.
    let bytes = if let Ok(array_buffer) = buf.clone().dyn_into::<js_sys::ArrayBuffer>() {
        Uint8ClampedArray::new(&array_buffer)
    } else if let Ok(typed_array) = buf.dyn_into::<Uint8ClampedArray>() {
        typed_array
    } else {
        return;
    };
    let len = bytes.length() as usize;
    if len < PREVIEW_CANVAS_BYTES {
        return;
    }

    // Pull bytes into a Vec for in-place R↔B swap. Cheaper than
    // hopping in-and-out of JS for each byte.
    let mut rgba = vec![0u8; PREVIEW_CANVAS_BYTES];
    bytes.copy_to(&mut rgba[..PREVIEW_CANVAS_BYTES]);
    for px in rgba.chunks_exact_mut(4) {
        // BGRA → RGBA: swap byte 0 (B) with byte 2 (R).
        px.swap(0, 2);
    }

    // Find the canvas + 2D context. Defensive — the canvas DOM node
    // is mounted by the component but a route change could have
    // unmounted it.
    let document = window.document();
    let Some(document) = document else {
        return;
    };
    let Some(canvas_el) = document.get_element_by_id(CANVAS_DOM_ID) else {
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
        PREVIEW_CANVAS_WIDTH,
        PREVIEW_CANVAS_HEIGHT,
    ) else {
        return;
    };
    let _ = ctx.put_image_data(&image_data, 0.0, 0.0);

    // First successful paint flips the state machine to Live so
    // the overlay copy ("Starting camera…") clears.
    if !matches!(state.get_untracked(), RecorderPreviewState::Live) {
        state.set(RecorderPreviewState::Live);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(
    dead_code,
    clippy::unused_async,
    reason = "native stub for symmetry with the wasm32 async impl; cargo check on native target sees no caller and no .await."
)]
async fn paint_one_frame(_state: RwSignal<RecorderPreviewState>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_initialising() {
        assert_eq!(
            RecorderPreviewState::default(),
            RecorderPreviewState::Initialising
        );
    }

    #[test]
    fn each_state_has_unique_slug() {
        let states = [
            RecorderPreviewState::Initialising,
            RecorderPreviewState::AwaitingPermission,
            RecorderPreviewState::PermissionDenied,
            RecorderPreviewState::Live,
        ];
        let mut slugs: Vec<_> = states.iter().map(|s| s.slug()).collect();
        slugs.sort_unstable();
        slugs.dedup();
        assert_eq!(slugs.len(), states.len());
    }

    #[test]
    fn live_has_empty_copy() {
        assert!(RecorderPreviewState::Live.copy().is_empty());
    }

    #[test]
    fn non_live_states_have_non_empty_copy() {
        for s in [
            RecorderPreviewState::Initialising,
            RecorderPreviewState::AwaitingPermission,
            RecorderPreviewState::PermissionDenied,
        ] {
            assert!(!s.copy().is_empty(), "state {s:?} had empty copy");
        }
    }
}
