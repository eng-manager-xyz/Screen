//! macOS display + window enumeration (M-SCK.1 / AUT-268).
//!
//! Pure data API mirroring [`crate::camera::list_cameras`] and
//! [`crate::microphone::list_microphones`]. Both functions go through
//! `SCShareableContent` (the same async path the system-audio
//! capture uses for app enumeration) and return typed `DisplaySource`
//! / `WindowSource` structs the recorder UI can pick from.
//!
//! ```admonish important title="No SCK *capture* here"
//! This module is enumeration-only. The actual frame-source SCK
//! pipeline (`SCStream` + `SCStreamConfiguration` for video output)
//! lives in M-SCK.0 (`crate::sck_video` — separate module). Keeping
//! them split mirrors the audio side's `microphone.rs` (enumeration)
//! vs `gstreamer_audio.rs` (capture) split.
//! ```
//!
//! macOS-only — Linux + Windows would need different per-OS
//! enumeration APIs (`pipewire-rs` listing on Linux,
//! `IDXGIOutputDuplication` enumeration on Windows). Cross-OS
//! support is out of scope for the M-SCK milestone; this module is
//! `#[cfg(target_os = "macos")]` and the public API on non-macOS
//! targets returns the `NotMacOs` error variant.

#![cfg(target_os = "macos")]
#![allow(
    unsafe_code,
    reason = "ScreenCaptureKit accessors (displayID, width, frame, title, owningApplication) are all unsafe FFI by objc2's design; each unsafe block has a SAFETY justification next to it."
)]

use serde::{Deserialize, Serialize};

use crate::sck_audio::{SystemAudioError, shareable_content_blocking};

/// Result of [`list_displays`] / [`list_windows`]. Re-uses
/// `SystemAudioError` because the underlying failure modes are
/// identical (SCK refused, TCC permission denied, etc.) — keeping
/// one error type means the M-SCK Tauri commands can surface
/// failures with the same string-conversion as the audio path.
pub type ScreenError = SystemAudioError;

/// One capturable display attached to the system.
///
/// `id` is `"display-<displayID>"` where `displayID` is the macOS
/// `CGDirectDisplayID`. Stable across reboots for built-in displays;
/// external displays can change IDs when unplugged + replugged into
/// a different port — same gotcha class as the camera + mic
/// AVFoundation-id situation. The picker should persist by `id` and
/// gracefully fall back to "first listed" when the persisted id no
/// longer matches.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DisplaySource {
    /// Stable identifier (`display-<displayID>`).
    pub id: String,
    /// Human-readable label (`"Display 1920×1080"`).
    pub label: String,
    /// Width in points (logical pixels, *not* backing pixels).
    /// For Retina displays the backing pixel count is `2 * width`.
    pub width: u32,
    /// Height in points.
    pub height: u32,
    /// `true` for the first display in the OS enumeration. There's
    /// no canonical "primary display" concept SCK exposes; first-
    /// in-list maps closely to "the display with the menubar" on
    /// most setups.
    pub is_primary: bool,
}

/// One on-screen window the user could capture.
///
/// `id` is `"window-<windowID>"` where `windowID` is the macOS
/// `CGWindowID`. Windows are short-lived (close on app quit) and
/// IDs are recycled aggressively — persistence by id is a worse
/// idea than for displays. The picker should re-enumerate on every
/// open instead of remembering ids across sessions.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowSource {
    /// Stable identifier (`window-<windowID>`) for the current
    /// session. NOT stable across app launches.
    pub id: String,
    /// Window title (`"GitHub – screen"`). `String::new()` for
    /// windows with no title set (uncommon — most apps title their
    /// windows).
    pub label: String,
    /// Width in points.
    pub width: u32,
    /// Height in points.
    pub height: u32,
    /// The owning app's bundle identifier
    /// (`"com.google.Chrome"`), or empty if SCK couldn't resolve
    /// it (system services without a bundle).
    pub bundle_id: String,
    /// The owning app's display name (`"Google Chrome"`). Used in
    /// the picker as "<app> — <window title>".
    pub display_name: String,
}

/// Enumerate every display the OS exposes to SCK
/// (M-SCK.1 / AUT-268).
///
/// Triggers the macOS Screen Recording TCC prompt on first run if
/// not yet granted. The error path surfaces the standard SCK
/// `"The user declined TCCs..."` message — the picker UX surfaces
/// it inline with the M-RECP.6 deep-link button.
///
/// # Errors
///
/// Returns [`ScreenError::EnumerationFailed`] when SCK refuses
/// (permission denied, framework-init failure).
pub fn list_displays() -> Result<Vec<DisplaySource>, ScreenError> {
    let content = shareable_content_blocking()?;
    let displays = unsafe { content.displays() };
    let mut out: Vec<DisplaySource> = Vec::with_capacity(displays.len());
    for (idx, display) in displays.iter().enumerate() {
        // SAFETY: width / height / displayID are property accessors
        // with no instance state. SCK guarantees the SCDisplay is
        // live for the iterator's lifetime.
        let width_raw = unsafe { display.width() };
        let height_raw = unsafe { display.height() };
        let display_id = unsafe { display.displayID() };
        let width = u32::try_from(width_raw).unwrap_or(0);
        let height = u32::try_from(height_raw).unwrap_or(0);
        out.push(DisplaySource {
            id: format!("display-{display_id}"),
            label: format!("Display {width}×{height}"),
            width,
            height,
            is_primary: idx == 0,
        });
    }
    Ok(out)
}

/// Enumerate every on-screen, capturable window
/// (M-SCK.1 / AUT-268).
///
/// Filters to windows that are:
///
/// - On a `windowLayer` of 0 (normal app windows — excludes the
///   menubar, dock, screensaver overlays, etc.)
/// - `isOnScreen` (excludes minimised + occluded windows the user
///   can't see right now).
///
/// Triggers the macOS Screen Recording TCC prompt on first run.
///
/// # Errors
///
/// Returns [`ScreenError::EnumerationFailed`] when SCK refuses.
pub fn list_windows() -> Result<Vec<WindowSource>, ScreenError> {
    let content = shareable_content_blocking()?;
    let windows = unsafe { content.windows() };
    let mut out: Vec<WindowSource> = Vec::with_capacity(windows.len());
    for window in &windows {
        // SAFETY: all four accessors are no-arg property reads; SCK
        // guarantees the SCWindow is live for the iterator's
        // lifetime; isOnScreen + windowLayer are pure bool / int.
        let on_screen = unsafe { window.isOnScreen() };
        let layer = unsafe { window.windowLayer() };
        if !on_screen || layer != 0 {
            continue;
        }
        let window_id = unsafe { window.windowID() };
        let frame = unsafe { window.frame() };
        // CGRect's size fields are CGFloat (f64 on 64-bit Apple
        // platforms). Negative dims would indicate a SCK bug;
        // clamp at 0 defensively.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "CGRect dims fit comfortably in u32 for any real display"
        )]
        let width = frame.size.width.max(0.0) as u32;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "CGRect dims fit comfortably in u32 for any real display"
        )]
        let height = frame.size.height.max(0.0) as u32;
        let title = unsafe { window.title() }
            .map(|s| s.to_string())
            .unwrap_or_default();
        let (bundle_id, display_name) = match unsafe { window.owningApplication() } {
            Some(app) => {
                let bid = unsafe { app.bundleIdentifier() }.to_string();
                let name = unsafe { app.applicationName() }.to_string();
                (bid, name)
            }
            None => (String::new(), String::new()),
        };
        out.push(WindowSource {
            id: format!("window-{window_id}"),
            label: title,
            width,
            height,
            bundle_id,
            display_name,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_source_serde_round_trip_preserves_every_field() {
        let display = DisplaySource {
            id: "display-1".into(),
            label: "Display 1920×1080".into(),
            width: 1920,
            height: 1080,
            is_primary: true,
        };
        let json = serde_json::to_string(&display).unwrap();
        let back: DisplaySource = serde_json::from_str(&json).unwrap();
        assert_eq!(back, display);
    }

    #[test]
    fn window_source_serde_round_trip_preserves_every_field() {
        let window = WindowSource {
            id: "window-42".into(),
            label: "GitHub – screen".into(),
            width: 1280,
            height: 720,
            bundle_id: "com.google.Chrome".into(),
            display_name: "Google Chrome".into(),
        };
        let json = serde_json::to_string(&window).unwrap();
        let back: WindowSource = serde_json::from_str(&json).unwrap();
        assert_eq!(back, window);
    }

    #[test]
    fn types_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DisplaySource>();
        assert_send_sync::<WindowSource>();
    }
}
