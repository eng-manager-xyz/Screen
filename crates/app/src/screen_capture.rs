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

/// Resolve the `webcam-bubble` Tauri window's `CGWindowID` so it can
/// be passed to [`ScreenCaptureConfig::excluded_window_ids`].
///
/// Returns `None` if (a) the bubble window isn't registered (e.g. the
/// tauri.conf.json label was renamed), (b) Tauri can't hand us the
/// raw `NSWindow`, or (c) the `NSWindow` hasn't been assigned a
/// number yet (returned `<= 0`, which happens before the window has
/// ever been shown). All paths log via `tracing::debug!` so callers
/// don't have to branch on the source of the miss — the screen
/// capture falls back to an empty exclusion list (current behaviour,
/// with the dup bubble).
///
/// Uses the `NSWindow` → `windowNumber` → `CGWindowID` equivalence
/// documented at <https://developer.apple.com/documentation/appkit/nswindow/1419068-windownumber>.
#[must_use]
#[allow(
    unsafe_code,
    reason = "objc2 msg_send to NSWindow for windowNumber. The selector is no-arg, NSInteger return; sound to call on any non-null NSWindow*."
)]
pub fn bubble_window_cg_id(app: &tauri::AppHandle) -> Option<u32> {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use tauri::Manager;

    let window = app.get_webview_window("webcam-bubble")?;
    let ns_window_ptr = match window.ns_window() {
        Ok(ptr) => ptr.cast::<AnyObject>(),
        Err(err) => {
            tracing::debug!(?err, "bubble_window_cg_id: ns_window() unavailable");
            return None;
        }
    };
    if ns_window_ptr.is_null() {
        return None;
    }
    // SAFETY: `WebviewWindow::ns_window()` on macOS returns a
    // non-null pointer to the underlying NSWindow when Ok. NSWindow
    // implements `- (NSInteger)windowNumber` (no-arg, NSInteger
    // return) — calling that selector is sound for any NSWindow*.
    let window_number: isize = unsafe { msg_send![ns_window_ptr, windowNumber] };
    if window_number <= 0 {
        tracing::debug!(
            window_number,
            "bubble_window_cg_id: NSWindow hasn't been assigned a number yet (shown < once?)"
        );
        return None;
    }
    u32::try_from(window_number).ok()
}

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
        self.start_with_frame_slot(config, None)
    }

    /// Same as [`Self::start`] but plumbs the M-PIX.2 frame slot
    /// into the SCK delegate so each captured frame's BGRA bytes
    /// are written to the shared slot for the encoder feed thread.
    pub fn start_with_frame_slot(
        &self,
        config: ScreenCaptureConfig,
        frame_slot: Option<media::sck_video::ScreenFrameSlot>,
    ) -> Result<(), ScreenError> {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = None;
        let stream = ScreenCaptureStream::new_with_frame_slot(config, frame_slot)?;
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
