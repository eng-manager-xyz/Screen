//! Shared timeline — `MediaTime`, `MediaDuration`, `MediaClock`,
//! `Timestamped<T>` (M-MEDIA.2 / AUT-98).
//!
//! Every audio chunk, video frame, cursor event, and visualization
//! window in the recorder is stamped against a single timeline. This
//! module is that timeline's vocabulary.
//!
//! # Why nanoseconds
//!
//! Internal representation is **`i64` nanoseconds** (signed so a
//! pre-origin offset is representable). `i64::MAX` nanoseconds is
//! ≈ 292 years — comfortable headroom for any recorder session.
//! `f64` seconds drops below 1 µs precision past ~10⁹ s; nanoseconds
//! stay exact through arithmetic.
//!
//! # Conversion helpers
//!
//! ```rust
//! use media::clock::MediaTime;
//!
//! // 30 fps, frame 90 → 3.0 s.
//! let t = MediaTime::from_frame(90, 30.0);
//! assert!((t.as_seconds() - 3.0).abs() < 1e-9);
//!
//! // 48 kHz, sample 48 000 → 1.0 s.
//! let t = MediaTime::from_sample(48_000, 48_000);
//! assert!((t.as_seconds() - 1.0).abs() < 1e-9);
//! ```
//!
//! # `MediaClock` modes
//!
//! - [`MediaClock::wall_clock`] — anchored to `Instant::now()`; production.
//! - [`MediaClock::manual`] — driven by explicit [`MediaClock::advance_by`];
//!   tests + headless examples that need byte-exact reproducibility.

use std::sync::Mutex;
use std::time::Instant;

/// One point on the media timeline. Resolution is nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaTime {
    nanos: i64,
}

impl MediaTime {
    /// Time zero — the timeline origin.
    pub const ZERO: Self = Self { nanos: 0 };

    /// Construct from raw nanoseconds since origin (can be negative).
    #[must_use]
    pub const fn from_nanos(nanos: i64) -> Self {
        Self { nanos }
    }

    /// Construct from seconds since origin (`f64`).
    #[must_use]
    pub fn from_seconds(s: f64) -> Self {
        Self {
            nanos: nanos_from_seconds(s),
        }
    }

    /// Construct from an audio sample index at a given sample rate.
    ///
    /// `MediaTime::from_sample(48_000, 48_000) == 1 s`.
    #[must_use]
    pub fn from_sample(index: u64, sample_rate: u32) -> Self {
        assert!(sample_rate > 0, "sample_rate must be > 0");
        // nanos = index * 1e9 / sample_rate, all in integer space to
        // avoid f64 precision loss for large sample counts.
        let numerator = u128::from(index) * 1_000_000_000_u128;
        let nanos = (numerator / u128::from(sample_rate))
            .try_into()
            .expect("sample timestamp overflows i64 nanoseconds");
        Self { nanos }
    }

    /// Construct from a video frame index at a given frame rate (fps).
    ///
    /// `MediaTime::from_frame(90, 30.0) == 3 s`.
    #[must_use]
    pub fn from_frame(index: u64, frame_rate: f64) -> Self {
        assert!(
            frame_rate.is_finite() && frame_rate > 0.0,
            "frame_rate must be finite and > 0"
        );
        Self::from_seconds(index_as_f64(index) / frame_rate)
    }

    /// Raw nanoseconds since origin.
    #[must_use]
    pub const fn as_nanos(self) -> i64 {
        self.nanos
    }

    /// Seconds since origin (`f64`).
    #[must_use]
    pub fn as_seconds(self) -> f64 {
        // i64 nanos → f64 seconds. Past ~9.2e15 ns (~106 days) the
        // f64 mantissa starts losing per-ns precision; arithmetic
        // should prefer nanoseconds for accuracy.
        #[expect(
            clippy::cast_precision_loss,
            reason = "f64 covers all reasonable session lengths; clients wanting exact nanos use as_nanos()"
        )]
        let secs = self.nanos as f64 / 1.0e9;
        secs
    }

    /// Audio sample index at a given rate, rounded to nearest.
    ///
    /// Rounding (vs truncation) keeps `from_sample` ↔ `to_sample`
    /// exact round-trip for any sample rate that doesn't divide 10⁹
    /// evenly (e.g. 44_100 Hz produces non-integer nanoseconds per
    /// sample; truncating would drift -1 every sample).
    ///
    /// `MediaTime::from_seconds(1.0).to_sample(48_000) == 48_000`.
    #[must_use]
    pub fn to_sample(self, sample_rate: u32) -> i64 {
        assert!(sample_rate > 0, "sample_rate must be > 0");
        // Round-half-toward-positive-infinity: `(x + N/2) / N` for
        // positive x. For negative x we mirror the sign on the offset
        // so round-trip stays exact symmetrically.
        let product = i128::from(self.nanos) * i128::from(sample_rate);
        let half: i128 = 500_000_000;
        let biased = if product >= 0 {
            product + half
        } else {
            product - half
        };
        (biased / 1_000_000_000_i128)
            .try_into()
            .expect("sample index overflows i64")
    }

    /// Video frame index at a given fps, rounded to nearest.
    ///
    /// Same round-trip rationale as [`Self::to_sample`].
    #[must_use]
    pub fn to_frame(self, frame_rate: f64) -> i64 {
        assert!(
            frame_rate.is_finite() && frame_rate > 0.0,
            "frame_rate must be finite and > 0"
        );
        let seconds = self.as_seconds();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "frame index from valid timestamp + fps fits in i64 for any reasonable session"
        )]
        let idx = (seconds * frame_rate).round() as i64;
        idx
    }
}

impl std::ops::Add<MediaDuration> for MediaTime {
    type Output = Self;
    fn add(self, rhs: MediaDuration) -> Self {
        Self {
            nanos: self.nanos.saturating_add(rhs.nanos),
        }
    }
}

impl std::ops::Sub<MediaDuration> for MediaTime {
    type Output = Self;
    fn sub(self, rhs: MediaDuration) -> Self {
        Self {
            nanos: self.nanos.saturating_sub(rhs.nanos),
        }
    }
}

impl std::ops::Sub<MediaTime> for MediaTime {
    type Output = MediaDuration;
    /// `b - a` returns the `MediaDuration` between two times.
    fn sub(self, rhs: MediaTime) -> MediaDuration {
        MediaDuration {
            nanos: self.nanos.saturating_sub(rhs.nanos),
        }
    }
}

/// An interval between two `MediaTime` values. Resolution is
/// nanoseconds. Can be negative (e.g., when measuring drift).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MediaDuration {
    nanos: i64,
}

impl MediaDuration {
    /// Zero duration.
    pub const ZERO: Self = Self { nanos: 0 };

    /// Construct from raw nanoseconds.
    #[must_use]
    pub const fn from_nanos(nanos: i64) -> Self {
        Self { nanos }
    }

    /// Construct from seconds.
    #[must_use]
    pub fn from_seconds(s: f64) -> Self {
        Self {
            nanos: nanos_from_seconds(s),
        }
    }

    /// Construct from milliseconds.
    #[must_use]
    pub const fn from_millis(ms: i64) -> Self {
        Self {
            nanos: ms.saturating_mul(1_000_000),
        }
    }

    /// Raw nanoseconds.
    #[must_use]
    pub const fn as_nanos(self) -> i64 {
        self.nanos
    }

    /// Seconds (`f64`).
    #[must_use]
    pub fn as_seconds(self) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "callers wanting exact nanos use as_nanos()"
        )]
        let secs = self.nanos as f64 / 1.0e9;
        secs
    }

    /// Absolute value — useful when measuring drift.
    #[must_use]
    pub const fn abs(self) -> Self {
        Self {
            nanos: self.nanos.abs(),
        }
    }
}

impl std::ops::Add<MediaDuration> for MediaDuration {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            nanos: self.nanos.saturating_add(rhs.nanos),
        }
    }
}

impl std::ops::Sub<MediaDuration> for MediaDuration {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            nanos: self.nanos.saturating_sub(rhs.nanos),
        }
    }
}

/// A value carried with its position on the media timeline.
///
/// Used wherever an audio chunk, video frame, or cursor event flows
/// through the system: `Timestamped<AudioChunk>`, `Timestamped<VideoFrame>`,
/// `Timestamped<CursorEvent>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timestamped<T> {
    /// When the value occurred on the media timeline.
    pub time: MediaTime,
    /// The carried value.
    pub value: T,
}

impl<T> Timestamped<T> {
    /// Construct a timestamped value.
    pub const fn new(time: MediaTime, value: T) -> Self {
        Self { time, value }
    }

    /// Borrow the inner value.
    pub const fn as_ref(&self) -> Timestamped<&T> {
        Timestamped {
            time: self.time,
            value: &self.value,
        }
    }
}

/// Authoritative timeline source. Capture pipelines stamp their
/// chunks/frames against a single shared `MediaClock` so audio, video,
/// cursor, and `wisp` overlays all agree on "now."
///
/// Two modes — pick at construction:
///
/// - [`MediaClock::wall_clock`] — anchored to `Instant::now()`. Used
///   by live capture and playback.
/// - [`MediaClock::manual`] — driven by [`MediaClock::advance_by`].
///   Used by tests, headless examples, and the synthetic sync harness
///   (M-MEDIA.7) so timestamps are byte-exact reproducible.
#[derive(Debug)]
pub struct MediaClock {
    inner: ClockInner,
}

#[derive(Debug)]
enum ClockInner {
    Wall { origin: Instant },
    Manual { current: Mutex<MediaTime> },
}

impl Default for MediaClock {
    fn default() -> Self {
        Self::wall_clock()
    }
}

impl MediaClock {
    /// Construct a wall-clock-backed `MediaClock`, anchored to *now*.
    /// Subsequent `now()` calls return `MediaTime` values relative to
    /// this anchor.
    #[must_use]
    pub fn wall_clock() -> Self {
        Self {
            inner: ClockInner::Wall {
                origin: Instant::now(),
            },
        }
    }

    /// Construct a manual clock starting at `start`. The clock only
    /// advances when [`Self::advance_by`] is called — useful for
    /// deterministic tests.
    #[must_use]
    pub fn manual(start: MediaTime) -> Self {
        Self {
            inner: ClockInner::Manual {
                current: Mutex::new(start),
            },
        }
    }

    /// Current timeline position.
    #[must_use]
    pub fn now(&self) -> MediaTime {
        match &self.inner {
            ClockInner::Wall { origin } => {
                let elapsed = origin.elapsed();
                let nanos = i64::try_from(elapsed.as_nanos())
                    .expect("wall-clock elapsed overflows i64 ns (session > 292y)");
                MediaTime::from_nanos(nanos)
            }
            ClockInner::Manual { current } => *current.lock().expect("clock poisoned"),
        }
    }

    /// Advance a manual clock by `duration`. No-op on a wall clock.
    pub fn advance_by(&self, duration: MediaDuration) {
        if let ClockInner::Manual { current } = &self.inner {
            let mut t = current.lock().expect("clock poisoned");
            *t = *t + duration;
        }
    }

    /// Attach the current timestamp to `value`.
    pub fn assign<T>(&self, value: T) -> Timestamped<T> {
        Timestamped::new(self.now(), value)
    }

    /// True for a manual clock; false for a wall clock. Used by tests
    /// and examples to gate assertions on byte-exact timestamps.
    #[must_use]
    pub const fn is_manual(&self) -> bool {
        matches!(self.inner, ClockInner::Manual { .. })
    }
}

fn nanos_from_seconds(s: f64) -> i64 {
    assert!(s.is_finite(), "seconds must be finite");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "i64 nanos covers ±292y — any realistic recorder session"
    )]
    let nanos = (s * 1.0e9).round() as i64;
    nanos
}

fn index_as_f64(index: u64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "frame indices above 2^53 (≈ 9.4e15) are not realistic for recorder sessions"
    )]
    let n = index as f64;
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_90_at_30fps_equals_three_seconds() {
        let t = MediaTime::from_frame(90, 30.0);
        assert!(
            (t.as_seconds() - 3.0).abs() < 1e-9,
            "got {} s",
            t.as_seconds()
        );
    }

    #[test]
    fn sample_48000_at_48khz_equals_one_second() {
        let t = MediaTime::from_sample(48_000, 48_000);
        assert!(
            (t.as_seconds() - 1.0).abs() < 1e-12,
            "got {} s ({} ns)",
            t.as_seconds(),
            t.as_nanos()
        );
        // Exact in integer space.
        assert_eq!(t.as_nanos(), 1_000_000_000);
    }

    #[test]
    fn sample_round_trip_is_exact() {
        // Pick a few sample indices and confirm to_sample inverts from_sample.
        for (idx, rate) in [
            (0_u64, 48_000_u32),
            (1, 44_100),
            (48_000, 48_000),
            (1_234_567, 48_000),
        ] {
            let t = MediaTime::from_sample(idx, rate);
            assert_eq!(
                t.to_sample(rate),
                i64::try_from(idx).unwrap(),
                "round-trip failed for idx={idx} rate={rate}"
            );
        }
    }

    #[test]
    fn frame_round_trip_is_exact_for_common_rates() {
        for (idx, fps) in [(0_u64, 30.0), (1, 30.0), (90, 30.0), (60, 60.0), (24, 24.0)] {
            let t = MediaTime::from_frame(idx, fps);
            assert_eq!(
                t.to_frame(fps),
                i64::try_from(idx).unwrap(),
                "round-trip failed for idx={idx} fps={fps}"
            );
        }
    }

    #[test]
    fn duration_arith_seconds() {
        let a = MediaDuration::from_seconds(1.5);
        let b = MediaDuration::from_seconds(0.5);
        assert!((((a + b).as_seconds()) - 2.0).abs() < 1e-9);
        assert!((((a - b).as_seconds()) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn duration_arith_via_media_time() {
        let t0 = MediaTime::from_seconds(1.0);
        let t1 = MediaTime::from_seconds(4.25);
        let d = t1 - t0;
        assert!((d.as_seconds() - 3.25).abs() < 1e-9);
        assert_eq!((t0 + d), t1);
    }

    #[test]
    fn media_time_is_monotonically_ordered() {
        let a = MediaTime::from_seconds(1.0);
        let b = MediaTime::from_seconds(1.000_000_001);
        let c = MediaTime::from_seconds(2.0);
        assert!(a < b);
        assert!(b < c);
        assert!(a < c);
        let mut v = vec![c, a, b];
        v.sort();
        assert_eq!(v, vec![a, b, c]);
    }

    #[test]
    fn manual_clock_only_advances_when_told() {
        let c = MediaClock::manual(MediaTime::ZERO);
        assert_eq!(c.now(), MediaTime::ZERO);
        c.advance_by(MediaDuration::from_seconds(0.5));
        assert!((c.now().as_seconds() - 0.5).abs() < 1e-12);
        c.advance_by(MediaDuration::from_seconds(0.5));
        assert!((c.now().as_seconds() - 1.0).abs() < 1e-12);
        assert!(c.is_manual());
    }

    #[test]
    fn wall_clock_is_monotonically_non_decreasing() {
        let c = MediaClock::wall_clock();
        let mut last = c.now();
        for _ in 0..10 {
            let t = c.now();
            assert!(t >= last, "wall clock went backwards");
            last = t;
        }
        assert!(!c.is_manual());
    }

    #[test]
    fn timestamped_carries_value() {
        let c = MediaClock::manual(MediaTime::from_seconds(2.5));
        let ts: Timestamped<&str> = c.assign("hello");
        assert_eq!(ts.value, "hello");
        assert!((ts.time.as_seconds() - 2.5).abs() < 1e-12);
    }

    #[test]
    fn duration_abs_for_drift_reporting() {
        let d = MediaDuration::from_seconds(-0.03);
        assert!((d.abs().as_seconds() - 0.03).abs() < 1e-9);
    }

    #[test]
    fn duration_from_millis_is_exact() {
        let d = MediaDuration::from_millis(20);
        assert_eq!(d.as_nanos(), 20_000_000);
    }

    #[test]
    fn types_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MediaTime>();
        assert_send_sync::<MediaDuration>();
        assert_send_sync::<MediaClock>();
        assert_send_sync::<Timestamped<u32>>();
    }
}
