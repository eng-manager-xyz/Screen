//! Reference to the source recording an [`EditProject`](crate::EditProject)
//! edits.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::segment::Frame;

/// Immutable metadata about the source recording a project edits.
///
/// The editor never rewrites the source file; every edit is expressed
/// against this reference (segments index into `0..frame_count`, the
/// renderer decodes from `path`). Stored in the project file so a
/// project re-opens against the right media.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipRef {
    /// Absolute path to the source recording on disk.
    pub path: PathBuf,
    /// Source frame width in pixels.
    pub width: u32,
    /// Source frame height in pixels.
    pub height: u32,
    /// Source frame rate (frames per second).
    pub source_fps: u32,
    /// Total number of frames in the source recording.
    pub frame_count: Frame,
}

impl ClipRef {
    /// Build a reference from a path + the source recording's metadata.
    #[must_use]
    pub fn new(
        path: PathBuf,
        width: u32,
        height: u32,
        source_fps: u32,
        frame_count: Frame,
    ) -> Self {
        Self {
            path,
            width,
            height,
            source_fps,
            frame_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_ref_round_trips_fields() {
        let c = ClipRef::new(PathBuf::from("/tmp/rec.mp4"), 1920, 1080, 30, 900);
        assert_eq!(c.width, 1920);
        assert_eq!(c.height, 1080);
        assert_eq!(c.source_fps, 30);
        assert_eq!(c.frame_count, 900);
        assert_eq!(c.path, PathBuf::from("/tmp/rec.mp4"));
    }
}
