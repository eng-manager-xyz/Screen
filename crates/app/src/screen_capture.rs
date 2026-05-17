//! Tauri-managed state for the screen-capture session
//! (M-SCK.2 / AUT-269, partial — lifecycle commands only).
//!
//! Holds `Option<ScreenCaptureStream>` behind a `Mutex` so the
//! `screen_*` IPC commands share one source of truth. Mirror of
//! [`crate::system_audio::SystemAudioCaptureState`] for the video
//! capture path.
//!
//! ```admonish important title="Partial implementation"
//! Per the M-RECORDER-completeness PR scope, this commit ships the
//! lifecycle surface only — the frame `Channel<T>` that pipes BGRA
//! pixels to the Leptos side is intentionally skipped (the user's
//! "data delivered" exclusion). The `start_screen_capture` command
//! returns `Ok(())` once the SCStream is up; observable state
//! is the cumulative frame counter via `screen_capture_counters`.
//! ```
//!
//! macOS-only — `crate::media::sck_video::ScreenCaptureStream` is
//! gated on `#[cfg(target_os = "macos")]`.

#![cfg(target_os = "macos")]

use std::sync::Mutex;

use media::sck_video::{ScreenCaptureConfig, ScreenCaptureStream, ScreenError};

/// Tauri-managed wrapper. The four `screen_*` Tauri commands
/// (`list_screen_displays`, `start_screen_capture`,
/// `stop_screen_capture`, `screen_capture_status`) read from /
/// write to this.
#[derive(Default)]
pub struct ScreenCaptureState(pub Mutex<Option<ScreenCaptureStream>>);

impl ScreenCaptureState {
    /// Start a session. Drops any in-flight stream first so SCK
    /// isn't asked to run two captures simultaneously.
    pub fn start(&self, config: ScreenCaptureConfig) -> Result<(), ScreenError> {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = None;
        let stream = ScreenCaptureStream::new(config)?;
        *guard = Some(stream);
        Ok(())
    }

    /// Stop the active session, if any.
    pub fn stop(&self) {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = None;
    }

    /// Snapshot of the cumulative frame counter (`0` when no
    /// session is active). Used by the Leptos diagnostic overlay
    /// + future frame-rate monitor.
    #[must_use]
    pub fn frames_received(&self) -> u64 {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map_or(0, |s| s.counters().frames_received())
    }

    /// `true` when a session is currently held.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_inactive() {
        let state = ScreenCaptureState::default();
        assert!(!state.is_active());
        assert_eq!(state.frames_received(), 0);
    }

    #[test]
    fn stop_is_idempotent_when_inactive() {
        let state = ScreenCaptureState::default();
        state.stop();
        state.stop();
        assert!(!state.is_active());
    }
}
