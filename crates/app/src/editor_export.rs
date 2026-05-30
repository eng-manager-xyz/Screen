//! Deferred export frame generator (ED.20 / M-EDIT).
//!
//! The optical printer was the lab's export stage: it re-photographed the
//! cut negative one frame at a time onto fresh stock, honoring every edit
//! decision as it went. This is that printer in software — given an
//! [`EditProject`], it walks the *project* frames `0..project_duration` and,
//! for each, maps to the source frame via [`EditProject::source_time`] (so
//! trim, split, and speed are all baked in), decodes it from a seekable
//! [`EditorVideoStream`], and composes it through the **same**
//! [`EditorPreview`] path the live preview uses. The result is a
//! deterministic frame stream the exporter (ED.21) feeds to the encoder, so
//! the file you export matches the cut you scrubbed.
//!
//! It is **forward-only**: project frames are visited in order, and our edit
//! ops never reorder the timeline, so the source frames it requests are
//! monotonic non-decreasing — the decode stream never re-spawns (cheap,
//! and asserted by the golden test via [`Self::spawn_count`]).
//!
//! The cinematic *visual* edits — zoom punch-ins, crop reframe, and the
//! background framing — apply as a transform on the composed screen sprite;
//! that render-integration step (and its visual verification) lands next.
//! This chunk is the frame-accurate timeline walk the whole export rests on.

use std::path::Path;
use std::time::Duration;

use decode::EditorVideoStream;
use edit::EditProject;

use crate::editor_preview::EditorPreview;
use crate::recording_compose::ComposedFrame;

/// One generated export frame.
pub struct ExportFrame {
    /// The composed BGRA frame.
    pub frame: ComposedFrame,
    /// Presentation timestamp in project time (matches the live encoder's
    /// `feed_real_capture` formula so timestamps line up).
    pub pts: Duration,
    /// The source frame this project frame mapped to (for verification).
    pub source_frame: u64,
}

/// Walks an [`EditProject`] into a deterministic composed-frame stream.
pub struct ExportFrameGenerator {
    project: EditProject,
    stream: EditorVideoStream,
    preview: EditorPreview,
    next: u64,
    total: u64,
}

impl ExportFrameGenerator {
    /// Open the source clip + compose pipeline for `project`.
    ///
    /// # Errors
    ///
    /// Returns a message if the source can't be opened or the wgpu compose
    /// pipeline can't be created.
    pub fn new(project: EditProject, source: &Path) -> Result<Self, String> {
        let stream = EditorVideoStream::open(source).map_err(|e| format!("open source: {e}"))?;
        let preview = EditorPreview::new(project.source.width, project.source.height)
            .map_err(|e| format!("init compose: {e}"))?;
        let total = project.project_duration();
        Ok(Self {
            project,
            stream,
            preview,
            next: 0,
            total,
        })
    }

    /// Total project frames the generator will emit.
    #[must_use]
    pub fn frame_count(&self) -> u64 {
        self.total
    }

    /// Decode pipelines spawned so far — stays `1` for a full forward walk.
    #[must_use]
    pub fn spawn_count(&self) -> u64 {
        self.stream.spawn_count()
    }

    /// Generate the next project frame, or `None` at the end of the project.
    pub fn next_frame(&mut self) -> Option<ExportFrame> {
        if self.next >= self.total {
            return None;
        }
        let f = self.next;
        let source_frame = self.project.source_time(f)?;
        let decoded = self.stream.frame(source_frame)?;
        let frame = self.preview.render_frame(decoded.bgra)?;
        let fps = u64::from(self.project.project_fps.max(1));
        let pts = Duration::from_micros(f * (1_000_000 / fps));
        self.next += 1;
        Some(ExportFrame {
            frame,
            pts,
            source_frame,
        })
    }
}
