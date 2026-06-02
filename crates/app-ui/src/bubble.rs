//! `<BubbleRoot />` — the Leptos tree mounted into the `webcam-bubble`
//! Tauri window (M-BUBBLE.0 / AUT-273 + the M-QUAL.4 overlay redesign).
//!
//! There is **no card**. The circle itself is the widget: a
//! `.bubble-stage` (the circle's bounding square) holds the clipped
//! circular camera feed plus three overlays painted on top of it —
//!   - `PREVIEW` pill near the top of the circle
//!   - pause / settings icon cluster at the upper-right edge
//!   - three recording-control buttons in a dark pill near the bottom
//!
//! …plus a floating device caption (`LuCamera` + `FaceTime HD ·
//! bottom-left`) below the circle on the transparent page background.
//!
//! The overlays are **siblings** of `.bubble-canvas-wrap`, not children
//! — the wrap is `overflow: hidden` to clip the feed to a circle, so
//! anything inside it would be clipped too. Keeping the overlays as
//! siblings (positioned against the stage) lets them sit on top of the
//! circle without being clipped. The circle mask here is the preview's
//! own; wisp owns the recorded frame's mask (CLAUDE.md "Mask is
//! wisp's, not CSS").
//!
//! The whole thing is draggable via `data-tauri-drag-region`; the
//! interactive clusters (icon cluster, controls) opt OUT of drag so a
//! click on a button doesn't initiate a window drag.

use leptos::prelude::*;
use leptos_icons::Icon;

use crate::camera_preview::CameraPreview;

/// Leptos component mounted at `?mount=bubble`. Renders the
/// borderless circular webcam surface with overlaid controls.
#[component]
pub fn BubbleRoot() -> impl IntoView {
    // AUT-276 — local size cycle (Small → Medium → Large → Small). Medium is
    // the `tauri.conf.json` default; the resize button drives the backend
    // `set_bubble_size`.
    let size_idx = RwSignal::new(1u8);
    view! {
        <div
            class="bubble-root bubble-root--design"
            data-tauri-drag-region="true"
            aria-label="Webcam bubble overlay"
        >
            <div class="bubble-stage" data-tauri-drag-region="true">
                <div
                    class="bubble-canvas-wrap"
                    data-tauri-drag-region="true"
                    aria-hidden="true"
                    title="Double-click to snap to the nearest corner"
                    on:dblclick=move |_| crate::bubble_ipc::snap_bubble_to_corner()
                >
                    <CameraPreview />
                </div>

                <span class="bubble-preview-chip" aria-hidden="true">
                    <span class="bubble-preview-chip-dot"></span>
                    "PREVIEW"
                </span>

                <span class="bubble-header-actions" data-tauri-drag-region="false">
                    <button
                        type="button"
                        class="bubble-icon-btn"
                        aria-label="Pause preview"
                        title="Pause preview"
                    >
                        <Icon icon=icondata::LuPause />
                    </button>
                    <button
                        type="button"
                        class="bubble-icon-btn"
                        aria-label="Bubble settings"
                        title="Bubble settings"
                    >
                        <Icon icon=icondata::LuSettings />
                    </button>
                    <button
                        type="button"
                        class="bubble-icon-btn"
                        aria-label="Resize bubble"
                        title="Resize bubble (small / medium / large)"
                        on:click=move |_| {
                            let next = next_bubble_size_idx(size_idx.get_untracked());
                            size_idx.set(next);
                            crate::bubble_ipc::set_bubble_size(bubble_size_slug(next));
                        }
                    >
                        <Icon icon=icondata::LuMaximize2 />
                    </button>
                </span>

                <div
                    class="bubble-controls"
                    data-tauri-drag-region="false"
                    role="toolbar"
                    aria-label="Recording controls"
                >
                    <button
                        type="button"
                        class="bubble-control bubble-control--active"
                        aria-label="Record"
                    >
                        <Icon icon=icondata::LuCircle />
                    </button>
                    <button type="button" class="bubble-control" aria-label="Pause recording">
                        <Icon icon=icondata::LuPause />
                    </button>
                    <button type="button" class="bubble-control" aria-label="Stop recording">
                        <Icon icon=icondata::LuSquare />
                    </button>
                </div>
            </div>

            <div class="bubble-caption" data-tauri-drag-region="false">
                <span class="bubble-caption-icon" aria-hidden="true">
                    <Icon icon=icondata::LuCamera />
                </span>
                <span class="bubble-caption-label">"FaceTime HD"</span>
                <span class="bubble-caption-sep" aria-hidden="true">"·"</span>
                <span class="bubble-caption-corner">"bottom-left"</span>
            </div>
        </div>
    }
}

/// Cycle the bubble size index: Small(0) → Medium(1) → Large(2) → Small (AUT-276).
fn next_bubble_size_idx(idx: u8) -> u8 {
    (idx + 1) % 3
}

/// The [`crate::bubble_ipc::set_bubble_size`] slug for a size index (AUT-276).
/// Out-of-range indices fall back to `"medium"` (the default).
fn bubble_size_slug(idx: u8) -> &'static str {
    match idx {
        0 => "small",
        2 => "large",
        _ => "medium",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bubble_size_cycle_wraps_and_maps_to_slugs() {
        assert_eq!(next_bubble_size_idx(0), 1);
        assert_eq!(next_bubble_size_idx(1), 2);
        assert_eq!(next_bubble_size_idx(2), 0, "wraps back to small");
        assert_eq!(bubble_size_slug(0), "small");
        assert_eq!(bubble_size_slug(1), "medium");
        assert_eq!(bubble_size_slug(2), "large");
        // Out-of-range → default medium.
        assert_eq!(bubble_size_slug(9), "medium");
    }
}
