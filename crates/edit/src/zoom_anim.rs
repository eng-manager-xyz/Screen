//! The zoom engine — a [`ZoomSegment`] as a pure function of the frame.
//!
//! On an animation stand, the rostrum camera pushes in by riding a screw
//! drive toward the artwork — a slow, eased move from wide to tight and
//! back. This module is that move in software: given a zoom region and a
//! project frame, it returns the [`ZoomTransform`] (a scale about a focal
//! point) the renderer applies to the composed frame. It is the heart of
//! the editor's signature look, and — because it is GPU-free arithmetic —
//! it is exhaustively unit-testable. Preview and export call the *same*
//! function, so what you scrub is what you ship.
//!
//! The profile is three phases over a zoom's `[start, end)` window: an
//! eased **ramp-in** from no-zoom to full `amount`, a **hold** at full,
//! and a symmetric eased **ramp-out** back to no-zoom. The ramp length is
//! clamped to half the window so the two ramps never overlap — a short
//! zoom degrades gracefully to a triangle (push-in straight into
//! push-out, no hold) instead of fighting itself.

use crate::project::EditProject;
use crate::segment::Frame;
use crate::zoom::{ZoomMode, ZoomSegment};

/// A scale-about-a-focal-point transform for one composed frame.
///
/// The renderer scales the framed screen by `scale` (`>= 1.0`) about the
/// normalized focal point `(center_x, center_y)` in `[0, 1]`. At identity
/// (`scale == 1.0`) the focal point is irrelevant — no push-in is applied.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomTransform {
    /// Zoom factor, `>= 1.0`. `1.0` is no zoom.
    pub scale: f64,
    /// Horizontal focal point, `0.0..=1.0` (`0` left, `1` right).
    pub center_x: f64,
    /// Vertical focal point, `0.0..=1.0` (`0` top, `1` bottom).
    pub center_y: f64,
}

impl ZoomTransform {
    /// The no-zoom transform — full frame, centred.
    #[must_use]
    pub fn identity() -> Self {
        Self {
            scale: 1.0,
            center_x: 0.5,
            center_y: 0.5,
        }
    }

    /// Whether this transform is (approximately) no-zoom.
    #[must_use]
    pub fn is_identity(self) -> bool {
        (self.scale - 1.0).abs() < 1e-6
    }
}

impl Default for ZoomTransform {
    fn default() -> Self {
        Self::identity()
    }
}

/// A sensible ramp length for `fps`: about 0.6 s of ease-in / ease-out —
/// the cinematic-but-snappy feel screen recorders converge on — floored
/// at a single frame so even a 1 fps project still animates.
#[must_use]
pub fn default_ramp_frames(fps: u32) -> Frame {
    (Frame::from(fps) * 6 / 10).max(1)
}

/// `num / den` as `f64`, via a lossless `u32` hop (frame counts are tiny;
/// `den` is always `> 0` at the call sites below).
fn frac(num: Frame, den: Frame) -> f64 {
    let n = u32::try_from(num).unwrap_or(u32::MAX);
    let d = u32::try_from(den).unwrap_or(u32::MAX);
    f64::from(n) / f64::from(d)
}

/// The zoom transform at project `frame` for one segment.
///
/// Ramps identity → `seg.amount` → identity over `[start, end)` using the
/// segment's ease. `ramp_frames` is the desired in/out ramp; it is clamped
/// to half the window so the ramps never overlap. Outside the window (or
/// for an empty window) the result is [`ZoomTransform::identity`].
#[must_use]
pub fn zoom_at(seg: &ZoomSegment, frame: Frame, ramp_frames: Frame) -> ZoomTransform {
    if seg.is_empty() || !seg.contains(frame) {
        return ZoomTransform::identity();
    }
    let (center_x, center_y) = match seg.mode {
        ZoomMode::Manual { x, y } => (f64::from(x), f64::from(y)),
        // Auto targets become concrete points via click telemetry (ED.17);
        // until resolved, an Auto zoom punches into the frame centre.
        ZoomMode::Auto => (0.5, 0.5),
    };
    let len = seg.len();
    let ramp = ramp_frames.min(len / 2);
    let local = frame - seg.start;
    let progress = if ramp == 0 {
        1.0
    } else if local < ramp {
        seg.ease.eval(frac(local, ramp))
    } else if local >= len - ramp {
        seg.ease.eval(frac(len - local, ramp))
    } else {
        1.0
    };
    let scale = 1.0 + (seg.amount - 1.0) * progress;
    ZoomTransform {
        scale,
        center_x,
        center_y,
    }
}

/// The active zoom transform at project `frame` across the whole project:
/// the transform of the first zoom region whose window contains `frame`,
/// or [`ZoomTransform::identity`] when none do. The ramp is derived from
/// the project's frame rate.
#[must_use]
pub fn active_zoom_at(project: &EditProject, frame: Frame) -> ZoomTransform {
    let ramp = default_ramp_frames(project.project_fps);
    project
        .zooms
        .iter()
        .find(|z| z.contains(frame))
        .map_or_else(ZoomTransform::identity, |z| zoom_at(z, frame, ramp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::ClipRef;
    use crate::zoom::{ZoomId, ZoomMode, ZoomSegment};
    use std::path::PathBuf;

    /// A 100-frame, 2× centre zoom over `[100, 200)`.
    fn seg() -> ZoomSegment {
        ZoomSegment::manual(ZoomId(1), 100, 200, 2.0)
    }

    #[test]
    fn identity_outside_window() {
        let z = seg();
        assert!(zoom_at(&z, 99, 10).is_identity());
        assert!(zoom_at(&z, 200, 10).is_identity());
        assert_eq!(zoom_at(&z, 50, 10), ZoomTransform::identity());
    }

    #[test]
    fn full_amount_during_hold() {
        // len 100, ramp 10 → hold is [110, 190).
        let t = zoom_at(&seg(), 150, 10);
        assert!((t.scale - 2.0).abs() < 1e-9, "hold = full amount");
        assert!((t.center_x - 0.5).abs() < 1e-9);
        assert!((t.center_y - 0.5).abs() < 1e-9);
    }

    #[test]
    fn ramp_in_is_monotonic_and_bounded() {
        let z = seg();
        let mut prev = 0.0;
        for f in 100..110 {
            let s = zoom_at(&z, f, 10).scale;
            assert!((1.0 - 1e-9..=2.0 + 1e-9).contains(&s));
            assert!(s + 1e-9 >= prev, "ramp-in non-decreasing at {f}");
            prev = s;
        }
        // The first frame is identity (ease(0) = 0).
        assert!((zoom_at(&z, 100, 10).scale - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ramp_out_returns_toward_identity() {
        let z = seg();
        let full = zoom_at(&z, 190, 10).scale; // start of ramp-out: still full
        let later = zoom_at(&z, 199, 10).scale; // deep in ramp-out: less
        assert!((full - 2.0).abs() < 1e-9);
        assert!(later < full, "ramp-out decreasing");
        assert!(later >= 1.0 - 1e-9);
    }

    #[test]
    fn ramp_clamped_to_half_window() {
        // len 10, requested ramp 100 → clamped to 5, no overlap (triangle).
        let z = ZoomSegment::manual(ZoomId(2), 0, 10, 3.0);
        let mid = zoom_at(&z, 5, 100).scale;
        assert!(mid > 1.0 && mid <= 3.0 + 1e-9);
        assert!((zoom_at(&z, 0, 100).scale - 1.0).abs() < 1e-9);
    }

    #[test]
    fn manual_target_sets_center() {
        let mut z = seg();
        z.mode = ZoomMode::Manual { x: 0.2, y: 0.8 };
        let t = zoom_at(&z, 150, 10);
        assert!((t.center_x - 0.2).abs() < 1e-6);
        assert!((t.center_y - 0.8).abs() < 1e-6);
    }

    #[test]
    fn auto_falls_back_to_centre() {
        let mut z = seg();
        z.mode = ZoomMode::Auto;
        let t = zoom_at(&z, 150, 10);
        assert!((t.center_x - 0.5).abs() < 1e-9);
        assert!((t.center_y - 0.5).abs() < 1e-9);
    }

    #[test]
    fn default_ramp_is_about_point_six_seconds() {
        assert_eq!(default_ramp_frames(30), 18);
        assert_eq!(default_ramp_frames(60), 36);
        assert_eq!(default_ramp_frames(1), 1); // floored at one frame
    }

    #[test]
    fn active_zoom_picks_containing_segment() {
        let mut p = EditProject::from_recording(ClipRef::new(
            PathBuf::from("/tmp/a.mp4"),
            1920,
            1080,
            30,
            600,
        ));
        p.zooms = vec![ZoomSegment::manual(ZoomId(1), 100, 200, 2.0)];
        assert!(active_zoom_at(&p, 50).is_identity());
        assert!(!active_zoom_at(&p, 150).is_identity());
        // The hold reaches full amount.
        assert!((active_zoom_at(&p, 150).scale - 2.0).abs() < 1e-9);
    }
}
