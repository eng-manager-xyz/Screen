//! M-CAM.3 / AUT-257 — runtime diagnostics for the camera pipeline.
//!
//! Lives in `tauri::State` so the worker thread can mutate cheap
//! atomic counters per frame without ever holding a mutex on the
//! hot path, and the Leptos overlay can poll a snapshot via the
//! `preview_diagnostics` IPC command at a much lower rate (~2 Hz).
//!
//! ```admonish important title="Why atomics, not a single Mutex<Stats>"
//! `WindowEvent::Moved` for the bubble was already locking a mutex
//! per drag event and that's fine — drag is at most 60 Hz. But the
//! camera worker pushes 30 frames per second, and `preview_status`
//! polling from Leptos lands ~2 Hz. A `Mutex<Stats>` would serialise
//! both producer + consumer on the same lock. Atomic `u64` / `u32`
//! reads + writes are wait-free; the consumer just snapshots whatever
//! was last written without blocking the worker.
//! ```
//!
//! The first-received frame is also dumped to PNG (one-shot, behind
//! a `PathBuf` mutex set only once) so the user has visual proof
//! that real pixels reached Rust — see [`maybe_dump_first_frame`].

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tauri::Manager;

/// Tauri-managed diagnostic counters for the camera pipeline.
///
/// All counters reset on `start_preview` so the user gets a
/// fresh-from-zero ticker each session — easier to eyeball "the
/// number is going up" without comparing against a baseline.
#[derive(Default)]
pub struct PreviewDiagnostics {
    /// Number of frames the worker has pulled from gst since the
    /// current session started. Resets on each `start_preview`.
    pub frames_received: AtomicU64,
    /// Source frame width in pixels (as reported by gst's caps
    /// negotiation). `0` until first frame.
    pub source_width: AtomicU32,
    /// Source frame height in pixels. `0` until first frame.
    pub source_height: AtomicU32,
    /// Source framerate × 100 (encoded as integer for atomic
    /// storage; 30 fps → `3000`, 29.97 → `2997`). `0` until known.
    pub source_fps_hundredths: AtomicU32,
    /// Path of the first-frame PNG dump, if one has been written
    /// this session. `None` until first frame dumps successfully.
    pub first_frame_dump_path: Mutex<Option<PathBuf>>,
}

impl PreviewDiagnostics {
    /// Reset all counters. Called on every `start_preview` so the
    /// user sees a fresh ticker per session.
    pub fn reset(&self) {
        self.frames_received.store(0, Ordering::Relaxed);
        self.source_width.store(0, Ordering::Relaxed);
        self.source_height.store(0, Ordering::Relaxed);
        self.source_fps_hundredths.store(0, Ordering::Relaxed);
        *self
            .first_frame_dump_path
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    /// Record that one frame just arrived. Worker thread calls this
    /// on every successful `next_frame()`.
    pub fn record_frame(&self) {
        self.frames_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Record the source dims + fps once they're known (after the
    /// gst caps negotiation completes — practically, on opening the
    /// stream). Cheap to call repeatedly with the same values.
    pub fn record_source(&self, width: u32, height: u32, fps: f64) {
        self.source_width.store(width, Ordering::Relaxed);
        self.source_height.store(height, Ordering::Relaxed);
        // Clamp the float into the u32 range as hundredths. 60 fps
        // → 6000 fits; even 600 fps → 60_000 fits comfortably.
        let hundredths = (fps * 100.0).clamp(0.0, f64::from(u32::MAX));
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "value clamped to [0, u32::MAX] above"
        )]
        let hundredths_u32 = hundredths as u32;
        self.source_fps_hundredths
            .store(hundredths_u32, Ordering::Relaxed);
    }

    /// Set the first-frame dump path (one-shot). Returns `true` on
    /// the first call (caller should encode + write the PNG),
    /// `false` on subsequent calls (already dumped this session).
    pub fn try_claim_dump_slot(&self, path: PathBuf) -> bool {
        let mut guard = self
            .first_frame_dump_path
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.is_some() {
            return false;
        }
        *guard = Some(path);
        true
    }

    /// Read a serialisable snapshot. The IPC command returns this;
    /// Leptos polls every 500ms.
    #[must_use]
    pub fn snapshot(&self) -> DiagnosticsSnapshot {
        let path = self
            .first_frame_dump_path
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned());
        DiagnosticsSnapshot {
            frames_received: self.frames_received.load(Ordering::Relaxed),
            source_width: self.source_width.load(Ordering::Relaxed),
            source_height: self.source_height.load(Ordering::Relaxed),
            source_fps_hundredths: self.source_fps_hundredths.load(Ordering::Relaxed),
            first_frame_dump_path: path,
        }
    }
}

/// Wire-format snapshot of [`PreviewDiagnostics`]. Returned by the
/// `preview_diagnostics` IPC command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticsSnapshot {
    /// Total frames received since `start_preview`.
    pub frames_received: u64,
    /// Source frame width in pixels. `0` if no frames yet.
    pub source_width: u32,
    /// Source frame height in pixels. `0` if no frames yet.
    pub source_height: u32,
    /// Source framerate × 100. `0` if unknown. Leptos divides by 100
    /// for display.
    pub source_fps_hundredths: u32,
    /// Absolute path of the first-frame PNG dump, or `None` if no
    /// dump succeeded this session.
    pub first_frame_dump_path: Option<String>,
}

/// One-shot writer: on the very first frame of each session, dump
/// the BGRA bytes to a PNG file under the app cache dir so the user
/// can confirm visually that real pixels reached Rust. Returns the
/// absolute path on success; logs + returns `None` on encode/write
/// failure (we don't want a diagnostic to crash the worker).
///
/// Subsequent calls within the same session no-op (the dump slot is
/// claimed once via [`PreviewDiagnostics::try_claim_dump_slot`]).
pub fn maybe_dump_first_frame(
    app: &tauri::AppHandle,
    diagnostics: &PreviewDiagnostics,
    bgra: &[u8],
    width: u32,
    height: u32,
) {
    let Ok(cache_dir) = app.path().app_cache_dir() else {
        tracing::warn!("app_cache_dir unavailable; first-frame dump skipped");
        return;
    };
    let path = cache_dir.join("first-frame.png");
    if !diagnostics.try_claim_dump_slot(path.clone()) {
        // Already dumped this session — fast path.
        return;
    }
    match write_bgra_as_png(&path, bgra, width, height) {
        Ok(()) => {
            tracing::info!(?path, "first-frame PNG dumped");
        }
        Err(err) => {
            tracing::warn!(?err, ?path, "first-frame PNG dump failed");
            // Clear the slot so a subsequent successful frame can retry.
            *diagnostics
                .first_frame_dump_path
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        }
    }
}

/// Encode + write BGRA bytes as a PNG. Splits the BGRA→RGBA byte
/// swap from the encode call so the swap itself is unit-testable.
fn write_bgra_as_png(
    path: &std::path::Path,
    bgra: &[u8],
    width: u32,
    height: u32,
) -> Result<(), String> {
    let rgba = bgra_to_rgba(bgra);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    let buf = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| "RgbaImage::from_raw rejected the buffer (size mismatch)".to_owned())?;
    buf.save_with_format(path, image::ImageFormat::Png)
        .map_err(|e| format!("save: {e}"))
}

/// In-place byte swap on each pixel: BGRA → RGBA. Allocates a new
/// `Vec<u8>` rather than mutating in place so the worker's frame
/// buffer is preserved (the follow-up wisp commit will upload the
/// same BGRA bytes to `VideoTexture` and benefits from byte order
/// matching the wisp expectation).
#[must_use]
fn bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(bgra.len());
    for chunk in bgra.chunks_exact(4) {
        // chunk = [B, G, R, A] → push [R, G, B, A]
        rgba.push(chunk[2]);
        rgba.push(chunk[1]);
        rgba.push(chunk[0]);
        rgba.push(chunk[3]);
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_reset_clears_counters() {
        let d = PreviewDiagnostics::default();
        d.record_frame();
        d.record_frame();
        d.record_source(640, 480, 30.0);
        assert_eq!(d.frames_received.load(Ordering::Relaxed), 2);
        assert_eq!(d.source_width.load(Ordering::Relaxed), 640);
        d.reset();
        assert_eq!(d.frames_received.load(Ordering::Relaxed), 0);
        assert_eq!(d.source_width.load(Ordering::Relaxed), 0);
        assert_eq!(d.source_height.load(Ordering::Relaxed), 0);
        assert_eq!(d.source_fps_hundredths.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn diagnostics_records_fps_as_hundredths() {
        let d = PreviewDiagnostics::default();
        d.record_source(640, 480, 30.0);
        assert_eq!(d.source_fps_hundredths.load(Ordering::Relaxed), 3000);
        d.record_source(640, 480, 29.97);
        assert_eq!(d.source_fps_hundredths.load(Ordering::Relaxed), 2997);
    }

    #[test]
    fn try_claim_dump_slot_is_one_shot() {
        let d = PreviewDiagnostics::default();
        assert!(d.try_claim_dump_slot(PathBuf::from("/tmp/a.png")));
        assert!(!d.try_claim_dump_slot(PathBuf::from("/tmp/b.png")));
        // Reset re-opens the slot for the next session.
        d.reset();
        assert!(d.try_claim_dump_slot(PathBuf::from("/tmp/c.png")));
    }

    #[test]
    fn snapshot_round_trips_through_serde() {
        let snapshot = DiagnosticsSnapshot {
            frames_received: 42,
            source_width: 640,
            source_height: 480,
            source_fps_hundredths: 2997,
            first_frame_dump_path: Some("/tmp/first-frame.png".to_owned()),
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let back: DiagnosticsSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back, snapshot);
    }

    #[test]
    fn bgra_to_rgba_swaps_red_and_blue_channels() {
        // Single pixel: B=0x11, G=0x22, R=0x33, A=0x44.
        let bgra = vec![0x11, 0x22, 0x33, 0x44];
        let rgba = bgra_to_rgba(&bgra);
        // Expect R=0x33, G=0x22, B=0x11, A=0x44.
        assert_eq!(rgba, vec![0x33, 0x22, 0x11, 0x44]);
    }

    #[test]
    fn bgra_to_rgba_handles_multi_pixel_buffers() {
        // 2 pixels: (B0,G0,R0,A0), (B1,G1,R1,A1).
        let bgra = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let rgba = bgra_to_rgba(&bgra);
        // (R0=3,G0=2,B0=1,A0=4), (R1=7,G1=6,B1=5,A1=8).
        assert_eq!(rgba, vec![3, 2, 1, 4, 7, 6, 5, 8]);
    }

    #[test]
    fn bgra_to_rgba_truncates_incomplete_trailing_pixel() {
        // chunks_exact drops a non-4-byte trailing remainder. Verify
        // the resulting RGBA length is the largest multiple of 4 ≤
        // input length.
        let bgra = vec![1, 2, 3, 4, 5, 6, 7]; // 7 bytes (one full pixel + 3 extra)
        let rgba = bgra_to_rgba(&bgra);
        assert_eq!(rgba.len(), 4);
    }
}
