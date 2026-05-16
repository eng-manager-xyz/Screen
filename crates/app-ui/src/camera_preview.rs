//! `<CameraPreview />` — Leptos canvas painted by wisp's masked
//! camera output (M-CAM.3 / AUT-257).
//!
//! Owns the `RecorderPreviewState` enum that drives the four UX
//! states (`Initialising` / `AwaitingPermission` / `PermissionDenied`
//! / `Live`), and a `<canvas>` element identified by a stable DOM
//! id so M-CAM.3's frame channel can `putImageData` directly into it.
//!
//! ```admonish important title="Pipeline status as of this commit"
//! M-CAM.3's wisp + readback pipeline is **partially landed**: the
//! Leptos UI scaffolding + state machine + canvas mount are in
//! place, but the Rust-side `start_preview` body is still a state-
//! only stub (M-CAM.2). The full pipeline (gst → wisp::Stage with
//! M-VEC.6 circle mask → offscreen RT → BGRA readback → channel
//! emit → putImageData) lands in a follow-up commit that doesn't
//! fit one session. The UI is ready to receive frames the moment
//! they start flowing.
//! ```
//!
//! ```admonish warning title="Mask is wisp's, not CSS"
//! Do NOT add `border-radius: 50%` to `.camera-preview`. The mask
//! is rendered into the wisp scene + baked into the readback
//! bytes. CSS-rounding the same edge would visibly double-crop the
//! circle (a 1-2 px band of partial-alpha pixels).
//! ```

use leptos::prelude::*;

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

/// `<CameraPreview />` — the recorder surface's webcam preview.
///
/// The component renders a 480×480 `<canvas>` plus an overlaid copy
/// element that displays the current `RecorderPreviewState`. The
/// canvas paints via wisp output → putImageData (wired by the
/// M-CAM.3 Tauri frame channel once it lands).
#[component]
pub fn CameraPreview() -> impl IntoView {
    // Static state for v0: starts in Initialising, transitions to
    // Live when the first frame is painted (driven by the Tauri
    // frame channel — wired in the M-CAM.3 follow-up). The state
    // lives in a signal so the channel listener can flip it from
    // outside the component scope.
    let state = RwSignal::new(RecorderPreviewState::default());
    view! {
        <section
            class="camera-preview-surface"
            data-state=move || state.get().slug()
        >
            <canvas
                id=CANVAS_DOM_ID
                class="camera-preview"
                width=480
                height=480
                aria-label="Live webcam preview, masked to a circle by wisp"
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
