//! Tauri-managed state for the screen-capture session
//! (M-SCK.2 / AUT-269, partial — lifecycle commands only).
//!
//! Holds `Option<ScreenCaptureStream>` behind a `Mutex` so the
//! `screen_*` IPC commands share one source of truth. Mirror of
//! [`crate::system_audio::SystemAudioCaptureState`] for the video
//! capture path.
//!
//! ```admonish note title="Preview frame channel (AUT-269)"
//! The preview frame channel ships: `start_screen_capture` plumbs a
//! **downscaled** [`ScreenFrameSlot`] (see [`PREVIEW_WIDTH`]) into the
//! SCK delegate, and `latest_screen_frame_bgra` returns the latest
//! frame to the webview — the same poll pattern as the camera preview
//! ([`crate::commands::latest_camera_frame_bgra`]). The recording path
//! is unaffected: it passes its own full-resolution slot via
//! [`Self::start_with_frame_slot`], bypassing the preview slot.
//! ```
//!
//! macOS-only — `crate::media::sck_video::ScreenCaptureStream` is
//! gated on `#[cfg(target_os = "macos")]`.

#![cfg(target_os = "macos")]

use std::sync::Mutex;

use media::sck_video::{ScreenCaptureConfig, ScreenCaptureStream, ScreenError, ScreenFrameSlot};

/// Downscaled preview-capture width (AUT-269). The recorder preview polls the
/// composed frame over IPC at ~15 fps; the full 1920×1080 capture is 8 MB/frame
/// (124 MB/s) — far too heavy for a smooth poll. 1280×720 is ~3.7 MB/frame,
/// crisp enough for a live preview, and the `SCStream` downscales the display
/// into it. The recording path keeps native resolution (it uses its own slot).
pub const PREVIEW_WIDTH: u32 = 1280;
/// See [`PREVIEW_WIDTH`]. 16:9; the `SCStream` scales the display into this box.
pub const PREVIEW_HEIGHT: u32 = 720;

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
pub fn bubble_window_cg_id(app: &tauri::AppHandle) -> Option<u32> {
    window_cg_id(app, "webcam-bubble")
}

/// The recorder's **own** window `CGWindowID`s (AUT-269) — `main`,
/// `webcam-bubble`, `tray-popover` — for [`ScreenCaptureConfig::excluded_window_ids`]
/// so a live screen *preview* doesn't capture the recorder UI showing the
/// preview (the screen-of-its-own-screen feedback loop). Missing / not-yet-shown
/// windows are simply skipped.
#[must_use]
pub fn own_window_cg_ids(app: &tauri::AppHandle) -> Vec<u32> {
    ["main", "webcam-bubble", "tray-popover"]
        .into_iter()
        .filter_map(|label| window_cg_id(app, label))
        .collect()
}

/// Resolve a Tauri window label's `CGWindowID` (== `NSWindow.windowNumber`).
///
/// Returns `None` if the window isn't registered, Tauri can't hand us the raw
/// `NSWindow`, or the `NSWindow` hasn't been assigned a number yet (`<= 0`,
/// before it has ever been shown). All paths log via `tracing::debug!`.
///
/// Uses the `NSWindow` → `windowNumber` → `CGWindowID` equivalence documented at
/// <https://developer.apple.com/documentation/appkit/nswindow/1419068-windownumber>.
#[must_use]
#[allow(
    unsafe_code,
    reason = "objc2 msg_send to NSWindow for windowNumber. The selector is no-arg, NSInteger return; sound to call on any non-null NSWindow*."
)]
pub fn window_cg_id(app: &tauri::AppHandle, label: &str) -> Option<u32> {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use tauri::Manager;

    let window = app.get_webview_window(label)?;
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

/// Tauri-managed wrapper for the screen-capture preview session. The
/// `screen_*` Tauri commands (`list_screen_displays`,
/// `start_screen_capture`, `stop_screen_capture`, `screen_capture_status`,
/// `latest_screen_frame_bgra`) read from / write to this.
#[derive(Default)]
pub struct ScreenCaptureState {
    /// The active SCK stream, if any.
    stream: Mutex<Option<ScreenCaptureStream>>,
    /// Latest downscaled preview frame (AUT-269). [`Self::start`] plumbs this
    /// into the SCK delegate; `latest_screen_frame_bgra` reads it. The
    /// recording path passes its own full-res slot via
    /// [`Self::start_with_frame_slot`] and never touches this one.
    preview_slot: ScreenFrameSlot,
}

impl ScreenCaptureState {
    /// Start a **preview** session: the captured frames land in the shared
    /// preview slot ([`Self::latest_frame`]) so the webview can poll them.
    /// Drops any in-flight stream first so SCK isn't asked to run two
    /// captures simultaneously.
    pub fn start(&self, config: ScreenCaptureConfig) -> Result<(), ScreenError> {
        let slot = ScreenFrameSlot::clone(&self.preview_slot);
        self.start_with_frame_slot(config, Some(slot))
    }

    /// Same as [`Self::start`] but plumbs an explicit M-PIX.2 frame slot
    /// into the SCK delegate so each captured frame's BGRA bytes are
    /// written to it. The recording orchestrator uses this with its own
    /// full-resolution slot.
    pub fn start_with_frame_slot(
        &self,
        config: ScreenCaptureConfig,
        frame_slot: Option<ScreenFrameSlot>,
    ) -> Result<(), ScreenError> {
        let mut guard = self
            .stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = None;
        let stream = ScreenCaptureStream::new_with_frame_slot(config, frame_slot)?;
        *guard = Some(stream);
        Ok(())
    }

    /// Stop the active session, if any. Clears the preview slot so a stale
    /// frame can't paint after the preview is torn down.
    pub fn stop(&self) {
        let mut guard = self
            .stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = None;
        *self
            .preview_slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    /// The latest preview frame's BGRA bytes (AUT-269), or empty when no
    /// frame has arrived. Cloned so the caller doesn't hold the slot lock.
    #[must_use]
    pub fn latest_frame(&self) -> Vec<u8> {
        self.preview_slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .unwrap_or_default()
    }

    /// Snapshot of the cumulative frame counter (`0` when no
    /// session is active). Used by the Leptos diagnostic overlay
    /// + future frame-rate monitor.
    #[must_use]
    pub fn frames_received(&self) -> u64 {
        self.stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map_or(0, |s| s.counters().frames_received())
    }

    /// `true` when a session is currently held.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.stream
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
