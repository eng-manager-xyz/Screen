//! Timeline segments — the ordered list that encodes trim, split, and
//! speed, plus the project↔source frame arithmetic.

use serde::{Deserialize, Serialize};

/// A zero-based frame index. Source frames (positions in the original
/// recording) and project frames (positions on the edited timeline) are
/// both `Frame`; field names and docs disambiguate which space a value
/// lives in. Matches `decode::VideoFrame::frame_index`'s `u64`.
pub type Frame = u64;

/// One contiguous slice of the source recording, played at `timescale`.
///
/// The project's video is the ordered concatenation of its segments:
///
/// - **Trim** = adjust [`source_start`](Self::source_start) /
///   [`source_end`](Self::source_end).
/// - **Split** = replace one segment with two adjacent ones that share
///   the cut frame.
/// - **Speed** = change [`timescale`](Self::timescale).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimelineSegment {
    /// First source frame of the slice (inclusive).
    pub source_start: Frame,
    /// One past the last source frame of the slice (exclusive), so
    /// `source_end - source_start` is the slice length in source frames.
    pub source_end: Frame,
    /// Playback speed multiplier. `1.0` = real time; `2.0` = 2× (the
    /// slice's source span is traversed in half the project time);
    /// `0.5` = half speed (slow motion). Always `> 0`; non-positive
    /// values are treated as `1.0` by the mapping helpers.
    pub timescale: f64,
}

impl TimelineSegment {
    /// A real-time (`timescale = 1.0`) slice of
    /// `[source_start, source_end)`.
    #[must_use]
    pub fn new(source_start: Frame, source_end: Frame) -> Self {
        Self {
            source_start,
            source_end,
            timescale: 1.0,
        }
    }

    /// A slice of `[source_start, source_end)` played at `timescale`.
    #[must_use]
    pub fn with_speed(source_start: Frame, source_end: Frame, timescale: f64) -> Self {
        Self {
            source_start,
            source_end,
            timescale,
        }
    }

    /// Length of the slice in **source** frames
    /// (`source_end - source_start`, saturating at 0).
    #[must_use]
    pub fn source_len(self) -> Frame {
        self.source_end.saturating_sub(self.source_start)
    }

    /// Length of the slice in **project** frames after applying
    /// `timescale`. A 100-source-frame slice at `timescale = 2.0`
    /// occupies 50 project frames; at `0.5`, 200 project frames. Always
    /// at least 1 project frame for a non-empty slice so a sped-up clip
    /// never vanishes entirely.
    #[must_use]
    pub fn project_len(self) -> Frame {
        let src = self.source_len();
        if src == 0 {
            return 0;
        }
        scale_div(src, self.norm_timescale()).max(1)
    }

    /// Map a project-frame offset *within this segment*
    /// (`0..self.project_len()`) to the source frame the renderer should
    /// decode. Clamped to the last source frame of the slice so it can
    /// never index past [`source_end`](Self::source_end).
    #[must_use]
    pub fn source_frame_at(self, project_offset: Frame) -> Frame {
        let raw = self.source_start + scale_mul(project_offset, self.norm_timescale());
        raw.min(self.source_end.saturating_sub(1).max(self.source_start))
    }

    /// `timescale`, sanitized: non-finite or non-positive values fall
    /// back to real time (`1.0`) so the mapping never divides by zero or
    /// produces NaN.
    fn norm_timescale(self) -> f64 {
        if self.timescale.is_finite() && self.timescale > 0.0 {
            self.timescale
        } else {
            1.0
        }
    }
}

/// `round(n / scale)` in frame space — the single contained cast site
/// for the project↔source mapping.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "frame counts are well under 2^52 so u64→f64 is lossless; the result is clamped non-negative and rounded before the f64→u64 cast"
)]
fn scale_div(n: Frame, scale: f64) -> Frame {
    (n as f64 / scale).round().max(0.0) as Frame
}

/// `round(n * scale)` in frame space.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "frame counts are well under 2^52 so u64→f64 is lossless; the result is clamped non-negative and rounded before the f64→u64 cast"
)]
fn scale_mul(n: Frame, scale: f64) -> Frame {
    (n as f64 * scale).round().max(0.0) as Frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_time_segment_maps_one_to_one() {
        let seg = TimelineSegment::new(100, 200);
        assert_eq!(seg.source_len(), 100);
        assert_eq!(seg.project_len(), 100);
        assert_eq!(seg.source_frame_at(0), 100);
        assert_eq!(seg.source_frame_at(50), 150);
        // Last project frame maps inside the slice (never == source_end).
        assert_eq!(seg.source_frame_at(99), 199);
    }

    #[test]
    fn double_speed_halves_project_length() {
        let seg = TimelineSegment::with_speed(0, 100, 2.0);
        assert_eq!(seg.source_len(), 100);
        assert_eq!(seg.project_len(), 50);
        // project offset p → source 2p
        assert_eq!(seg.source_frame_at(0), 0);
        assert_eq!(seg.source_frame_at(25), 50);
        // End of segment stays inside the slice.
        assert!(seg.source_frame_at(49) < 100);
    }

    #[test]
    fn half_speed_doubles_project_length() {
        let seg = TimelineSegment::with_speed(0, 100, 0.5);
        assert_eq!(seg.project_len(), 200);
        assert_eq!(seg.source_frame_at(0), 0);
        assert_eq!(seg.source_frame_at(100), 50);
    }

    #[test]
    fn empty_slice_has_zero_project_length() {
        let seg = TimelineSegment::new(42, 42);
        assert_eq!(seg.source_len(), 0);
        assert_eq!(seg.project_len(), 0);
    }

    #[test]
    fn non_empty_sped_up_slice_never_vanishes() {
        // 1 source frame at 100× would round to 0 without the .max(1).
        let seg = TimelineSegment::with_speed(0, 1, 100.0);
        assert_eq!(seg.project_len(), 1);
    }

    #[test]
    fn invalid_timescale_falls_back_to_real_time() {
        for bad in [0.0, -2.0, f64::NAN, f64::INFINITY] {
            let seg = TimelineSegment::with_speed(0, 100, bad);
            assert_eq!(seg.project_len(), 100, "bad timescale {bad} → real time");
            assert_eq!(seg.source_frame_at(10), 10);
        }
    }

    #[test]
    fn source_frame_never_reaches_source_end() {
        let seg = TimelineSegment::with_speed(10, 20, 0.3);
        for p in 0..seg.project_len() {
            let f = seg.source_frame_at(p);
            assert!(f >= seg.source_start && f < seg.source_end, "p={p} → f={f}");
        }
    }
}
