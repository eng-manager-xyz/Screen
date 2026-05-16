//! Camera preview lifecycle (M-CAM.2 / AUT-256).
//!
//! Owns the [`PreviewSession`] state machine that Tauri-managed state
//! holds. M-CAM.3 (AUT-257) fills in the actual wisp + `GStreamer`
//! pipeline behind `start`/`stop`; this ticket lands the shell so the
//! Leptos side can invoke the commands and the click-through state
//! transitions are testable in isolation.
//!
//! The state machine is pure Rust (no `tauri::*` types, no async, no
//! I/O) so the four-state transition contract works on every OS
//! including Windows, where Tauri 2's `mock_builder` won't even link
//! at test-time (per CLAUDE.md).

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Lifecycle state of the camera preview pipeline.
///
/// `Idle` is the resting state. `Starting`/`Stopping` are transient
/// states that exist so a re-entrant `start_preview` call can detect
/// "already booting, drop the new one" instead of double-spawning gst
/// child processes. `Running` is the steady state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreviewLifecycle {
    /// No pipeline running.
    #[default]
    Idle,
    /// `start_preview` invoked but the pipeline isn't producing
    /// frames yet (gst spawn + first-frame latency).
    Starting,
    /// Pipeline is producing frames.
    Running,
    /// `stop_preview` invoked but the gst child is still being
    /// torn down. Transient — the click handler should drop into
    /// `Idle` once the child has been reaped.
    Stopping,
}

/// Error variants the IPC command surface can return to Leptos.
///
/// `PermissionPending` is the macOS first-run case where the OS
/// shows a prompt and the gst pipeline blocks until the user clicks.
/// `PermissionDenied` is the post-prompt rejection. The Leptos
/// `RecorderPreviewState` (M-CAM.3) maps each variant to the right
/// loading-state copy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum CameraError {
    /// macOS permission prompt is showing; the pipeline hasn't
    /// produced a frame yet but isn't denied either.
    #[error("camera permission prompt is pending user response")]
    PermissionPending,
    /// User has explicitly denied camera access in System Settings.
    #[error("camera access denied; user must grant in System Settings")]
    PermissionDenied,
    /// The selected camera is in use by another app (or, on macOS,
    /// the missing-Info.plist failure mode that masquerades as this).
    #[error("camera device is busy or otherwise unavailable")]
    DeviceBusy,
    /// gst pipeline spawn / runtime failure.
    #[error("gst pipeline failed: {0}")]
    GstFailed(String),
}

/// Tauri-managed wrapper around [`PreviewLifecycle`]. Held in
/// `tauri::State` so the IPC command handlers + the future frame
/// emitter share one source of truth.
#[derive(Default)]
pub struct PreviewState(pub Mutex<PreviewLifecycle>);

impl PreviewLifecycle {
    /// Attempt to advance to `Starting`. Returns the previous state
    /// (so callers can distinguish "started fresh" from "already
    /// running, refused").
    #[must_use]
    pub fn try_start(self) -> Self {
        match self {
            Self::Idle => Self::Starting,
            // Re-entrant start while in Starting/Running/Stopping is
            // a no-op at the state level — the caller is expected
            // to first stop the existing session.
            other => other,
        }
    }

    /// Mark the pipeline as running (first frame received).
    #[must_use]
    pub fn mark_running(self) -> Self {
        match self {
            Self::Starting => Self::Running,
            other => other,
        }
    }

    /// Begin teardown — moves Running → Stopping, or stays Idle if
    /// no pipeline is up.
    #[must_use]
    pub fn try_stop(self) -> Self {
        match self {
            Self::Running | Self::Starting => Self::Stopping,
            other => other,
        }
    }

    /// Complete teardown — moves Stopping → Idle.
    #[must_use]
    pub fn finish_stop(self) -> Self {
        match self {
            Self::Stopping => Self::Idle,
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_idle() {
        assert_eq!(PreviewLifecycle::default(), PreviewLifecycle::Idle);
    }

    #[test]
    fn idle_can_start() {
        assert_eq!(
            PreviewLifecycle::Idle.try_start(),
            PreviewLifecycle::Starting
        );
    }

    #[test]
    fn starting_cannot_start_again() {
        assert_eq!(
            PreviewLifecycle::Starting.try_start(),
            PreviewLifecycle::Starting
        );
    }

    #[test]
    fn running_cannot_start_again() {
        assert_eq!(
            PreviewLifecycle::Running.try_start(),
            PreviewLifecycle::Running
        );
    }

    #[test]
    fn starting_can_mark_running() {
        assert_eq!(
            PreviewLifecycle::Starting.mark_running(),
            PreviewLifecycle::Running
        );
    }

    #[test]
    fn running_can_stop() {
        assert_eq!(
            PreviewLifecycle::Running.try_stop(),
            PreviewLifecycle::Stopping
        );
    }

    #[test]
    fn stopping_finishes_to_idle() {
        assert_eq!(
            PreviewLifecycle::Stopping.finish_stop(),
            PreviewLifecycle::Idle
        );
    }

    #[test]
    fn full_round_trip() {
        let mut s = PreviewLifecycle::default();
        s = s.try_start();
        assert_eq!(s, PreviewLifecycle::Starting);
        s = s.mark_running();
        assert_eq!(s, PreviewLifecycle::Running);
        s = s.try_stop();
        assert_eq!(s, PreviewLifecycle::Stopping);
        s = s.finish_stop();
        assert_eq!(s, PreviewLifecycle::Idle);
    }

    #[test]
    fn camera_error_round_trips_serde() {
        let cases = [
            CameraError::PermissionPending,
            CameraError::PermissionDenied,
            CameraError::DeviceBusy,
            CameraError::GstFailed("spawn failed".into()),
        ];
        for err in cases {
            let json = serde_json::to_string(&err).unwrap();
            let back: CameraError = serde_json::from_str(&json).unwrap();
            assert_eq!(back, err);
        }
    }
}
