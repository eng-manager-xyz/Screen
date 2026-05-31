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
//! and asserted by the golden test via [`ExportFrameGenerator::spawn_count`]).
//!
//! The cinematic *visual* edits — zoom punch-ins, crop reframe, and the
//! background framing — apply as a transform on the composed screen sprite;
//! that render-integration step (and its visual verification) lands next.
//! This chunk is the frame-accurate timeline walk the whole export rests on.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use decode::EditorVideoStream;
use edit::EditProject;
use media::encode::{EncoderConfig, LiveGstreamerEncoder, OutputFormat, VideoEncoder};

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
        // Export is a strictly-forward, monotonic walk (`source_time` is
        // non-decreasing across project frames), so a single-frame cache is
        // sufficient — it still serves the repeated source frame a slow-
        // motion segment requests, while the default 300-frame LRU would
        // pin ~2.5 GB of decoded BGRA at 1080p for no benefit.
        let stream = EditorVideoStream::open_with_cache(source, 1)
            .map_err(|e| format!("open source: {e}"))?;
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

/// Export an entire edited project to a single video file (ED.21).
///
/// Drives the [`ExportFrameGenerator`] frame by frame into a fresh
/// [`LiveGstreamerEncoder`] (reused unchanged from the live recorder), then
/// finalizes the container. Synchronous and long-running — call it from a
/// blocking task. `cancel` is polled once per frame (for the export UI,
/// ED.22) and `on_progress(done, total)` reports after each frame.
///
/// The output is composed at the **source** clip's dimensions; the cinematic
/// visual transforms (zoom / crop / background) and the per-segment audio
/// retime land with the render-integration + audio follow-ups, so this is a
/// faithful, retimed, video-only export today.
///
/// # Errors
///
/// Returns a message if the source can't be opened, the encoder can't start,
/// a frame fails to encode, or `cancel` fired mid-export.
pub fn export_edited_project(
    project: EditProject,
    source: &Path,
    output_path: PathBuf,
    format: OutputFormat,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<PathBuf, String> {
    let total = project.project_duration();
    let mut config = EncoderConfig::for_output(output_path, format);
    config.width = project.source.width;
    config.height = project.source.height;
    config.framerate = project.project_fps;

    let mut generator = ExportFrameGenerator::new(project, source)?;
    let mut encoder =
        LiveGstreamerEncoder::new(config).map_err(|e| format!("start encoder: {e}"))?;

    let mut done = 0u64;
    while let Some(ef) = generator.next_frame() {
        if cancel.load(Ordering::Relaxed) {
            return Err("export cancelled".to_owned());
        }
        encoder
            .push_video_frame(&ef.frame.bytes, ef.pts)
            .map_err(|e| format!("encode frame {done}: {e}"))?;
        done += 1;
        on_progress(done, total);
    }
    // The generator stops early (without a per-frame error) only if a source
    // decode failed mid-walk — don't finalize a truncated file as success.
    if done < total {
        return Err(format!(
            "export ended early at {done}/{total} frames (source decode failed?)"
        ));
    }
    Box::new(encoder)
        .finalize()
        .map_err(|e| format!("finalize export: {e}"))
}
