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

use crate::segment::Frame;
use crate::style::AutoZoomConfig;
use crate::zoom::{EditEase, ZoomId, ZoomMode, ZoomSegment};

/// A click captured during recording: a project frame plus a normalized
/// position in the composed frame (`(0, 0)` top-left, `(1, 1)` bottom-right).
#[derive(Clone, Copy, Debug, PartialEq)]
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
}
