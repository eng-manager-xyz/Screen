//! `<BubbleRoot />` — the Leptos tree mounted into the `webcam-bubble`
//! Tauri window (M-BUBBLE.0 / AUT-273).
//!
//! Renders a borderless full-viewport root that the bubble window's
//! transparent body lays over. For M-BUBBLE.0 this is a static
//! placeholder so the borderless window is visually findable —
//! M-BUBBLE.2 (AUT-275) replaces the placeholder with a `<canvas>`
//! that paints frames from the same M-CAM.2 broadcast channel the
//! `AppShell` preview uses.
//!
//! The `data-tauri-drag-region` attribute on the root turns the
//! whole window into a drag handle — Tauri 2 handles OS-side window
//! drag without any JS. Click-through hit-testing for the four
//! transparent corners is M-BUBBLE.1's concern, not this ticket's.

use leptos::prelude::*;

/// Leptos component mounted at `?mount=bubble`. Renders a circular
/// placeholder so the empty transparent window is findable until
/// M-BUBBLE.2 wires the canvas.
#[component]
pub fn BubbleRoot() -> impl IntoView {
    view! {
        <div
            class="bubble-root"
            data-tauri-drag-region="true"
            aria-label="Webcam bubble overlay"
        >
            <div class="bubble-root-placeholder">
                "Webcam"
            </div>
        </div>
    }
}
