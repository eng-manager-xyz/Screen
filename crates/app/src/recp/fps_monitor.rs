//! M-RECP.2 / AUT-263 — Frame-rate budget instrumentation.
//!
//! `FrameRateMonitor` keeps a sliding window of frame timestamps and
//! emits a `tracing::warn!` when the sustained fps drops below a
//! threshold. Hysteresis (separate low / high thresholds) prevents
//! log spam when fps oscillates near the boundary.
//!
//! Wired into M-CAM.3's frame pipeline once the pipeline lands.

use std::collections::VecDeque;
use std::time::Duration;

/// Default warn threshold — emit when sustained fps drops below 24.
pub const WARN_THRESHOLD_FPS: f64 = 24.0;

/// Default recovery threshold — clear the warn state when fps returns
/// above 26 (hysteresis prevents log spam).
pub const RECOVER_THRESHOLD_FPS: f64 = 26.0;

/// Sliding-window frame-rate monitor.
///
/// Constructed once per preview pipeline; `observe` is called every
/// frame with the current wall-clock timestamp. The internal buffer
/// caps at `capacity` to keep memory bounded.
#[derive(Debug)]
pub struct FrameRateMonitor {
    capacity: usize,
    window: VecDeque<Duration>,
    warn_threshold: f64,
    recover_threshold: f64,
    /// `true` when we've already emitted the low-fps warning and
    /// haven't recovered yet. Suppresses repeat warns.
    in_warn_state: bool,
}

impl Default for FrameRateMonitor {
    fn default() -> Self {
        // 150 frames = 5 seconds at 30 fps.
        Self::new(150, WARN_THRESHOLD_FPS, RECOVER_THRESHOLD_FPS)
    }
}

impl FrameRateMonitor {
    /// Build a monitor with the given window size + thresholds.
    #[must_use]
    pub fn new(capacity: usize, warn_threshold: f64, recover_threshold: f64) -> Self {
        Self {
            capacity,
            window: VecDeque::with_capacity(capacity),
            warn_threshold,
            recover_threshold,
            in_warn_state: false,
        }
    }

    /// Record a frame timestamp + return `Some(fps)` when the window
    /// crosses a threshold and the caller should emit a warn/recover
    /// log line. `None` for the common case where no transition
    /// happened.
    pub fn observe(&mut self, timestamp: Duration) -> Option<Transition> {
        if self.window.len() == self.capacity {
            self.window.pop_front();
        }
        self.window.push_back(timestamp);
        if self.window.len() < 2 {
            return None;
        }
        let fps = self.current_fps();
        if !self.in_warn_state && fps < self.warn_threshold {
            self.in_warn_state = true;
            Some(Transition::DroppedBelow(fps))
        } else if self.in_warn_state && fps >= self.recover_threshold {
            self.in_warn_state = false;
            Some(Transition::Recovered(fps))
        } else {
            None
        }
    }

    /// Snapshot the current sustained fps over the window.
    #[must_use]
    pub fn current_fps(&self) -> f64 {
        if self.window.len() < 2 {
            return 0.0;
        }
        let first = self.window.front().copied().unwrap_or_default();
        let last = self.window.back().copied().unwrap_or_default();
        let elapsed = last.saturating_sub(first).as_secs_f64();
        if elapsed <= 0.0 {
            return 0.0;
        }
        #[allow(
            clippy::cast_precision_loss,
            reason = "window length is well under 2^53; lossy conversion is fine"
        )]
        let frames = (self.window.len() - 1) as f64;
        frames / elapsed
    }
}

/// Threshold-crossing event from [`FrameRateMonitor::observe`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Transition {
    /// Sustained fps dropped below the warn threshold. Caller should
    /// emit `tracing::warn!`. `0.0` is the measured fps.
    DroppedBelow(f64),
    /// Sustained fps recovered above the recover threshold. Caller
    /// can emit an `info` log clearing the warning.
    Recovered(f64),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    #[test]
    fn default_window_is_five_seconds_at_30_fps() {
        let m = FrameRateMonitor::default();
        assert_eq!(m.capacity, 150);
    }

    #[test]
    fn single_frame_has_no_fps() {
        let mut m = FrameRateMonitor::default();
        assert_eq!(m.observe(ts(0)), None);
        assert!(m.current_fps().abs() < 1e-6);
    }

    #[test]
    fn thirty_fps_steady_state_no_transition() {
        let mut m = FrameRateMonitor::default();
        for i in 0..60 {
            // 33 ms per frame ≈ 30 fps
            let t = u64::try_from(i).unwrap() * 33;
            assert_eq!(m.observe(ts(t)), None);
        }
        let fps = m.current_fps();
        assert!((fps - 30.0).abs() < 1.0, "got {fps}");
    }

    #[test]
    fn fps_drop_below_threshold_emits_transition() {
        let mut m = FrameRateMonitor::default();
        // 100 ms per frame = 10 fps.
        let mut transition = None;
        for i in 0..30 {
            let t = u64::try_from(i).unwrap() * 100;
            if let Some(tr) = m.observe(ts(t)) {
                transition = Some(tr);
            }
        }
        assert!(
            matches!(transition, Some(Transition::DroppedBelow(_))),
            "expected DroppedBelow, got {transition:?}"
        );
    }

    #[test]
    fn recovery_emits_transition_only_after_warn() {
        // Use a small window so old low-fps frames evict quickly +
        // the recovery transition fires inside the test loop.
        let mut m = FrameRateMonitor::new(30, WARN_THRESHOLD_FPS, RECOVER_THRESHOLD_FPS);
        // Step 1: 30 frames at 100 ms apart = 10 fps. Fills the
        // capacity-30 window entirely.
        for i in 0..30 {
            let t = u64::try_from(i).unwrap() * 100;
            m.observe(ts(t));
        }
        assert!(m.in_warn_state);

        // Step 2: 200 frames at 16 ms apart starting after the drop
        // ends — enough to fully push out the slow frames + dominate
        // the window at high fps.
        let mut recovered = None;
        for i in 0..200 {
            let t = 3000 + u64::try_from(i).unwrap() * 16;
            if let Some(tr) = m.observe(ts(t)) {
                recovered = Some(tr);
            }
        }
        assert!(
            matches!(recovered, Some(Transition::Recovered(_))),
            "expected Recovered, got {recovered:?}; final fps = {}",
            m.current_fps()
        );
    }

    #[test]
    fn hysteresis_prevents_repeat_warns() {
        let mut m = FrameRateMonitor::default();
        // Drop to low fps.
        let mut warn_count = 0;
        for i in 0..200 {
            let t = u64::try_from(i).unwrap() * 100;
            if let Some(Transition::DroppedBelow(_)) = m.observe(ts(t)) {
                warn_count += 1;
            }
        }
        // Even after 200 frames at 10 fps, we should have warned only
        // once — the second warn requires a recovery in between.
        assert_eq!(warn_count, 1);
    }
}
