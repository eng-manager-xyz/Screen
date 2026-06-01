//! Cursor telemetry capture (ED.17 / M-EDIT).
//!
//! Records where the cursor was during a recording so the editor can drive
//! the cursor overlay (ED.19) and auto-zoom (ED.17's already-tested
//! [`auto_zoom_segments`](edit::telemetry::auto_zoom_segments)) consumer.
//!
//! ## What this captures, and what it doesn't
//!
//! The **cursor position track** is captured by polling the global cursor
//! location — `CGEventCreate(NULL)` + `CGEventGetLocation`, which read the
//! current pointer position with **no Input-Monitoring permission** and no
//! event tap. A background thread samples at ~60 Hz; at stop the timestamped
//! samples are resampled onto the project frame grid ([`samples_to_track`]).
//!
//! The **click log** (for click ripples + auto-zoom) needs a `CGEventTap`,
//! which *does* require Input-Monitoring permission and a `CFRunLoop`
//! callback — see ISS-16. The data model + the plumbing are ready for it;
//! this module ships the no-permission position half.
//!
//! The pure parts ([`normalize_cursor_to_frame`], [`samples_to_track`]) are
//! exhaustively unit-tested; the macOS poller thread is runtime-only.

use std::time::Duration;

use edit::CursorSample;

/// Normalize a global cursor `point` to `[0, 1]²` within the captured display
/// `rect` (`(origin_x, origin_y, width, height)`), top-left origin — the
/// [`CursorSample`] convention. Points outside the rect clamp to the edge; a
/// zero-size axis maps to `0.0`. Pure.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the normalized result is in [0, 1], well within f32 precision"
)]
pub fn normalize_cursor_to_frame(point: (f64, f64), rect: (f64, f64, f64, f64)) -> (f32, f32) {
    let (px, py) = point;
    let (rx, ry, rw, rh) = rect;
    let nx = if rw > 0.0 {
        ((px - rx) / rw).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let ny = if rh > 0.0 {
        ((py - ry) / rh).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (nx as f32, ny as f32)
}

/// Resample timestamped cursor samples (`(elapsed_since_start, x, y)`, sorted
/// by time) onto the project frame grid: each sample's frame is
/// `floor(elapsed_secs · project_fps)`, and the latest sample at a frame
/// wins (one [`CursorSample`] per frame). Pure.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "elapsed·fps is a non-negative frame index well under 2^52; the f64→u64 cast is floor of a clamped-non-negative value"
)]
pub fn samples_to_track(samples: &[(Duration, f32, f32)], project_fps: u32) -> Vec<CursorSample> {
    let fps = f64::from(project_fps.max(1));
    let mut out: Vec<CursorSample> = Vec::new();
    for &(t, x, y) in samples {
        let frame = (t.as_secs_f64() * fps).max(0.0) as u64;
        match out.last_mut() {
            // Collapse consecutive samples that land on the same frame —
            // keep the latest position at that frame.
            Some(last) if last.frame == frame => {
                last.x = x;
                last.y = y;
            }
            _ => out.push(CursorSample::new(frame, x, y)),
        }
    }
    out
}

/// The main display's bounds in CG points (`(origin_x, origin_y, width,
/// height)`) — the rect [`CursorPoller`] normalizes the global cursor against.
/// On non-macOS (no capture) returns a 1080p placeholder. The captured
/// display is assumed to be the main one (multi-display targeting is a
/// refinement — see ISS-17).
#[must_use]
pub fn main_display_bounds() -> (f64, f64, f64, f64) {
    #[cfg(target_os = "macos")]
    {
        let bounds = objc2_core_graphics::CGDisplayBounds(objc2_core_graphics::CGMainDisplayID());
        (
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            bounds.size.height,
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        (0.0, 0.0, 1920.0, 1080.0)
    }
}

/// Parse a `CGDirectDisplayID` out of a recording's screen-source id
/// (`"display-<id>"`). Returns `None` for the primary display (`None` / `""`),
/// a window source (`"window-..."`), or a malformed id. Pure.
#[must_use]
pub fn parse_display_id(source_id: Option<&str>) -> Option<u32> {
    source_id?.strip_prefix("display-")?.parse::<u32>().ok()
}

/// Bounds (CG points) of the *captured* display, from the recording's
/// screen-source id (ISS-17). `"display-<id>"` → that display's
/// [`main_display_bounds`]-style rect; primary / window / malformed → the main
/// display (window-source framing is a further refinement). Non-macOS: the
/// 1080p placeholder.
#[must_use]
pub fn display_bounds_for_source(source_id: Option<&str>) -> (f64, f64, f64, f64) {
    #[cfg(target_os = "macos")]
    {
        if let Some(id) = parse_display_id(source_id) {
            let bounds = objc2_core_graphics::CGDisplayBounds(id);
            return (
                bounds.origin.x,
                bounds.origin.y,
                bounds.size.width,
                bounds.size.height,
            );
        }
        main_display_bounds()
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Keep `source_id` used + the parse path exercised off macOS.
        let _ = parse_display_id(source_id);
        main_display_bounds()
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use objc2_core_graphics::CGEvent;

    use super::normalize_cursor_to_frame;

    /// Polls the global cursor position on a background thread for the
    /// duration of a recording (ED.17). No Input-Monitoring permission:
    /// `CGEventCreate(NULL)` returns an event populated with the current
    /// pointer state, and `CGEventGetLocation` reads it.
    pub struct CursorPoller {
        stop: Arc<AtomicBool>,
        handle: Option<JoinHandle<Vec<(Duration, f32, f32)>>>,
    }

    impl CursorPoller {
        /// Start polling at ~60 Hz, normalizing each sample to `rect`
        /// (`(origin_x, origin_y, width, height)` in CG points).
        #[must_use]
        pub fn start(rect: (f64, f64, f64, f64)) -> Self {
            let stop = Arc::new(AtomicBool::new(false));
            let stop_thread = Arc::clone(&stop);
            let handle = std::thread::spawn(move || {
                let mut samples: Vec<(Duration, f32, f32)> = Vec::new();
                let start = Instant::now();
                while !stop_thread.load(Ordering::Relaxed) {
                    // `CGEvent::new(None)` == CGEventCreate(NULL): an event
                    // carrying the current mouse location (no permission).
                    if let Some(event) = CGEvent::new(None) {
                        let p = CGEvent::location(Some(&event));
                        let (x, y) = normalize_cursor_to_frame((p.x, p.y), rect);
                        samples.push((start.elapsed(), x, y));
                    }
                    std::thread::sleep(Duration::from_millis(16));
                }
                samples
            });
            Self {
                stop,
                handle: Some(handle),
            }
        }

        /// Stop polling and return the timestamped samples (sorted by time).
        /// Feed them to [`super::samples_to_track`] to get the project track.
        #[must_use]
        pub fn stop(mut self) -> Vec<(Duration, f32, f32)> {
            self.stop.store(true, Ordering::Relaxed);
            self.handle
                .take()
                .and_then(|h| h.join().ok())
                .unwrap_or_default()
        }
    }

    impl Drop for CursorPoller {
        fn drop(&mut self) {
            // If `stop()` wasn't called, signal + join so the thread doesn't
            // outlive the recording (mirrors the capture workers' drop).
            self.stop.store(true, Ordering::Relaxed);
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use std::time::Duration;

    /// Non-macOS stub: cursor capture is macOS-first (ED.17). The editor
    /// simply gets no cursor track on other platforms.
    pub struct CursorPoller;

    impl CursorPoller {
        /// No-op start — no cursor capture off macOS.
        #[must_use]
        pub fn start(_rect: (f64, f64, f64, f64)) -> Self {
            Self
        }

        /// No-op stop — always an empty track off macOS.
        #[must_use]
        pub fn stop(self) -> Vec<(Duration, f32, f32)> {
            Vec::new()
        }
    }
}

pub use imp::CursorPoller;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_maps_rect_to_unit_square() {
        // 1920×1080 display at origin (0,0).
        let rect = (0.0, 0.0, 1920.0, 1080.0);
        let (cx, cy) = normalize_cursor_to_frame((960.0, 540.0), rect);
        assert!((cx - 0.5).abs() < 1e-4 && (cy - 0.5).abs() < 1e-4, "centre");
        let (tx, ty) = normalize_cursor_to_frame((0.0, 0.0), rect);
        assert!(tx.abs() < 1e-4 && ty.abs() < 1e-4, "top-left origin");
        let (bx, by) = normalize_cursor_to_frame((1920.0, 1080.0), rect);
        assert!(
            (bx - 1.0).abs() < 1e-4 && (by - 1.0).abs() < 1e-4,
            "bottom-right"
        );
    }

    #[test]
    fn normalize_is_relative_to_a_non_zero_origin() {
        // A secondary display at origin (1920, 0).
        let rect = (1920.0, 0.0, 1280.0, 720.0);
        let (cx, cy) = normalize_cursor_to_frame((1920.0 + 640.0, 360.0), rect);
        assert!((cx - 0.5).abs() < 1e-4 && (cy - 0.5).abs() < 1e-4);
    }

    #[test]
    fn normalize_clamps_outside_and_survives_zero_size() {
        let rect = (0.0, 0.0, 100.0, 100.0);
        let (lx, ly) = normalize_cursor_to_frame((-50.0, 250.0), rect);
        assert!(
            lx.abs() < 1e-6 && (ly - 1.0).abs() < 1e-6,
            "clamped to edges"
        );
        // Zero-size axis maps to 0, never NaN/inf.
        let (zx, zy) = normalize_cursor_to_frame((10.0, 10.0), (0.0, 0.0, 0.0, 100.0));
        assert!(zx.abs() < 1e-6 && (zy - 0.1).abs() < 1e-6);
    }

    #[test]
    fn samples_to_track_resamples_onto_the_frame_grid() {
        // 30 fps → 1 frame per 1/30 s. Two samples in frame 0, one in frame 1.
        let samples = [
            (Duration::from_millis(0), 0.1, 0.1),
            (Duration::from_millis(10), 0.2, 0.2), // still frame 0 (< 33ms)
            (Duration::from_millis(40), 0.5, 0.6), // frame 1
        ];
        let track = samples_to_track(&samples, 30);
        assert_eq!(track.len(), 2, "two distinct frames");
        assert_eq!(track[0].frame, 0);
        // Latest sample at frame 0 wins.
        assert!((track[0].x - 0.2).abs() < 1e-6 && (track[0].y - 0.2).abs() < 1e-6);
        assert_eq!(track[1].frame, 1);
        assert!((track[1].x - 0.5).abs() < 1e-6);
    }

    #[test]
    fn samples_to_track_empty_is_empty() {
        assert!(samples_to_track(&[], 30).is_empty());
    }

    #[test]
    fn parse_display_id_handles_the_source_id_forms() {
        // ISS-17: "display-<CGDirectDisplayID>" → the id; everything else →
        // None (so the caller falls back to the main display).
        assert_eq!(parse_display_id(Some("display-69733382")), Some(69_733_382));
        assert_eq!(parse_display_id(Some("display-1")), Some(1));
        assert_eq!(parse_display_id(None), None, "primary display");
        assert_eq!(parse_display_id(Some("")), None);
        assert_eq!(parse_display_id(Some("window-42")), None, "window source");
        assert_eq!(parse_display_id(Some("display-")), None, "malformed");
        assert_eq!(parse_display_id(Some("display-abc")), None, "non-numeric");
    }
}
