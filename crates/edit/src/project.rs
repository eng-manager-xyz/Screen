//! [`EditProject`] — the top-level, serializable edit document and the
//! project↔source frame mapping built on top of the segment list.

use serde::{Deserialize, Serialize};

use crate::clip::ClipRef;
use crate::segment::{Frame, TimelineSegment};
use crate::style::{AspectRatio, BackgroundConfig, CropRect, CursorConfig};
use crate::zoom::ZoomSegment;

/// On-disk schema version. Bumped when the project file format changes
/// incompatibly so older files can be migrated (ED.23).
pub const SCHEMA_VERSION: u32 = 1;

/// Default project timeline frame rate. The editor's time authority runs
/// at this rate regardless of the source recording's frame rate.
pub const DEFAULT_PROJECT_FPS: u32 = 30;

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

fn default_project_fps() -> u32 {
    DEFAULT_PROJECT_FPS
}

/// The serialized source of truth for one editing session.
///
/// The edited video is the ordered concatenation of [`segments`]; the
/// cinematic framing comes from [`background`] / [`cursor`] / [`crop`] /
/// [`aspect`]; cinematic punch-ins come from [`zooms`]. Nothing here is
/// a GPU or media handle — the renderer and encoder re-derive every
/// frame from this model at preview + export time.
///
/// [`segments`]: Self::segments
/// [`background`]: Self::background
/// [`cursor`]: Self::cursor
/// [`crop`]: Self::crop
/// [`aspect`]: Self::aspect
/// [`zooms`]: Self::zooms
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EditProject {
    /// On-disk schema version (see [`SCHEMA_VERSION`]).
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// The source recording this project edits.
    pub source: ClipRef,
    /// Ordered timeline slices (trim / split / speed).
    pub segments: Vec<TimelineSegment>,
    /// Cinematic zoom regions.
    #[serde(default)]
    pub zooms: Vec<ZoomSegment>,
    /// Background framing (wallpaper / padding / radius / shadow).
    #[serde(default)]
    pub background: BackgroundConfig,
    /// Cursor styling + auto-zoom detection settings.
    #[serde(default)]
    pub cursor: CursorConfig,
    /// Optional crop / reframe of the source. `None` = full frame.
    #[serde(default)]
    pub crop: Option<CropRect>,
    /// Output aspect ratio.
    #[serde(default)]
    pub aspect: AspectRatio,
    /// Timeline frame rate (the editor's time authority).
    #[serde(default = "default_project_fps")]
    pub project_fps: u32,
    /// Monotonic counter for allocating fresh [`crate::ZoomId`]s
    /// (managed by edit operations in ED.2).
    #[serde(default)]
    pub next_zoom_id: u32,
}

impl EditProject {
    /// Build a fresh project from a source recording: a single,
    /// full-length, real-time segment with default framing and no zooms —
    /// i.e. "the recording, untouched", ready to edit.
    #[must_use]
    pub fn from_recording(source: ClipRef) -> Self {
        let segments = vec![TimelineSegment::new(0, source.frame_count)];
        Self {
            schema_version: SCHEMA_VERSION,
            source,
            segments,
            zooms: Vec::new(),
            background: BackgroundConfig::default(),
            cursor: CursorConfig::default(),
            crop: None,
            aspect: AspectRatio::default(),
            project_fps: DEFAULT_PROJECT_FPS,
            next_zoom_id: 0,
        }
    }

    /// Total length of the edited timeline in **project** frames.
    #[must_use]
    pub fn project_duration(&self) -> Frame {
        self.segments
            .iter()
            .copied()
            .map(TimelineSegment::project_len)
            .sum()
    }

    /// Locate a project frame: returns `(segment index, source frame)`,
    /// or `None` if the frame is at/past the end of the timeline.
    ///
    /// This is the core of the editor's time model — it walks the segment
    /// list accumulating project-frame lengths until it finds the segment
    /// containing `project_frame`, then maps the within-segment offset to
    /// a source frame via that segment's `timescale`.
    #[must_use]
    pub fn locate(&self, project_frame: Frame) -> Option<(usize, Frame)> {
        let mut acc: Frame = 0;
        for (index, seg) in self.segments.iter().enumerate() {
            let len = seg.project_len();
            if project_frame < acc + len {
                return Some((index, seg.source_frame_at(project_frame - acc)));
            }
            acc += len;
        }
        None
    }

    /// Map a project frame to the **source** frame the renderer should
    /// decode, or `None` past the end of the timeline.
    #[must_use]
    pub fn source_time(&self, project_frame: Frame) -> Option<Frame> {
        self.locate(project_frame).map(|(_, frame)| frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::CropRect;
    use crate::zoom::{ZoomId, ZoomSegment};

    fn sample_clip() -> ClipRef {
        ClipRef::new(PathBuf::from("/tmp/rec.mp4"), 1920, 1080, 30, 900)
    }

    use std::path::PathBuf;

    #[test]
    fn from_recording_is_single_full_length_realtime_segment() {
        let proj = EditProject::from_recording(sample_clip());
        assert_eq!(proj.segments.len(), 1);
        assert_eq!(proj.segments[0], TimelineSegment::new(0, 900));
        assert_eq!(proj.project_duration(), 900);
        assert!(proj.zooms.is_empty());
        assert!(proj.crop.is_none());
        assert_eq!(proj.project_fps, DEFAULT_PROJECT_FPS);
        assert_eq!(proj.schema_version, SCHEMA_VERSION);
        assert_eq!(proj.aspect, AspectRatio::Wide);
    }

    #[test]
    fn source_time_walks_segments_including_a_speed_segment() {
        let mut proj = EditProject::from_recording(sample_clip());
        // seg0: src[0,300) real-time   → 300 project frames (project 0..300)
        // seg1: src[300,600) at 2×     → 150 project frames (project 300..450)
        // seg2: src[600,900) real-time → 300 project frames (project 450..750)
        proj.segments = vec![
            TimelineSegment::new(0, 300),
            TimelineSegment::with_speed(300, 600, 2.0),
            TimelineSegment::new(600, 900),
        ];
        assert_eq!(proj.project_duration(), 300 + 150 + 300);

        assert_eq!(proj.source_time(0), Some(0));
        assert_eq!(proj.source_time(299), Some(299));
        // First project frame of the 2× segment maps to its source start.
        assert_eq!(proj.source_time(300), Some(300));
        // Mid 2× segment: project offset 75 → source 300 + 150 = 450.
        assert_eq!(proj.source_time(375), Some(450));
        // First project frame of the final real-time segment.
        assert_eq!(proj.source_time(450), Some(600));
        assert_eq!(proj.source_time(749), Some(899));
        // Past the end of the timeline.
        assert_eq!(proj.source_time(750), None);
    }

    #[test]
    fn locate_returns_segment_index() {
        let mut proj = EditProject::from_recording(sample_clip());
        proj.segments = vec![TimelineSegment::new(0, 300), TimelineSegment::new(300, 900)];
        assert_eq!(proj.locate(0), Some((0, 0)));
        assert_eq!(proj.locate(299), Some((0, 299)));
        assert_eq!(proj.locate(300), Some((1, 300)));
        assert_eq!(proj.locate(899), Some((1, 899)));
        assert_eq!(proj.locate(900), None);
    }

    #[test]
    fn serde_round_trip_is_lossless() {
        let mut proj = EditProject::from_recording(sample_clip());
        proj.segments = vec![
            TimelineSegment::new(0, 300),
            TimelineSegment::with_speed(300, 600, 2.0),
            TimelineSegment::new(600, 900),
        ];
        proj.zooms = vec![ZoomSegment::manual(ZoomId(0), 30, 90, 1.6)];
        proj.next_zoom_id = 1;
        proj.crop = Some(CropRect {
            x: 0.1,
            y: 0.0,
            width: 0.8,
            height: 1.0,
        });
        proj.aspect = AspectRatio::Vertical;

        let json = serde_json::to_string_pretty(&proj).expect("serialize");
        let back: EditProject = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(proj, back);
    }

    #[test]
    fn missing_optional_fields_deserialize_to_defaults() {
        // A minimal project file (only required fields) should fill the
        // rest from defaults — the forward-compat path for ED.23.
        let json = r#"{
            "source": { "path": "/tmp/x.mp4", "width": 1280, "height": 720, "source_fps": 30, "frame_count": 600 },
            "segments": [ { "source_start": 0, "source_end": 600, "timescale": 1.0 } ]
        }"#;
        let proj: EditProject = serde_json::from_str(json).expect("deserialize minimal");
        assert_eq!(proj.schema_version, SCHEMA_VERSION);
        assert_eq!(proj.project_fps, DEFAULT_PROJECT_FPS);
        assert_eq!(proj.aspect, AspectRatio::Wide);
        assert!(proj.zooms.is_empty());
        assert!(proj.crop.is_none());
        assert_eq!(proj.background, BackgroundConfig::default());
        assert_eq!(proj.project_duration(), 600);
    }
}
