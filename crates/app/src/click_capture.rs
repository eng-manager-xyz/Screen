//! Click telemetry capture (ED.17 / ISS-16 / M-EDIT).
//!
//! Records *where the user clicked* during a recording so the editor can drive
//! the already-tested auto-zoom generator
//! ([`auto_zoom_segments`](edit::telemetry::auto_zoom_segments)) and the ED.19
//! click ripples. It is the companion to [`cursor_capture`](crate::cursor_capture):
//! that module captures the cursor *position* track with no permission; this
//! one captures the *click* log, which needs more.
//!
//! ## Why a tap, and why it's runtime-only
//!
//! Unlike the cursor position (readable with `CGEventCreate(NULL)`), clicks
//! must be observed through a **`CGEventTap`** — a listen-only system event
//! tap for left/right mouse-down. A tap requires the **Input-Monitoring**
//! permission (the OS prompts the user; it cannot be granted in CI/headless)
//! and a **`CFRunLoop`** to pump the tap's mach-port source. So the live tap is
//! *runtime-only*: there is no automated test for the capture itself. What
//! *is* exhaustively tested is the pure arithmetic either side of it —
//! [`samples_to_clicks`] (timestamp → project-frame mapping) and the
//! `RecordingState` handoff — plus the non-macOS stub, so the whole module
//! compiles + gate-greens on every OS.
//!
//! ## Graceful degradation
//!
//! If the tap can't be created (permission not yet granted) the worker logs
//! and exits cleanly — the recording proceeds, the editor simply gets no
//! click log (auto-zoom stays available as a manual tool). Capture never
//! blocks or fails a recording.

use std::time::Duration;

use edit::ClickEvent;

/// Resample timestamped clicks (`(elapsed_since_start, x, y)`, normalized to
/// the captured frame, sorted by time) onto the project frame grid: each
/// click's frame is `floor(elapsed_secs · project_fps)`. Unlike the cursor
/// track, every click is kept (two clicks one frame apart are two real events
/// — the auto-zoom clusterer merges them by time itself). Pure.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "elapsed·fps is a non-negative frame index well under 2^52; the f64→u64 cast is floor of a clamped-non-negative value"
)]
pub fn samples_to_clicks(samples: &[(Duration, f32, f32)], project_fps: u32) -> Vec<ClickEvent> {
    let fps = f64::from(project_fps.max(1));
    samples
        .iter()
        .map(|&(t, x, y)| {
            let frame = (t.as_secs_f64() * fps).max(0.0) as u64;
            ClickEvent::new(frame, x, y)
        })
        .collect()
}

#[cfg(target_os = "macos")]
mod imp {
    #![allow(
        unsafe_code,
        reason = "CGEventTap + CFRunLoop are C FFI: CGEventTapCreate is unsafe (takes a raw callback + user_info), the tap callback is `extern \"C-unwind\"`, and unblocking the worker reads the run loop through a raw pointer. Each unsafe site documents its own SAFETY invariant. The wider `unsafe_code = warn` is workspace-wide; this FFI module scopes the justification."
    )]

    use std::ffi::c_void;
    use std::ptr::NonNull;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use objc2_core_foundation::{CFMachPort, CFRunLoop, kCFRunLoopCommonModes};
    use objc2_core_graphics::{
        CGEvent, CGEventMask, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
        CGEventTapProxy, CGEventType,
    };

    use crate::cursor_capture::normalize_cursor_to_frame;

    /// State the C tap callback writes into. Lives behind an [`Arc`] held by
    /// both [`ClickTap`] and its worker thread; the raw pointer handed to
    /// `CGEventTapCreate` borrows it (the worker's `Arc` clone keeps it alive
    /// for the whole run-loop lifetime).
    struct Shared {
        /// Captured-display rect (CG points) clicks are normalized against.
        rect: (f64, f64, f64, f64),
        /// Recording start, for the per-click elapsed timestamp.
        start: Instant,
        /// Accumulated `(elapsed, x, y)` clicks, drained at stop.
        clicks: Mutex<Vec<(Duration, f32, f32)>>,
    }

    /// The C event-tap callback: on a left/right mouse-down, record the
    /// normalized click position + elapsed time, then pass the event through
    /// unchanged (listen-only never mutates the stream).
    ///
    /// # Safety
    ///
    /// `user_info` is the `Shared` pointer passed to `CGEventTapCreate`; it is
    /// valid for the run loop's lifetime (the worker holds an `Arc`). `event`
    /// is a live `CGEvent` for the duration of the call.
    unsafe extern "C-unwind" fn tap_callback(
        _proxy: CGEventTapProxy,
        etype: CGEventType,
        event: NonNull<CGEvent>,
        user_info: *mut c_void,
    ) -> *mut CGEvent {
        if !user_info.is_null()
            && (etype == CGEventType::LeftMouseDown || etype == CGEventType::RightMouseDown)
        {
            // SAFETY: `user_info` is the `Arc<Shared>` raw pointer, alive for
            // the run loop. We borrow, never drop, it.
            let shared = unsafe { &*(user_info.cast::<Shared>()) };
            let ev = unsafe { event.as_ref() };
            let p = CGEvent::location(Some(ev));
            let (x, y) = normalize_cursor_to_frame((p.x, p.y), shared.rect);
            if let Ok(mut clicks) = shared.clicks.lock() {
                clicks.push((shared.start.elapsed(), x, y));
            }
        }
        event.as_ptr()
    }

    /// Captures mouse-down clicks through a listen-only `CGEventTap` for the
    /// duration of a recording (ED.17 / ISS-16). The tap's mach-port source is
    /// pumped on a dedicated worker thread's `CFRunLoop`; [`Self::stop`] stops
    /// that run loop and drains the clicks.
    pub struct ClickTap {
        shared: Arc<Shared>,
        /// The worker's `CFRunLoop` pointer, as a `usize` address (`0` until
        /// published) — `stop()` calls the thread-safe `CFRunLoopStop` on it
        /// to unblock the worker. A pointer (not the `CFRetained`, which is
        /// `!Send`) keeps `ClickTap` — and thus the shared `RecordingState` —
        /// `Send + Sync`; the run loop stays alive on the worker thread for
        /// the whole capture, so the address is valid until we join.
        runloop_ptr: Arc<AtomicUsize>,
        handle: Option<JoinHandle<()>>,
    }

    impl ClickTap {
        /// Start capturing, normalizing each click to `rect` (`(origin_x,
        /// origin_y, width, height)` in CG points — the captured display).
        ///
        /// Spawns the run-loop worker. If the tap can't be created (no
        /// Input-Monitoring permission), the worker logs + exits and capture
        /// degrades to an empty log; the recording is unaffected.
        #[must_use]
        pub fn start(rect: (f64, f64, f64, f64)) -> Self {
            let shared = Arc::new(Shared {
                rect,
                start: Instant::now(),
                clicks: Mutex::new(Vec::new()),
            });
            let runloop_ptr = Arc::new(AtomicUsize::new(0));
            let shared_thread = Arc::clone(&shared);
            let runloop_ptr_thread = Arc::clone(&runloop_ptr);

            let handle = std::thread::spawn(move || {
                // The Arc clone keeps `Shared` alive while the callback may
                // fire; the raw pointer borrows it.
                let user_info = Arc::as_ptr(&shared_thread).cast_mut().cast::<c_void>();
                // Mask bit per event type is `1 << type`.
                let mask: CGEventMask = (1u64 << CGEventType::LeftMouseDown.0)
                    | (1u64 << CGEventType::RightMouseDown.0);

                // SAFETY: a correct callback + a valid (borrowed) user_info.
                let tap = unsafe {
                    CGEvent::tap_create(
                        CGEventTapLocation::SessionEventTap,
                        CGEventTapPlacement::HeadInsertEventTap,
                        CGEventTapOptions::ListenOnly,
                        mask,
                        Some(tap_callback),
                        user_info,
                    )
                };
                let Some(tap) = tap else {
                    tracing::warn!(
                        "click tap unavailable (Input-Monitoring permission not granted?); \
                         recording proceeds without a click log"
                    );
                    return;
                };
                let Some(source) = CFMachPort::new_run_loop_source(None, Some(&tap), 0) else {
                    tracing::warn!("CFMachPort run-loop source creation failed; no click log");
                    return;
                };
                let Some(rl) = CFRunLoop::current() else {
                    return;
                };
                // SAFETY: `kCFRunLoopCommonModes` is a CF constant string.
                let mode = unsafe { kCFRunLoopCommonModes };
                rl.add_source(Some(&source), mode);
                CGEvent::tap_enable(&tap, true);
                // Publish the run loop's address so `stop()` can unblock us.
                // `rl` (the `CFRetained`) stays on this thread's stack — and CF
                // owns the per-thread run loop for the thread's lifetime — so
                // the pointer is valid until we join.
                let rl_ptr = (&raw const *rl) as usize;
                runloop_ptr_thread.store(rl_ptr, Ordering::SeqCst);
                CFRunLoop::run();
                // Keep the tap + source + run loop alive until the loop returns.
                drop(source);
                drop(tap);
                drop(rl);
            });

            Self {
                shared,
                runloop_ptr,
                handle: Some(handle),
            }
        }

        /// Stop capturing and return the timestamped clicks (in time order).
        /// Feed them to [`super::samples_to_clicks`] for the project log.
        #[must_use]
        pub fn stop(mut self) -> Vec<(Duration, f32, f32)> {
            self.stop_runloop();
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
            std::mem::take(
                &mut *self
                    .shared
                    .clicks
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            )
        }

        /// Stop the worker's run loop, waiting briefly for it to be published
        /// if `stop` raced an only-just-started worker (bounded so a worker
        /// that exited early — no permission — can never hang the caller).
        fn stop_runloop(&self) {
            let mut waited = Duration::ZERO;
            let step = Duration::from_millis(5);
            loop {
                // Take the address exactly once (swap to 0) so Drop after
                // `stop()` is a no-op.
                let addr = self.runloop_ptr.swap(0, Ordering::SeqCst);
                if addr != 0 {
                    // SAFETY: the worker holds the run loop alive (blocked in
                    // `CFRunLoop::run`) until this `CFRunLoopStop` returns it;
                    // `CFRunLoopStop` is thread-safe.
                    let rl = unsafe { &*(addr as *const CFRunLoop) };
                    rl.stop();
                    return;
                }
                // Worker finished (early-return) → nothing to stop.
                if self.handle.as_ref().is_none_or(JoinHandle::is_finished) {
                    return;
                }
                if waited >= Duration::from_secs(2) {
                    return;
                }
                std::thread::sleep(step);
                waited += step;
            }
        }
    }

    impl Drop for ClickTap {
        fn drop(&mut self) {
            // If `stop()` already ran, handle is None and this is a no-op;
            // otherwise stop the loop + join so the worker can't outlive us.
            self.stop_runloop();
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use std::time::Duration;

    /// Non-macOS stub: click capture is macOS-first (ED.17 / ISS-16). The
    /// editor simply gets no click log on other platforms.
    pub struct ClickTap;

    impl ClickTap {
        /// No-op start — no click capture off macOS.
        #[must_use]
        pub fn start(_rect: (f64, f64, f64, f64)) -> Self {
            Self
        }

        /// No-op stop — always an empty click log off macOS.
        #[must_use]
        pub fn stop(self) -> Vec<(Duration, f32, f32)> {
            Vec::new()
        }
    }
}

pub use imp::ClickTap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_to_clicks_maps_each_click_to_its_frame() {
        // 30 fps → 1 frame per 1/30 s.
        let samples = [
            (Duration::from_millis(0), 0.1, 0.2),
            (Duration::from_millis(40), 0.5, 0.6),   // frame 1
            (Duration::from_millis(1000), 0.9, 0.9), // frame 30
        ];
        let clicks = samples_to_clicks(&samples, 30);
        assert_eq!(clicks.len(), 3, "every click is kept");
        assert_eq!(clicks[0].frame, 0);
        assert!((clicks[0].x - 0.1).abs() < 1e-6 && (clicks[0].y - 0.2).abs() < 1e-6);
        assert_eq!(clicks[1].frame, 1);
        assert_eq!(clicks[2].frame, 30);
    }

    #[test]
    fn samples_to_clicks_keeps_two_clicks_in_one_frame() {
        // Two distinct clicks both inside frame 0 stay as two events (the
        // auto-zoom clusterer, not this resampler, merges by time).
        let samples = [
            (Duration::from_millis(2), 0.2, 0.2),
            (Duration::from_millis(8), 0.8, 0.8),
        ];
        let clicks = samples_to_clicks(&samples, 30);
        assert_eq!(clicks.len(), 2);
        assert_eq!(clicks[0].frame, 0);
        assert_eq!(clicks[1].frame, 0);
    }

    #[test]
    fn samples_to_clicks_empty_is_empty() {
        assert!(samples_to_clicks(&[], 30).is_empty());
    }

    #[test]
    fn samples_to_clicks_feeds_auto_zoom() {
        // End-to-end shape check: captured clicks → ClickEvents → the existing
        // auto-zoom generator produces a zoom (proves the wiring contract).
        let samples = [(Duration::from_millis(1000), 0.4, 0.6)];
        let clicks = samples_to_clicks(&samples, 30);
        let zooms = edit::telemetry::auto_zoom_segments(
            &clicks,
            30,
            &edit::style::AutoZoomConfig::default(),
        );
        assert_eq!(zooms.len(), 1, "one click → one auto-zoom region");
    }
}
