//! Microphone capture lifecycle (M-MIC.1 / AUT-278) + worker.
//!
//! Structural mirror of [`crate::preview`] (M-CAM.2 / M-CAM.3):
//! a four-state lifecycle [`MicLifecycle`] managed inside
//! [`MicCaptureState`] (Tauri-managed) plus a dedicated worker
//! thread defined in [`pipeline`] that owns the `gst-launch-1.0`
//! subprocess. The state machine is pure Rust (no `tauri::*`, no
//! async, no I/O) so its transition contract works on every OS
//! including Windows, where Tauri 2's `mock_builder` won't even
//! link at test time (per CLAUDE.md).

pub mod pipeline;

pub use pipeline::{MicCaptureHandle, MicCapturePipeline};

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Lifecycle state of the microphone capture worker.
///
/// Mirror of [`crate::preview::PreviewLifecycle`]. `Starting` /
/// `Stopping` exist so a re-entrant `start_mic_capture` can detect
/// "already booting, drop the new one" instead of double-spawning
/// gst children. `Running` is the steady state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MicLifecycle {
    /// No worker running.
    #[default]
    Idle,
    /// `start_mic_capture` invoked but no audio chunk has arrived
    /// yet (gst spawn + first-frame latency, ~100–300 ms on macOS
    /// `osxaudiosrc`).
    Starting,
    /// Worker is producing audio chunks.
    Running,
    /// `stop_mic_capture` invoked but the gst child is still being
    /// torn down. Transient — drops into `Idle` once reaped.
    Stopping,
}

/// IPC-surface error variants for the mic capture commands.
///
/// Mirror of [`crate::preview::CameraError`]. `PermissionPending`
/// is the macOS first-run case where the OS shows
/// `NSMicrophoneUsageDescription` and the gst pipeline blocks until
/// the user clicks. `PermissionDenied` is the post-prompt rejection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum MicError {
    /// macOS microphone-permission prompt is showing; no chunk has
    /// arrived yet but access isn't denied either.
    #[error("microphone permission prompt is pending user response")]
    PermissionPending,
    /// User has explicitly denied microphone access in System Settings.
    #[error("microphone access denied; user must grant in System Settings")]
    PermissionDenied,
    /// The selected microphone is held by another app, or — on macOS
    /// — the missing-Info.plist failure mode that masquerades as
    /// busy.
    #[error("microphone is busy or otherwise unavailable")]
    DeviceBusy,
    /// gst-launch pipeline spawn / runtime failure.
    #[error("gst pipeline failed: {0}")]
    GstFailed(String),
    /// The picker handed us a `mic_id` that no longer matches any
    /// device on the host — typically the mic was unplugged between
    /// `list_microphones()` and `start_mic_capture` (Bluetooth
    /// devices sleep, USB devices yanked). Caller should re-enumerate
    /// + re-prompt the user.
    ///
    /// M-MIC.3 / AUT-284 + M-RECORD-EXPORT tightening — was silently
    /// falling back to `autoaudiosrc`, which gave the wrong device.
    #[error("microphone id `{0}` not present on this host (was the mic unplugged?)")]
    NotFound(String),
}

/// Tauri-managed wrapper around [`MicLifecycle`]. Held in
/// `tauri::State` so the IPC handlers and the worker thread share
/// one source of truth for the lifecycle.
#[derive(Default)]
pub struct MicCaptureState(pub Mutex<MicLifecycle>);

impl MicLifecycle {
    /// Attempt to advance to `Starting`. Re-entrant calls (already
    /// Starting / Running / Stopping) are a no-op — the caller is
    /// expected to first stop the existing session.
    #[must_use]
    pub fn try_start(self) -> Self {
        match self {
            Self::Idle => Self::Starting,
            other => other,
        }
    }

    /// Mark the worker as running (first audio chunk received).
    /// Idempotent — `Running.mark_running() == Running`.
    #[must_use]
    pub fn mark_running(self) -> Self {
        match self {
            Self::Starting => Self::Running,
            other => other,
        }
    }

    /// Begin teardown — moves Running / Starting → Stopping. Stays
    /// in `Idle` if no worker is up.
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
        assert_eq!(MicLifecycle::default(), MicLifecycle::Idle);
    }

    #[test]
    fn idle_can_start() {
        assert_eq!(MicLifecycle::Idle.try_start(), MicLifecycle::Starting);
    }

    #[test]
    fn starting_cannot_start_again() {
        assert_eq!(MicLifecycle::Starting.try_start(), MicLifecycle::Starting);
    }

    #[test]
    fn running_cannot_start_again() {
        assert_eq!(MicLifecycle::Running.try_start(), MicLifecycle::Running);
    }

    #[test]
    fn starting_can_mark_running() {
        assert_eq!(MicLifecycle::Starting.mark_running(), MicLifecycle::Running);
    }

    #[test]
    fn mark_running_is_idempotent_on_running() {
        // First-chunk arrival can race with subsequent chunks; the
        // worker calls mark_running on every chunk to avoid an extra
        // "first-frame" guard — so the transition has to be a no-op
        // once Running is reached.
        assert_eq!(MicLifecycle::Running.mark_running(), MicLifecycle::Running);
    }

    #[test]
    fn idle_mark_running_is_noop() {
        // A spurious mark_running from a worker that wasn't actually
        // started (shouldn't happen, but the API has to be safe) must
        // not put us into Running without a try_start.
        assert_eq!(MicLifecycle::Idle.mark_running(), MicLifecycle::Idle);
    }

    #[test]
    fn running_can_stop() {
        assert_eq!(MicLifecycle::Running.try_stop(), MicLifecycle::Stopping);
    }

    #[test]
    fn starting_can_stop_before_first_chunk() {
        // User clicks stop during the macOS permission prompt: we
        // must move Starting → Stopping, not stay stuck Starting.
        assert_eq!(MicLifecycle::Starting.try_stop(), MicLifecycle::Stopping);
    }

    #[test]
    fn stopping_finishes_to_idle() {
        assert_eq!(MicLifecycle::Stopping.finish_stop(), MicLifecycle::Idle);
    }

    #[test]
    fn full_round_trip() {
        let mut s = MicLifecycle::default();
        s = s.try_start();
        assert_eq!(s, MicLifecycle::Starting);
        s = s.mark_running();
        assert_eq!(s, MicLifecycle::Running);
        s = s.try_stop();
        assert_eq!(s, MicLifecycle::Stopping);
        s = s.finish_stop();
        assert_eq!(s, MicLifecycle::Idle);
    }

    #[test]
    fn mic_error_round_trips_serde() {
        let cases = [
            MicError::PermissionPending,
            MicError::PermissionDenied,
            MicError::DeviceBusy,
            MicError::GstFailed("spawn failed: ENOENT".into()),
            MicError::NotFound("mic-cafebabe".into()),
        ];
        for err in cases {
            let json = serde_json::to_string(&err).unwrap();
            let back: MicError = serde_json::from_str(&json).unwrap();
            assert_eq!(back, err);
        }
    }
}
