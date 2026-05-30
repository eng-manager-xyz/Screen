//! [`EditorPlayer`] — the editor's frame-indexed, variable-rate playback
//! clock (ED.4 / M-EDIT).
//!
//! The recorder's [`Player`](crate::Player) is wall-clock paced and 1×
//! only. The editor needs a clock that is the single authority over time:
//! it must seek to an exact frame, step one frame at a time, play at a
//! chosen rate, honour in/out points, and loop. Crucially it is built on
//! [`wisp_animation::Driver`] — the *same* clock the zoom animation engine
//! (ED.16) samples — so the playhead and the cinematic zoom Tracks advance
//! in lockstep in both realtime preview and deterministic export.
//!
//! `EditorPlayer` is a pure clock: it computes `current_frame()` but does
//! not own the decoder. The preview (ED.6) and export (ED.20) read
//! `current_frame()` and pull that frame from
//! [`decode::EditorVideoStream`](decode::editor_stream::EditorVideoStream).

use std::time::Duration;

use wisp_animation::Driver;

/// A frame-indexed playback clock over a fixed-length project.
///
/// Time is measured in **project frames** at [`fps`](Self::fps). The
/// playhead is constrained to `[in_frame, out_frame)`; out of those, in/out
/// points scope playback + export, and looping wraps back to `in_frame`.
#[derive(Clone, Debug)]
pub struct EditorPlayer {
    driver: Driver,
    fps: u32,
    duration_frames: u64,
    in_frame: u64,
    /// Exclusive end of the playable range (defaults to `duration_frames`).
    out_frame: u64,
    looping: bool,
}

impl EditorPlayer {
    /// A realtime player for a project of `duration_frames` at `fps`.
    /// Starts paused at frame 0 with no in/out trim and looping off.
    #[must_use]
    pub fn new(fps: u32, duration_frames: u64) -> Self {
        Self::with_driver(Driver::realtime(), fps, duration_frames)
    }

    /// A fixed-step player (one frame per [`tick`](Self::tick) at rate 1×)
    /// for deterministic, reproducible stepping — the export clock.
    #[must_use]
    pub fn fixed(fps: u32, duration_frames: u64) -> Self {
        let fps = fps.max(1);
        let dt = Duration::from_secs_f64(1.0 / f64::from(fps));
        Self::with_driver(Driver::fixed(dt), fps, duration_frames)
    }

    fn with_driver(driver: Driver, fps: u32, duration_frames: u64) -> Self {
        let fps = fps.max(1);
        Self {
            driver,
            fps,
            duration_frames,
            in_frame: 0,
            out_frame: duration_frames,
            looping: false,
        }
    }

    // ── Transport ────────────────────────────────────────────────────

    /// Begin advancing. If the playhead is parked at the end of a
    /// non-looping range, restart from the in-point first.
    pub fn play(&mut self) {
        if !self.looping && self.current_frame() >= self.last_frame() {
            self.driver.seek(self.frame_to_elapsed(self.in_frame));
        }
        self.driver.play();
    }

    /// Pause advancing.
    pub fn pause(&mut self) {
        self.driver.pause();
    }

    /// Toggle play/pause.
    pub fn toggle_play(&mut self) {
        if self.is_playing() {
            self.pause();
        } else {
            self.play();
        }
    }

    /// Whether the clock is currently advancing.
    #[must_use]
    pub const fn is_playing(&self) -> bool {
        self.driver.is_playing()
    }

    /// Seek to an exact frame (clamped to the playable range).
    pub fn seek(&mut self, frame: u64) {
        let clamped = frame.clamp(self.in_frame, self.last_frame());
        self.driver.seek(self.frame_to_elapsed(clamped));
    }

    /// Step `delta` frames (negative = back) and pause. Clamps at the
    /// range boundaries.
    pub fn step(&mut self, delta: i64) {
        self.driver.pause();
        self.seek(self.current_frame().saturating_add_signed(delta));
    }

    /// Set the playback rate (`1.0` = realtime, `2.0` = 2×, `0.5` = half).
    /// Negative rates clamp to 0 (reverse playback is a future ticket).
    pub fn set_rate(&mut self, rate: f32) {
        self.driver.set_time_scale(rate);
    }

    /// Current playback rate.
    #[must_use]
    pub const fn rate(&self) -> f32 {
        self.driver.time_scale()
    }

    /// Advance the clock by `dt` (realtime mode) or one fixed step (fixed
    /// mode). At the end of the range: loop back to the in-point if
    /// [`looping`](Self::looping), otherwise clamp to the last frame and
    /// pause. No-op while paused.
    pub fn tick(&mut self, dt: Duration) {
        if !self.driver.is_playing() {
            return;
        }
        self.driver.tick(dt);
        if self.raw_frame() >= self.out_frame {
            if self.looping {
                self.driver.seek(self.frame_to_elapsed(self.in_frame));
            } else {
                self.driver.seek(self.frame_to_elapsed(self.last_frame()));
                self.driver.pause();
            }
        }
    }

    // ── In / out + loop ──────────────────────────────────────────────

    /// Set the in/out points (order-independent). The out-point is the
    /// exclusive end; the range is clamped to the project and forced
    /// non-empty. The playhead is re-clamped into the new range.
    pub fn set_in_out(&mut self, a: u64, b: u64) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        self.in_frame = lo.min(self.duration_frames.saturating_sub(1));
        self.out_frame = hi
            .max(self.in_frame + 1)
            .min(self.duration_frames.max(self.in_frame + 1));
        let reclamped = self.current_frame();
        self.driver.seek(self.frame_to_elapsed(reclamped));
    }

    /// Clear in/out points back to the full project range.
    pub fn clear_in_out(&mut self) {
        self.in_frame = 0;
        self.out_frame = self.duration_frames;
    }

    /// In-point (inclusive).
    #[must_use]
    pub const fn in_frame(&self) -> u64 {
        self.in_frame
    }

    /// Out-point (exclusive).
    #[must_use]
    pub const fn out_frame(&self) -> u64 {
        self.out_frame
    }

    /// Enable / disable looping over the in/out range.
    pub const fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    /// Whether looping is enabled.
    #[must_use]
    pub const fn looping(&self) -> bool {
        self.looping
    }

    // ── Queries ──────────────────────────────────────────────────────

    /// The current playhead frame (clamped to the playable range).
    #[must_use]
    pub fn current_frame(&self) -> u64 {
        self.raw_frame().clamp(self.in_frame, self.last_frame())
    }

    /// Project frame rate.
    #[must_use]
    pub const fn fps(&self) -> u32 {
        self.fps
    }

    /// Total project length in frames.
    #[must_use]
    pub const fn duration_frames(&self) -> u64 {
        self.duration_frames
    }

    /// Borrow the underlying [`Driver`] — e.g. for the zoom engine (ED.16)
    /// to sample animation Tracks against the same clock as the playhead.
    #[must_use]
    pub const fn driver(&self) -> &Driver {
        &self.driver
    }

    /// Normalised progress through the in/out range, `0.0..=1.0`.
    #[must_use]
    pub fn progress(&self) -> f32 {
        let span = self.out_frame.saturating_sub(self.in_frame);
        if span == 0 {
            return 0.0;
        }
        let into = self.current_frame().saturating_sub(self.in_frame);
        #[allow(
            clippy::cast_precision_loss,
            reason = "frame counts are well under 2^24; f32 is plenty for a 0..1 scrubber fraction"
        )]
        let p = into as f32 / span as f32;
        p.clamp(0.0, 1.0)
    }

    // ── Internals ────────────────────────────────────────────────────

    /// Last playable frame index (`out_frame - 1`, never below `in_frame`).
    fn last_frame(&self) -> u64 {
        self.out_frame.saturating_sub(1).max(self.in_frame)
    }

    /// Frame implied by the driver's elapsed time, before range clamping.
    fn raw_frame(&self) -> u64 {
        // +epsilon so a seek to k/fps that floats to k - 1e-12 still floors
        // to k rather than k-1.
        let raw = self.driver.elapsed().as_secs_f64() * f64::from(self.fps) + 1e-6;
        if raw <= 0.0 {
            return 0;
        }
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "raw is positive and finite (elapsed * fps); frame counts fit u64"
        )]
        let frame = raw.floor() as u64;
        frame
    }

    fn frame_to_elapsed(&self, frame: u64) -> Duration {
        #[allow(
            clippy::cast_precision_loss,
            reason = "frame counts are well under 2^52; u64→f64 is lossless at these magnitudes"
        )]
        let secs = frame as f64 / f64::from(self.fps);
        Duration::from_secs_f64(secs.max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FPS: u32 = 30;

    #[test]
    fn starts_paused_at_zero() {
        let p = EditorPlayer::new(FPS, 1000);
        assert!(!p.is_playing());
        assert_eq!(p.current_frame(), 0);
        assert!((p.rate() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn play_and_tick_advances_by_dt_times_fps() {
        let mut p = EditorPlayer::new(FPS, 1000);
        p.play();
        p.tick(Duration::from_secs(1)); // 1 s @ 30 fps = 30 frames
        assert_eq!(p.current_frame(), 30);
    }

    #[test]
    fn rate_scales_advance() {
        let mut p = EditorPlayer::new(FPS, 1000);
        p.set_rate(2.0);
        p.play();
        p.tick(Duration::from_secs(1)); // 2× → 60 frames
        assert_eq!(p.current_frame(), 60);

        let mut half = EditorPlayer::new(FPS, 1000);
        half.set_rate(0.5);
        half.play();
        half.tick(Duration::from_secs(1)); // 0.5× → 15 frames
        assert_eq!(half.current_frame(), 15);
    }

    #[test]
    fn pause_holds_the_playhead() {
        let mut p = EditorPlayer::new(FPS, 1000);
        p.play();
        p.tick(Duration::from_secs(1));
        p.pause();
        p.tick(Duration::from_secs(5));
        assert_eq!(p.current_frame(), 30);
    }

    #[test]
    fn seek_is_exact() {
        let mut p = EditorPlayer::new(FPS, 1000);
        for target in [0, 1, 7, 99, 100, 333, 999] {
            p.seek(target);
            assert_eq!(p.current_frame(), target, "seek({target})");
        }
    }

    #[test]
    fn step_moves_one_frame_and_pauses() {
        let mut p = EditorPlayer::new(FPS, 1000);
        p.play();
        p.seek(100);
        p.step(1);
        assert_eq!(p.current_frame(), 101);
        assert!(!p.is_playing(), "stepping pauses");
        p.step(-1);
        assert_eq!(p.current_frame(), 100);
        p.step(-1000); // clamps at 0
        assert_eq!(p.current_frame(), 0);
    }

    #[test]
    fn in_out_clamps_the_playhead() {
        let mut p = EditorPlayer::new(FPS, 1000);
        p.set_in_out(10, 20); // playable [10, 20), last = 19
        assert_eq!(p.in_frame(), 10);
        assert_eq!(p.out_frame(), 20);
        p.seek(5);
        assert_eq!(p.current_frame(), 10, "below in clamps to in");
        p.seek(50);
        assert_eq!(p.current_frame(), 19, "above out clamps to last");
        // Order-independent.
        p.set_in_out(40, 30);
        assert_eq!(p.in_frame(), 30);
        assert_eq!(p.out_frame(), 40);
    }

    #[test]
    fn not_looping_clamps_and_pauses_at_end() {
        let mut p = EditorPlayer::new(FPS, 10); // 10 frames, out = 10
        p.play();
        p.tick(Duration::from_secs(1)); // would reach frame 30 → past end
        assert_eq!(p.current_frame(), 9, "clamped to last frame");
        assert!(!p.is_playing(), "paused at end");
    }

    #[test]
    fn looping_wraps_to_in_point() {
        let mut p = EditorPlayer::new(FPS, 1000);
        p.set_in_out(0, 10);
        p.set_looping(true);
        p.play();
        p.tick(Duration::from_secs(1)); // past out → wrap
        assert!(p.is_playing(), "loop keeps playing");
        assert!(
            p.current_frame() < 10,
            "wrapped back into [0,10), got {}",
            p.current_frame()
        );
    }

    #[test]
    fn play_from_end_restarts() {
        let mut p = EditorPlayer::new(FPS, 100);
        p.seek(99); // last frame
        p.play();
        // Restarted from the in-point rather than staying parked at the end.
        assert_eq!(p.current_frame(), 0);
        assert!(p.is_playing());
    }

    #[test]
    fn fixed_driver_steps_one_frame_per_tick() {
        let mut p = EditorPlayer::fixed(FPS, 100);
        p.play();
        for expected in 1..=5 {
            p.tick(Duration::ZERO); // fixed mode ignores the dt
            assert_eq!(p.current_frame(), expected);
        }
    }

    #[test]
    fn progress_spans_in_out() {
        let mut p = EditorPlayer::new(FPS, 100);
        p.set_in_out(0, 100);
        p.seek(0);
        assert!(p.progress() < 1e-6);
        p.seek(50);
        assert!((p.progress() - 0.5).abs() < 0.02, "got {}", p.progress());
        p.seek(99);
        assert!(p.progress() > 0.95);
    }
}
