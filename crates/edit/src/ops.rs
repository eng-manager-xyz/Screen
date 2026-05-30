//! Edit operations and the invariants they preserve.
//!
//! Every timeline edit is an [`EditOp`] applied to an
//! [`EditProject`]. Operations are validated and
//! invariant-preserving (see [`EditProject::check_invariants`]). Undo /
//! redo is layered on top in [`crate::history`].
//!
//! Trim / split / speed operate on the segment list; the zoom operations
//! operate on the zoom list, which is kept sorted by start frame.

use crate::project::EditProject;
use crate::segment::{Frame, TimelineSegment};
use crate::style::{AspectRatio, BackgroundConfig, CropRect, CursorConfig};
use crate::zoom::{ZoomId, ZoomSegment};

/// Which edge of a segment a [`EditOp::Trim`] moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrimEdge {
    /// The in-point (`source_start`).
    Start,
    /// The out-point (`source_end`).
    End,
}

/// A single, undoable edit applied to an [`EditProject`].
#[derive(Clone, Debug, PartialEq)]
pub enum EditOp {
    /// Split the segment containing project frame `at` into two adjacent
    /// segments sharing the cut. A no-op if `at` lands on a boundary.
    Split {
        /// Project frame to cut at.
        at: Frame,
    },
    /// Move segment `index`'s in/out point to source frame `to`,
    /// clamped so the segment stays non-empty and within the source.
    Trim {
        /// Index into [`EditProject::segments`].
        index: usize,
        /// Which edge to move.
        edge: TrimEdge,
        /// Target source frame.
        to: Frame,
    },
    /// Remove project range `[start, end)`, closing the gap (ripple).
    RippleDelete {
        /// Range start (project frame, inclusive).
        start: Frame,
        /// Range end (project frame, exclusive).
        end: Frame,
    },
    /// Set segment `index`'s playback speed (`timescale`), sanitized to a
    /// finite positive multiplier.
    SetSpeed {
        /// Index into [`EditProject::segments`].
        index: usize,
        /// New speed multiplier.
        timescale: f64,
    },
    /// Add a zoom region. Its `id` is assigned fresh on insert (the `id`
    /// field of the supplied value is ignored).
    AddZoom {
        /// The zoom to add (window / amount / mode / ease).
        zoom: ZoomSegment,
    },
    /// Remove the zoom with the given id.
    RemoveZoom {
        /// Id of the zoom to remove.
        id: ZoomId,
    },
    /// Move a zoom's window to `[start, end)`.
    MoveZoom {
        /// Id of the zoom to move.
        id: ZoomId,
        /// New window start (project frame).
        start: Frame,
        /// New window end (project frame).
        end: Frame,
    },
    /// Set the crop rectangle. A full-frame rect clears the crop.
    SetCrop {
        /// Normalized crop rect (`[0, 1]` of the source frame).
        rect: CropRect,
    },
    /// Set the output aspect ratio (reframes the export canvas).
    SetAspect {
        /// New aspect ratio.
        ratio: AspectRatio,
    },
    /// Set the background framing (backdrop + padding / radius / shadow).
    SetBackground {
        /// New background config.
        config: BackgroundConfig,
    },
    /// Set the cursor styling (size / smoothing / ripples / hide-static).
    SetCursor {
        /// New cursor config.
        cursor: CursorConfig,
    },
}

/// Why an [`EditOp`] could not be applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    /// A segment index was out of range.
    SegmentIndexOutOfRange {
        /// The requested index.
        index: usize,
        /// The number of segments.
        len: usize,
    },
    /// A delete / move range was empty (`end <= start`).
    EmptyRange,
    /// No zoom with the requested id exists.
    ZoomNotFound(ZoomId),
    /// A project frame was at or past the end of the timeline.
    PastEndOfTimeline(Frame),
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SegmentIndexOutOfRange { index, len } => {
                write!(f, "segment index {index} out of range (len {len})")
            }
            Self::EmptyRange => write!(f, "range is empty (end <= start)"),
            Self::ZoomNotFound(id) => write!(f, "no zoom with id {id:?}"),
            Self::PastEndOfTimeline(frame) => {
                write!(f, "project frame {frame} is past the end of the timeline")
            }
        }
    }
}

impl std::error::Error for EditError {}

impl EditProject {
    /// Apply an edit operation, mutating the project in place.
    ///
    /// Prefer driving edits through [`crate::History`], which makes them
    /// undoable; this is the underlying primitive.
    ///
    /// # Errors
    ///
    /// Returns [`EditError`] for an out-of-range segment index, an empty
    /// range, an unknown zoom id, or a split past the end of the timeline.
    pub fn apply(&mut self, op: &EditOp) -> Result<(), EditError> {
        // Matched by reference so non-`Copy` payloads (e.g. `SetBackground`,
        // which owns a wallpaper `String`) bind without moving out of `*op`.
        match op {
            EditOp::Split { at } => self.apply_split(*at),
            EditOp::Trim { index, edge, to } => self.apply_trim(*index, *edge, *to),
            EditOp::RippleDelete { start, end } => self.apply_ripple_delete(*start, *end),
            EditOp::SetSpeed { index, timescale } => self.apply_set_speed(*index, *timescale),
            EditOp::AddZoom { zoom } => {
                self.apply_add_zoom(*zoom);
                Ok(())
            }
            EditOp::RemoveZoom { id } => self.apply_remove_zoom(*id),
            EditOp::MoveZoom { id, start, end } => self.apply_move_zoom(*id, *start, *end),
            EditOp::SetCrop { rect } => {
                self.apply_set_crop(*rect);
                Ok(())
            }
            EditOp::SetAspect { ratio } => {
                self.aspect = *ratio;
                Ok(())
            }
            EditOp::SetBackground { config } => {
                self.background = config.clone();
                Ok(())
            }
            EditOp::SetCursor { cursor } => {
                self.cursor = *cursor;
                Ok(())
            }
        }
    }

    fn apply_split(&mut self, at: Frame) -> Result<(), EditError> {
        let (index, cut) = self.locate(at).ok_or(EditError::PastEndOfTimeline(at))?;
        let seg = self.segments[index];
        // A split exactly on a segment boundary changes nothing.
        if cut <= seg.source_start || cut >= seg.source_end {
            return Ok(());
        }
        let left = TimelineSegment::with_speed(seg.source_start, cut, seg.timescale);
        let right = TimelineSegment::with_speed(cut, seg.source_end, seg.timescale);
        self.segments.splice(index..=index, [left, right]);
        Ok(())
    }

    fn apply_trim(&mut self, index: usize, edge: TrimEdge, to: Frame) -> Result<(), EditError> {
        let len = self.segments.len();
        let frame_count = self.source.frame_count;
        let seg = self
            .segments
            .get_mut(index)
            .ok_or(EditError::SegmentIndexOutOfRange { index, len })?;
        match edge {
            TrimEdge::Start => {
                let hi = seg.source_end.saturating_sub(1);
                seg.source_start = to.min(hi);
            }
            TrimEdge::End => {
                let lo = seg.source_start + 1;
                seg.source_end = to.max(lo).min(frame_count.max(lo));
            }
        }
        Ok(())
    }

    fn apply_ripple_delete(&mut self, d0: Frame, d1: Frame) -> Result<(), EditError> {
        if d1 <= d0 {
            return Err(EditError::EmptyRange);
        }
        let mut out: Vec<TimelineSegment> = Vec::with_capacity(self.segments.len() + 1);
        let mut seg_start: Frame = 0;
        for seg in &self.segments {
            let seg_end = seg_start + seg.project_len();
            let overlaps = seg_start < d1 && seg_end > d0;
            if overlaps {
                if seg_start < d0 {
                    let cut = seg.source_frame_at(d0 - seg_start);
                    if cut > seg.source_start {
                        out.push(TimelineSegment::with_speed(
                            seg.source_start,
                            cut,
                            seg.timescale,
                        ));
                    }
                }
                if seg_end > d1 {
                    let cut = seg.source_frame_at(d1 - seg_start);
                    if seg.source_end > cut {
                        out.push(TimelineSegment::with_speed(
                            cut,
                            seg.source_end,
                            seg.timescale,
                        ));
                    }
                }
            } else {
                out.push(*seg);
            }
            seg_start = seg_end;
        }
        self.segments = out;
        Ok(())
    }

    fn apply_set_speed(&mut self, index: usize, timescale: f64) -> Result<(), EditError> {
        let len = self.segments.len();
        let seg = self
            .segments
            .get_mut(index)
            .ok_or(EditError::SegmentIndexOutOfRange { index, len })?;
        seg.timescale = if timescale.is_finite() && timescale > 0.0 {
            timescale
        } else {
            1.0
        };
        Ok(())
    }

    fn apply_add_zoom(&mut self, zoom: ZoomSegment) {
        let id = ZoomId(self.next_zoom_id);
        self.next_zoom_id = self.next_zoom_id.wrapping_add(1);
        self.zooms.push(ZoomSegment { id, ..zoom });
        self.zooms.sort_by_key(|z| z.start);
    }

    fn apply_remove_zoom(&mut self, id: ZoomId) -> Result<(), EditError> {
        let before = self.zooms.len();
        self.zooms.retain(|z| z.id != id);
        if self.zooms.len() == before {
            return Err(EditError::ZoomNotFound(id));
        }
        Ok(())
    }

    fn apply_move_zoom(&mut self, id: ZoomId, start: Frame, end: Frame) -> Result<(), EditError> {
        if end <= start {
            return Err(EditError::EmptyRange);
        }
        let zoom = self
            .zooms
            .iter_mut()
            .find(|z| z.id == id)
            .ok_or(EditError::ZoomNotFound(id))?;
        zoom.start = start;
        zoom.end = end;
        self.zooms.sort_by_key(|z| z.start);
        Ok(())
    }

    fn apply_set_crop(&mut self, rect: CropRect) {
        // A full-frame crop is stored as "no crop" so the export fast-path
        // can skip the videocrop element entirely.
        if rect.is_full() {
            self.crop = None;
            return;
        }
        // Sanitize to a valid in-frame sub-rect (non-zero extent, fully
        // inside `[0, 1]`) — the UI can submit any field value.
        let x = rect.x.clamp(0.0, 1.0 - 1e-3);
        let y = rect.y.clamp(0.0, 1.0 - 1e-3);
        let width = rect.width.clamp(1e-3, 1.0 - x);
        let height = rect.height.clamp(1e-3, 1.0 - y);
        self.crop = Some(CropRect {
            x,
            y,
            width,
            height,
        });
    }

    /// Check the project's structural invariants. Used by tests (and a
    /// useful debugging aid): segments are non-empty and within the
    /// source; the zoom list is sorted, each zoom is non-empty, and every
    /// zoom id is below `next_zoom_id`.
    ///
    /// # Errors
    ///
    /// Returns a human-readable description of the first violated
    /// invariant.
    pub fn check_invariants(&self) -> Result<(), String> {
        for (i, s) in self.segments.iter().enumerate() {
            if s.source_start >= s.source_end {
                return Err(format!("segment {i} is empty or inverted: {s:?}"));
            }
            if s.source_end > self.source.frame_count {
                return Err(format!(
                    "segment {i} extends past source ({} > {})",
                    s.source_end, self.source.frame_count
                ));
            }
            if !(s.timescale.is_finite() && s.timescale > 0.0) {
                return Err(format!("segment {i} has invalid timescale {}", s.timescale));
            }
        }
        for pair in self.zooms.windows(2) {
            if pair[0].start > pair[1].start {
                return Err("zoom list is not sorted by start".to_string());
            }
        }
        for (i, z) in self.zooms.iter().enumerate() {
            if z.end <= z.start {
                return Err(format!("zoom {i} is empty: {z:?}"));
            }
            if z.id.0 >= self.next_zoom_id {
                return Err(format!(
                    "zoom {i} id {} >= next_zoom_id {}",
                    z.id.0, self.next_zoom_id
                ));
            }
        }
        if let Some(c) = self.crop {
            let in_unit = |v: f32| (-1e-4..=1.0 + 1e-4).contains(&v);
            if !(in_unit(c.x)
                && in_unit(c.y)
                && c.width > 0.0
                && c.height > 0.0
                && c.x + c.width <= 1.0 + 1e-4
                && c.y + c.height <= 1.0 + 1e-4)
            {
                return Err(format!("crop rect out of bounds: {c:?}"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::ClipRef;
    use crate::zoom::ZoomSegment;
    use std::path::PathBuf;

    fn project() -> EditProject {
        EditProject::from_recording(ClipRef::new(
            PathBuf::from("/tmp/rec.mp4"),
            1920,
            1080,
            30,
            900,
        ))
    }

    #[test]
    fn split_makes_two_segments_and_preserves_duration() {
        let mut p = project();
        let before = p.project_duration();
        p.apply(&EditOp::Split { at: 300 }).unwrap();
        assert_eq!(p.segments.len(), 2);
        assert_eq!(p.segments[0], TimelineSegment::new(0, 300));
        assert_eq!(p.segments[1], TimelineSegment::new(300, 900));
        // Real-time split is exactly duration-preserving.
        assert_eq!(p.project_duration(), before);
        p.check_invariants().unwrap();
    }

    #[test]
    fn split_at_boundary_is_noop() {
        let mut p = project();
        p.apply(&EditOp::Split { at: 0 }).unwrap();
        assert_eq!(p.segments.len(), 1);
    }

    #[test]
    fn split_past_end_errors() {
        let mut p = project();
        assert_eq!(
            p.apply(&EditOp::Split { at: 5000 }),
            Err(EditError::PastEndOfTimeline(5000))
        );
    }

    #[test]
    fn trim_clamps_to_non_empty_and_bounds() {
        let mut p = project();
        p.apply(&EditOp::Trim {
            index: 0,
            edge: TrimEdge::Start,
            to: 100,
        })
        .unwrap();
        assert_eq!(p.segments[0].source_start, 100);
        // Trim end past the source clamps to frame_count.
        p.apply(&EditOp::Trim {
            index: 0,
            edge: TrimEdge::End,
            to: 99_999,
        })
        .unwrap();
        assert_eq!(p.segments[0].source_end, 900);
        p.check_invariants().unwrap();
    }

    #[test]
    fn trim_bad_index_errors() {
        let mut p = project();
        assert!(matches!(
            p.apply(&EditOp::Trim {
                index: 9,
                edge: TrimEdge::Start,
                to: 0
            }),
            Err(EditError::SegmentIndexOutOfRange { index: 9, len: 1 })
        ));
    }

    #[test]
    fn ripple_delete_closes_the_gap() {
        let mut p = project();
        // Delete the middle 300 project frames; duration drops by ~300 and
        // the timeline stays a contiguous concatenation (no gap).
        p.apply(&EditOp::RippleDelete {
            start: 300,
            end: 600,
        })
        .unwrap();
        assert_eq!(p.project_duration(), 600);
        // The two surviving pieces are [0,300) and [600,900).
        assert_eq!(p.segments.len(), 2);
        assert_eq!(p.segments[0], TimelineSegment::new(0, 300));
        assert_eq!(p.segments[1], TimelineSegment::new(600, 900));
        p.check_invariants().unwrap();
    }

    #[test]
    fn ripple_delete_empty_range_errors() {
        let mut p = project();
        assert_eq!(
            p.apply(&EditOp::RippleDelete { start: 50, end: 50 }),
            Err(EditError::EmptyRange)
        );
    }

    #[test]
    fn set_speed_changes_duration() {
        let mut p = project();
        p.apply(&EditOp::SetSpeed {
            index: 0,
            timescale: 2.0,
        })
        .unwrap();
        assert!((p.segments[0].timescale - 2.0).abs() < 1e-9);
        assert_eq!(p.project_duration(), 450);
        // Invalid speed sanitizes to real time.
        p.apply(&EditOp::SetSpeed {
            index: 0,
            timescale: -1.0,
        })
        .unwrap();
        assert!((p.segments[0].timescale - 1.0).abs() < 1e-9);
    }

    #[test]
    fn zoom_add_remove_move() {
        let mut p = project();
        p.apply(&EditOp::AddZoom {
            zoom: ZoomSegment::manual(ZoomId(999), 100, 200, 1.6),
        })
        .unwrap();
        assert_eq!(p.zooms.len(), 1);
        // Id is assigned fresh (the 999 placeholder is ignored).
        let id = p.zooms[0].id;
        assert_eq!(id, ZoomId(0));
        assert_eq!(p.next_zoom_id, 1);
        p.check_invariants().unwrap();

        // Move it.
        p.apply(&EditOp::MoveZoom {
            id,
            start: 400,
            end: 500,
        })
        .unwrap();
        assert_eq!(p.zooms[0].start, 400);
        assert_eq!(p.zooms[0].end, 500);

        // Remove it.
        p.apply(&EditOp::RemoveZoom { id }).unwrap();
        assert!(p.zooms.is_empty());
        // Removing a missing id errors.
        assert_eq!(
            p.apply(&EditOp::RemoveZoom { id }),
            Err(EditError::ZoomNotFound(id))
        );
    }

    #[test]
    fn added_zooms_stay_sorted_by_start() {
        let mut p = project();
        for start in [300, 100, 200] {
            p.apply(&EditOp::AddZoom {
                zoom: ZoomSegment::manual(ZoomId(0), start, start + 50, 1.5),
            })
            .unwrap();
        }
        let starts: Vec<_> = p.zooms.iter().map(|z| z.start).collect();
        assert_eq!(starts, vec![100, 200, 300]);
        p.check_invariants().unwrap();
    }

    #[test]
    fn set_aspect_changes_ratio() {
        let mut p = project();
        assert_eq!(p.aspect, AspectRatio::Wide);
        p.apply(&EditOp::SetAspect {
            ratio: AspectRatio::Vertical,
        })
        .unwrap();
        assert_eq!(p.aspect, AspectRatio::Vertical);
        p.check_invariants().unwrap();
    }

    #[test]
    fn set_crop_stores_subrect_and_clears_on_full() {
        let mut p = project();
        assert!(p.crop.is_none());
        let rect = CropRect {
            x: 0.1,
            y: 0.1,
            width: 0.8,
            height: 0.8,
        };
        p.apply(&EditOp::SetCrop { rect }).unwrap();
        assert_eq!(p.crop, Some(rect));
        p.check_invariants().unwrap();
        // A full-frame crop clears it.
        p.apply(&EditOp::SetCrop {
            rect: CropRect::full(),
        })
        .unwrap();
        assert!(p.crop.is_none());
    }

    #[test]
    fn out_of_bounds_crop_fails_invariants() {
        let mut p = project();
        // 0.5 + 0.8 > 1.0 — runs off the right edge.
        p.crop = Some(CropRect {
            x: 0.5,
            y: 0.0,
            width: 0.8,
            height: 1.0,
        });
        assert!(p.check_invariants().is_err());
    }

    #[test]
    fn set_background_replaces_config() {
        let mut p = project();
        let bg = BackgroundConfig {
            padding: 96,
            corner_radius: 20,
            shadow: 40,
            ..BackgroundConfig::default()
        };
        p.apply(&EditOp::SetBackground { config: bg.clone() })
            .unwrap();
        assert_eq!(p.background, bg);
        p.check_invariants().unwrap();
    }

    #[test]
    fn set_cursor_replaces_config() {
        let mut p = project();
        let cur = CursorConfig {
            size_pct: 220,
            smoothing: 40,
            click_ripples: false,
            ..CursorConfig::default()
        };
        p.apply(&EditOp::SetCursor { cursor: cur }).unwrap();
        assert_eq!(p.cursor, cur);
        p.check_invariants().unwrap();
    }
}
