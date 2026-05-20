//! `<BubbleRoot />` — the Leptos tree mounted into the `webcam-bubble`
//! Tauri window (M-BUBBLE.0 / AUT-273 + design pass).
//!
//! Renders the design's borderless-circle webcam preview with:
//!   - PREVIEW chip (top-left of the circle)
//!   - drag-handle dots + gear icon (top-right, free-floating)
//!   - three control pills (mute / pause / stop) at the bottom of the
//!     circle
//!   - device label + corner-hint text below the circle
//!
//! The CIRCLE itself is a `<canvas>` that M-CAM.2 / M-PIX.8 paints
//! camera frames into (the `crate::camera_preview::CameraPreview`
//! component handles the per-tick paint loop). The mask is wisp's
//! responsibility — do not clip-path the canvas or you double-crop the
//! circle. See CLAUDE.md "Mask is wisp's, not CSS".
//!
//! The whole bubble is draggable via `data-tauri-drag-region` on the
//! root. The three control pills + the gear + the drag-handle glyph
//! must opt OUT of drag (`data-tauri-drag-region="false"` on their
//! ancestors) so a click on them doesn't initiate a window drag.

use leptos::prelude::*;

use crate::camera_preview::CameraPreview;

/// Leptos component mounted at `?mount=bubble`. Renders the design's
/// circular webcam surface + floating controls.
#[component]
pub fn BubbleRoot() -> impl IntoView {
    view! {
        <div
            class="bubble-root bubble-root--design"
            data-tauri-drag-region="true"
            aria-label="Webcam bubble overlay"
        >
            <div class="bubble-overlay" data-tauri-drag-region="false">
                <span class="bubble-preview-chip" aria-hidden="true">
                    <span class="bubble-preview-chip-dot"></span>
                    "PREVIEW"
                </span>
                <span class="bubble-handle-cluster">
                    <button
                        type="button"
                        class="bubble-handle bubble-handle--drag"
                        aria-label="Drag the bubble"
                        title="Drag the bubble"
                    >
                        <span class="bubble-handle-glyph" aria-hidden="true">
                            "⋮⋮"
                        </span>
                    </button>
                    <button
                        type="button"
                        class="bubble-handle bubble-handle--gear"
                        aria-label="Bubble settings"
                        title="Bubble settings"
                    >
                        <span class="bubble-handle-glyph" aria-hidden="true">
                            "⚙"
                        </span>
                    </button>
                </span>
            </div>

            <div class="bubble-canvas-wrap" aria-hidden="true">
                <CameraPreview />
            </div>

            <div class="bubble-controls" data-tauri-drag-region="false" role="toolbar" aria-label="Webcam controls">
                <button type="button" class="bubble-control bubble-control--mute" aria-label="Mute camera">
                    <span class="bubble-control-glyph" aria-hidden="true">"●"</span>
                </button>
                <button type="button" class="bubble-control bubble-control--pause" aria-label="Pause preview">
                    <span class="bubble-control-glyph" aria-hidden="true">"❚❚"</span>
                </button>
                <button type="button" class="bubble-control bubble-control--stop" aria-label="Stop preview">
                    <span class="bubble-control-glyph" aria-hidden="true">"■"</span>
                </button>
            </div>

            <div class="bubble-caption" data-tauri-drag-region="false">
                <span class="bubble-caption-icon" aria-hidden="true">"📷"</span>
                <span class="bubble-caption-label">"FaceTime HD"</span>
                <span class="bubble-caption-sep" aria-hidden="true">"·"</span>
                <span class="bubble-caption-corner">"bottom-left"</span>
            </div>
        </div>
    }
}
