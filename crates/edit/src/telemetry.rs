//! Click telemetry → auto-zoom regions (ED.17 / M-EDIT).
//!
//! Screen recorders earn their "cinematic" feel by punching in where the
//! user is *working* — and the user tells you where that is every time they
//! click. This module turns a recorded click log into
//! [`ZoomSegment`]s: clicks close together in time
//! form one cluster, and each cluster becomes a zoom that opens just before
//! the first click, holds through the last, and targets the cluster's
//! centroid. The result is an ordinary, fully-editable list of zooms (the
//! user can nudge, delete, or retune any of them) — auto-zoom is a *starting
//! point*, not a lock-in.
//!
//! It is pure arithmetic over a click list, so it is exhaustively testable
//! without a recorder. The OS-level capture that *produces* the click log
//! (a per-platform surface, macOS first) is a separate follow-up; this is
//! the generator that consumes it.

use serde::{Deserialize, Serialize};

use crate::segment::Frame;
use crate::style::AutoZoomConfig;
use crate::zoom::{EditEase, ZoomId, ZoomMode, ZoomSegment};

/// A click captured during recording: a project frame plus a normalized
/// position in the composed frame (`(0, 0)` top-left, `(1, 1)` bottom-right).
///
/// `Serialize`/`Deserialize` so a captured click log persists in the project
/// document — it feeds both auto-zoom (ED.17) and the cursor click-ripple
/// overlay (ED.19).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClickEvent {
    /// Project frame the click occurred at.
    pub frame: Frame,
    /// Horizontal position, `0.0..=1.0`.
    pub x: f32,
    /// Vertical position, `0.0..=1.0`.
    pub y: f32,
}

impl ClickEvent {
    /// A click at `frame` and normalized `(x, y)`.
    #[must_use]
    pub fn new(frame: Frame, x: f32, y: f32) -> Self {
        Self { frame, x, y }
    }
}

/// A cursor position sample at a project frame, normalized to the composed
/// frame (`(0, 0)` top-left, `(1, 1)` bottom-right — the same convention as
/// [`ClickEvent`]).
///
/// Captured by the macOS event tap during recording (ED.17, the `app` crate)
/// and consumed by the cursor overlay renderer (ED.19). The `frame` is a
/// **project** frame, so the overlay syncs to the playhead 1:1.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CursorSample {
    /// Project frame this position was sampled at.
    pub frame: Frame,
    /// Horizontal position, `0.0..=1.0`.
    pub x: f32,
    /// Vertical position, `0.0..=1.0`.
    pub y: f32,
}

impl CursorSample {
    /// A cursor sample at `frame` and normalized `(x, y)`.
    #[must_use]
    pub fn new(frame: Frame, x: f32, y: f32) -> Self {
        Self { frame, x, y }
    }
}

/// Generate auto-zoom regions from a click log.
///
/// Clicks within ~1 s of each other form one cluster; each cluster becomes
/// a zoom that opens ~0.3 s before the first click, holds `cfg.hold_time_ms`
/// after the last, and targets the cluster's centroid at `cfg.max_zoom`.
/// Sub-half-second windows are dropped, and adjacent windows are clamped so
/// they never overlap. Returns an empty list if detection is disabled or
/// there are no clicks. The regions are concrete `Manual`-targeted zooms so
/// they punch into the click immediately under the [zoom
/// engine](crate::zoom_anim); the user edits them like any other zoom.
#[must_use]
pub fn auto_zoom_segments(
    clicks: &[ClickEvent],
    fps: u32,
    cfg: &AutoZoomConfig,
) -> Vec<ZoomSegment> {
    if !cfg.detect_from_cursor || clicks.is_empty() {
        return Vec::new();
    }
    let fps_f = u64::from(fps.max(1));
    let hold = (fps_f * u64::from(cfg.hold_time_ms) / 1000).max(1);
    let merge_gap = fps_f; // ~1 s: clicks within a second cluster together
    let lead_in = fps_f * 3 / 10; // ~0.3 s ramp-in before the first click
    let min_len = fps_f / 2; // drop windows shorter than ~0.5 s

    // Cluster the clicks by time gap (sorted by frame).
    let mut sorted = clicks.to_vec();
    sorted.sort_by_key(|c| c.frame);
    let mut clusters: Vec<Vec<ClickEvent>> = Vec::new();
    let mut prev_frame: Option<Frame> = None;
    for c in sorted {
        let new_cluster = prev_frame.is_none_or(|pf| c.frame.saturating_sub(pf) > merge_gap);
        if new_cluster {
            clusters.push(Vec::new());
        }
        clusters
            .last_mut()
            .expect("a cluster was just pushed when needed")
            .push(c);
        prev_frame = Some(c.frame);
    }

    // One zoom per cluster.
    let mut out: Vec<ZoomSegment> = Vec::new();
    for (i, cluster) in clusters.iter().enumerate() {
        let first = cluster.first().expect("non-empty cluster").frame;
        let last = cluster.last().expect("non-empty cluster").frame;
        let start = first.saturating_sub(lead_in);
        let end = last + hold;
        if end.saturating_sub(start) < min_len {
            continue;
        }
        // Accumulate the count as f32 inside the fold so the centroid
        // divisor needs no integer cast (and no overflow cap on huge
        // clusters). `n >= 1.0` because the cluster is non-empty.
        let (sx, sy, n) = cluster
            .iter()
            .fold((0.0f32, 0.0f32, 0.0f32), |(ax, ay, an), c| {
                (ax + c.x, ay + c.y, an + 1.0)
            });
        out.push(ZoomSegment {
            id: ZoomId(u32::try_from(i).unwrap_or(u32::MAX)),
            start,
            end,
            amount: cfg.max_zoom,
            mode: ZoomMode::Manual {
                x: (sx / n).clamp(0.0, 1.0),
                y: (sy / n).clamp(0.0, 1.0),
            },
            ease: EditEase::default(),
        });
    }

    // Clamp adjacent windows so they never overlap (the later zoom wins its
    // own span; the earlier one ends where the next begins).
    for i in 1..out.len() {
        let next_start = out[i].start;
        if out[i - 1].end > next_start {
            out[i - 1].end = next_start;
        }
    }
    out.retain(|z| z.end > z.start);
    out
}

/// The smoothed cursor position at project `frame`, for the ED.19 overlay.
///
/// `track` is assumed sorted by frame (the capture produces it in frame
/// order). Returns `None` for an empty track; clamps to the first sample for
/// frames before the track starts. `smoothing` (`0..=100`) is an exponential
/// moving average over the samples up to `frame` — `0` is the raw latest
/// sample, higher values lag more (taming jitter, the way a fluid head tames
/// handheld). Pure arithmetic, exhaustively testable.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "`smoothing` is clamped to 0..=100 so the u32→f32 is exact"
)]
pub fn cursor_at(track: &[CursorSample], frame: Frame, smoothing: u32) -> Option<(f32, f32)> {
    let first = track.first()?;
    // alpha: 1.0 (no smoothing) → 0.08 (max lag).
    let s = (smoothing.min(100) as f32) / 100.0;
    let alpha = 1.0 - 0.92 * s;
    let mut pos = (first.x, first.y);
    let mut started = false;
    for sample in track.iter().take_while(|s| s.frame <= frame) {
        if started {
            pos.0 += alpha * (sample.x - pos.0);
            pos.1 += alpha * (sample.y - pos.1);
        } else {
            pos = (sample.x, sample.y);
            started = true;
        }
    }
    Some(pos)
}

/// Active click ripples at project `frame`, for the ED.19 overlay.
///
/// Each entry is `(x, y, age)` where `age` ramps `0.0` (at the click) →
/// `1.0` (at the end of the `ripple_frames` window); the renderer expands +
/// fades a ring across that age. Clicks outside the window are skipped.
/// Returns empty when `ripple_frames == 0`. Pure.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "the age numerator is < ripple_frames (a small per-window frame count) so the u64→f32 is exact"
)]
pub fn ripples_at(clicks: &[ClickEvent], frame: Frame, ripple_frames: u32) -> Vec<(f32, f32, f32)> {
    if ripple_frames == 0 {
        return Vec::new();
    }
    let span = u64::from(ripple_frames);
    clicks
        .iter()
        .filter(|c| frame >= c.frame && frame - c.frame < span)
        .map(|c| {
            let age = (frame - c.frame) as f32 / span as f32;
            (c.x, c.y, age)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> AutoZoomConfig {
        AutoZoomConfig::default() // detect=true, hold 1200 ms, max_zoom 2.4
    }

    #[test]
    fn no_clicks_or_disabled_yields_nothing() {
        assert!(auto_zoom_segments(&[], 30, &cfg()).is_empty());
        let mut off = cfg();
        off.detect_from_cursor = false;
        assert!(auto_zoom_segments(&[ClickEvent::new(100, 0.5, 0.5)], 30, &off).is_empty());
    }

    #[test]
    fn single_click_makes_one_centred_zoom() {
        // fps 30: hold=36, lead=9. Click at 100 → [91, 136), 2.4×, at (0.3,0.7).
        let z = auto_zoom_segments(&[ClickEvent::new(100, 0.3, 0.7)], 30, &cfg());
        assert_eq!(z.len(), 1);
        assert_eq!(z[0].start, 91);
        assert_eq!(z[0].end, 136);
        assert!((z[0].amount - 2.4).abs() < 1e-9);
        match z[0].mode {
            ZoomMode::Manual { x, y } => {
                assert!((x - 0.3).abs() < 1e-6 && (y - 0.7).abs() < 1e-6);
            }
            ZoomMode::Auto => panic!("auto-zoom should target the click"),
        }
    }

    #[test]
    fn nearby_clicks_merge_into_one_cluster_at_centroid() {
        // 100 and 120 are within merge_gap (30) → one zoom; centroid x = 0.4.
        let z = auto_zoom_segments(
            &[
                ClickEvent::new(100, 0.2, 0.5),
                ClickEvent::new(120, 0.6, 0.5),
            ],
            30,
            &cfg(),
        );
        assert_eq!(z.len(), 1);
        assert_eq!(z[0].start, 91); // 100 - 9
        assert_eq!(z[0].end, 156); // 120 + 36
        if let ZoomMode::Manual { x, .. } = z[0].mode {
            assert!((x - 0.4).abs() < 1e-6, "centroid of 0.2 and 0.6");
        }
    }

    #[test]
    fn distant_clicks_make_separate_non_overlapping_zooms() {
        // 100 and 140: gap 40 > merge_gap 30 → two clusters. A's window
        // [91,136) overlaps B's [131,176) → A clamped to end at 131.
        let z = auto_zoom_segments(
            &[
                ClickEvent::new(100, 0.2, 0.2),
                ClickEvent::new(140, 0.8, 0.8),
            ],
            30,
            &cfg(),
        );
        assert_eq!(z.len(), 2);
        assert!(z[0].end <= z[1].start, "windows must not overlap");
        assert_eq!(z[1].start, 131); // 140 - 9
    }

    #[test]
    fn clicks_are_sorted_before_clustering() {
        // Out-of-order input clusters the same as sorted input.
        let z = auto_zoom_segments(
            &[
                ClickEvent::new(140, 0.8, 0.8),
                ClickEvent::new(100, 0.2, 0.2),
            ],
            30,
            &cfg(),
        );
        assert_eq!(z.len(), 2);
        assert!(z[0].start < z[1].start);
    }

    #[test]
    fn cursor_at_empty_track_is_none() {
        assert!(cursor_at(&[], 10, 0).is_none());
    }

    #[test]
    fn cursor_at_no_smoothing_is_the_latest_sample() {
        let track = [
            CursorSample::new(0, 0.1, 0.2),
            CursorSample::new(10, 0.6, 0.7),
        ];
        // At frame 10 with smoothing 0 → the raw latest sample.
        let (x, y) = cursor_at(&track, 10, 0).unwrap();
        assert!((x - 0.6).abs() < 1e-6 && (y - 0.7).abs() < 1e-6);
        // At frame 5, the latest sample at/before is frame 0.
        let (x, y) = cursor_at(&track, 5, 0).unwrap();
        assert!((x - 0.1).abs() < 1e-6 && (y - 0.2).abs() < 1e-6);
    }

    #[test]
    fn cursor_at_before_track_clamps_to_first() {
        let track = [CursorSample::new(10, 0.3, 0.4)];
        let (x, y) = cursor_at(&track, 0, 50).unwrap();
        assert!((x - 0.3).abs() < 1e-6 && (y - 0.4).abs() < 1e-6);
    }

    #[test]
    fn cursor_at_smoothing_lags_behind_a_jump() {
        // Position jumps 0 → 1 at frame 1; heavy smoothing must land between
        // (lagging the jump), never overshoot past the raw target.
        let track = [
            CursorSample::new(0, 0.0, 0.0),
            CursorSample::new(1, 1.0, 1.0),
        ];
        let (raw, _) = cursor_at(&track, 1, 0).unwrap();
        assert!((raw - 1.0).abs() < 1e-6, "no smoothing reaches the jump");
        let (lag, _) = cursor_at(&track, 1, 100).unwrap();
        assert!(
            lag > 0.0 && lag < 1.0,
            "max smoothing lags between (got {lag})"
        );
    }

    #[test]
    fn ripples_at_ramps_age_across_the_window() {
        let clicks = [ClickEvent::new(10, 0.5, 0.5)];
        // At the click: age 0.
        let r = ripples_at(&clicks, 10, 12);
        assert_eq!(r.len(), 1);
        assert!((r[0].2 - 0.0).abs() < 1e-6);
        // Halfway: age 0.5.
        assert!((ripples_at(&clicks, 16, 12)[0].2 - 0.5).abs() < 1e-6);
        // Past the window: gone.
        assert!(ripples_at(&clicks, 22, 12).is_empty());
        // Before the click: gone.
        assert!(ripples_at(&clicks, 5, 12).is_empty());
        // Zero window: never any ripples.
        assert!(ripples_at(&clicks, 10, 0).is_empty());
    }
}
