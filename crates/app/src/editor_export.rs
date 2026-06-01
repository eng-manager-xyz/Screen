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
//! The cinematic *visual* edits apply as a transform on the composed screen
//! sprite: the zoom punch-in (ED.16) + the crop / aspect reframe (ED.15) are
//! written into the screen sprite each frame via
//! [`EditorPreview::render_framed`](crate::editor_preview::EditorPreview::render_framed),
//! and the background framing (ED.18) — the backdrop fill, padding inset, and
//! rounded-corner frame window — is applied once at construction via
//! [`EditorPreview::set_background`](crate::editor_preview::EditorPreview::set_background).
//! This chunk is the frame-accurate timeline walk the whole export rests on.
//!
//! Audio (ED.21) rides the same timeline: [`export_edited_project`] decodes
//! the source's audio once ([`media::encode::decode_source_audio_f32`]),
//! retimes it in pure Rust per the segment list ([`retime_audio`] — trim +
//! per-segment speed via linear-interpolation resample), and feeds it to the
//! encoder's audio scratch so the finalize remux muxes it onto the retimed
//! video. `GStreamer` owns the intake; Rust owns the edit arithmetic.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use decode::EditorVideoStream;
use edit::style::CropRect;
use edit::zoom_anim::active_zoom_at;
use edit::{EditProject, TimelineSegment};
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
        let mut preview = EditorPreview::new(project.source.width, project.source.height)
            .map_err(|e| format!("init compose: {e}"))?;
        // Background framing (ED.18) is constant across the export — set the
        // backdrop + rounded-corner clip + padding once, here, so the
        // per-frame `render_framed` only updates the zoom/crop transform.
        preview.set_background(&project.background);
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
        // Kept for the returned `source_frame` (verification); the compose
        // itself re-derives it inside `compose_project_frame`.
        let source_frame = self.project.source_time(f)?;
        let frame = compose_project_frame(&mut self.preview, &mut self.stream, &self.project, f)?;
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

/// Compose project frame `frame` into a [`ComposedFrame`] — the single
/// per-frame compose shared by the deferred **export** generator and the live
/// editor **preview** (AUT-510), so what you scrub is what you ship.
///
/// Maps the project frame to its source frame (trim / split / speed via
/// [`EditProject::source_time`]), decodes it from `stream`, then applies the
/// cinematic framing: the zoom punch-in ([`active_zoom_at`], ED.16), the crop /
/// aspect reframe (ED.15), and — when the project carries a cursor track — the
/// cursor overlay + click ripples ([`cursor_for_frame`] + `ripples_at`, ED.19).
/// Returns `None` past the end of the timeline or on a decode miss.
#[must_use]
pub fn compose_project_frame(
    preview: &mut EditorPreview,
    stream: &mut EditorVideoStream,
    project: &EditProject,
    frame: u64,
) -> Option<ComposedFrame> {
    let source_frame = project.source_time(frame)?;
    let decoded = stream.frame(source_frame)?;
    let zoom = active_zoom_at(project, frame);
    let crop = project.crop.unwrap_or_else(CropRect::full);
    if let Some(track) = project.cursor_track.as_deref() {
        let cfg = project.cursor;
        // Ripple window ≈ 0.4 s.
        let ripple_frames = (project.project_fps * 2 / 5).max(1);
        let clicks = project.clicks.as_deref().unwrap_or(&[]);
        let ripples = edit::telemetry::ripples_at(clicks, frame, ripple_frames);
        // `hide_static` fades the pointer out while parked, but a live click
        // ripple keeps it visible (see `cursor_for_frame`).
        let cursor = cursor_for_frame(track, &cfg, project.project_fps, frame, !ripples.is_empty());
        preview.render_framed_with_cursor(decoded.bgra, zoom, crop, cursor, &ripples, &cfg)
    } else {
        preview.render_framed(decoded.bgra, zoom, crop)
    }
}

/// The pointer position to draw at project `frame`, applying
/// [`CursorConfig::hide_static`](edit::style::CursorConfig::hide_static).
///
/// Returns `None` (pointer hidden) only when the cursor is parked **and** no
/// click ripple is live — a click is an action worth showing even if the
/// cursor hasn't moved (the Screen Studio rule). Otherwise the smoothed
/// position from [`cursor_at`](edit::telemetry::cursor_at). Pure, so the
/// hide-while-static decision is unit-testable without a renderer.
#[must_use]
fn cursor_for_frame(
    track: &[edit::telemetry::CursorSample],
    cfg: &edit::style::CursorConfig,
    fps: u32,
    frame: edit::segment::Frame,
    ripple_active: bool,
) -> Option<(f32, f32)> {
    if cfg.hide_static && !ripple_active && edit::telemetry::cursor_is_static(track, frame, fps) {
        None
    } else {
        edit::telemetry::cursor_at(track, frame, cfg.smoothing)
    }
}

/// Retime the source audio to the edited timeline (ED.21).
///
/// `full` is the source's interleaved F32LE audio (`channels` samples per
/// sample-frame). For each timeline segment, the source sample-frame range
/// `[source_start, source_end)` — mapped from source *video* frames via
/// `source_fps` and `sample_rate` — is resampled to the segment's **project**
/// duration: a `timescale` of `2.0` emits half as many sample-frames (sped
/// up), `0.5` twice as many (slowed). Linear interpolation between adjacent
/// sample-frames keeps it click-free; the segments are then concatenated, so
/// the result matches the retimed video frame-for-frame.
///
/// **v1 ships speed-with-pitch by design**: a sped-up segment rises in pitch,
/// a slowed one drops — the industry default for editor speed ramps (Premiere
/// / FCP / Resolve / Loom all default to it). Pitch-*preserving* retime
/// (time-stretch) is a tracked enhancement, **ISS-18**: it slots in behind
/// this exact pure `(samples, segments, fps, rate, channels) -> Vec<f32>`
/// contract, so callers and tests don't change when it lands. Pure — no gst,
/// exhaustively testable.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "audio sample counts are well under 2^52 so the u64/usize→f64 conversions are lossless; positions are clamped to a valid range before the f64→usize index cast, and the interpolation fraction is in [0, 1)"
)]
fn retime_audio(
    full: &[f32],
    segments: &[TimelineSegment],
    source_fps: u32,
    sample_rate: u32,
    channels: usize,
) -> Vec<f32> {
    if full.is_empty() || channels == 0 {
        return Vec::new();
    }
    let total_frames = full.len() / channels;
    if total_frames == 0 {
        return Vec::new();
    }
    let fps = f64::from(source_fps.max(1));
    let rate = f64::from(sample_rate.max(1));
    // Source video frame → source audio sample-frame position.
    let frame_to_sample = |frame: u64| (frame as f64) * rate / fps;
    let last = (total_frames - 1) as f64;
    // Linearly interpolate channel `ch` at sample-frame position `pos`,
    // clamped to the valid range.
    let sample_at = |pos: f64, ch: usize| -> f32 {
        let clamped = pos.clamp(0.0, last);
        let i0 = clamped.floor() as usize;
        let i1 = (i0 + 1).min(total_frames - 1);
        let frac = (clamped - i0 as f64) as f32;
        let a = full[i0 * channels + ch];
        let b = full[i1 * channels + ch];
        a + (b - a) * frac
    };

    let mut out = Vec::new();
    for seg in segments {
        let ts = if seg.timescale.is_finite() && seg.timescale > 0.0 {
            seg.timescale
        } else {
            1.0
        };
        let start = frame_to_sample(seg.source_start);
        let end = frame_to_sample(seg.source_end);
        let src_span = (end - start).max(0.0);
        if src_span <= 0.0 {
            continue;
        }
        // Project sample-frames = source span / timescale (2× → half as many).
        let out_frames = (src_span / ts).round().max(0.0) as usize;
        out.reserve(out_frames * channels);
        for j in 0..out_frames {
            let pos = start + (j as f64) * ts;
            for ch in 0..channels {
                out.push(sample_at(pos, ch));
            }
        }
    }
    out
}

/// Export an entire edited project to a single video file (ED.21).
///
/// Drives the [`ExportFrameGenerator`] frame by frame into a fresh
/// [`LiveGstreamerEncoder`] (reused unchanged from the live recorder), then
/// finalizes the container. Synchronous and long-running — call it from a
/// blocking task. `cancel` is polled once per frame (for the export UI,
/// ED.22) and `on_progress(done, total)` reports after each frame.
///
/// The output is composed at the **source** clip's dimensions. The zoom
/// punch-ins (ED.16), crop / aspect reframe (ED.15), and background framing
/// (ED.18 — backdrop, padding, rounded corners) are all baked into each
/// frame by the generator; the per-segment audio retime (ED.21) muxes on at
/// finalize. Drop-shadow + inset border are deferred (ISS-14).
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

    // Capture the audio-retime inputs before `project` moves into the
    // generator (ED.21): the segment list + the source recording's fps map
    // source frames → audio sample positions.
    let segments = project.segments.clone();
    let source_fps = project.source.source_fps;
    let (sample_rate, channels) = (config.sample_rate, config.channels);

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

    // ED.21 audio: decode the source's audio once, retime it to the edited
    // timeline in pure Rust, and feed it to the encoder's audio scratch so the
    // finalize remux muxes it. The video already succeeded — an audio decode
    // failure falls back to a video-only export rather than sinking it.
    match media::encode::decode_source_audio_f32(source, sample_rate, channels) {
        Ok(samples) if !samples.is_empty() => {
            let retimed = retime_audio(
                &samples,
                &segments,
                source_fps,
                sample_rate,
                usize::from(channels),
            );
            if !retimed.is_empty() {
                encoder
                    .push_audio_chunk(&retimed, Duration::ZERO)
                    .map_err(|e| format!("push edited audio: {e}"))?;
            }
        }
        Ok(_) => { /* source has no audio track — video-only export */ }
        Err(e) => {
            tracing::warn!(error = %e, "edited audio decode failed — exporting video-only");
        }
    }

    Box::new(encoder)
        .finalize()
        .map_err(|e| format!("finalize export: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 10 audio sample-frames per video frame (rate 300 / fps 30) keeps the
    // arithmetic exact and the fixtures small.
    const FPS: u32 = 30;
    const RATE: u32 = 300;

    /// Mono ramp: sample-frame `i` holds the value `i` (built without an
    /// integer→float cast so it stays clippy-clean).
    fn mono_ramp(frames: usize) -> Vec<f32> {
        (0..frames)
            .scan(0.0f32, |acc, _| {
                let v = *acc;
                *acc += 1.0;
                Some(v)
            })
            .collect()
    }

    #[test]
    fn retime_audio_empty_or_degenerate_is_empty() {
        let seg = TimelineSegment::new(0, 30);
        assert!(retime_audio(&[], &[seg], FPS, RATE, 1).is_empty());
        assert!(retime_audio(&mono_ramp(300), &[seg], FPS, RATE, 0).is_empty());
        // No segments → nothing to emit.
        assert!(retime_audio(&mono_ramp(300), &[], FPS, RATE, 1).is_empty());
    }

    #[test]
    fn retime_audio_realtime_preserves_length_and_content() {
        // Source: 30 frames → 300 sample-frames. A single real-time segment
        // over the whole clip reproduces it exactly.
        let full = mono_ramp(300);
        let out = retime_audio(&full, &[TimelineSegment::new(0, 30)], FPS, RATE, 1);
        assert_eq!(out.len(), 300);
        assert!((out[0] - 0.0).abs() < 1e-3);
        assert!((out[100] - 100.0).abs() < 1e-3);
        assert!((out[299] - 299.0).abs() < 1e-3);
    }

    /// The **speed-with-pitch contract** (ED.21, ISS-18): a 2× segment emits
    /// half the sample-frames *and* strides the source 2× — which is exactly
    /// what raises the pitch with the tempo. This is the deliberate v1
    /// behavior; a pitch-preserving time-stretch would keep `out[1]` near the
    /// *unstrided* source while still halving the length. Asserting the stride
    /// pins the contract so a future ISS-18 change is a conscious one.
    #[test]
    fn retime_audio_double_speed_shifts_pitch_with_tempo() {
        let full = mono_ramp(300);
        let out = retime_audio(
            &[full.as_slice()].concat(),
            &[TimelineSegment::with_speed(0, 30, 2.0)],
            FPS,
            RATE,
            1,
        );
        // 300 source sample-frames / 2.0 → 150 (tempo doubled).
        assert_eq!(out.len(), 150);
        // Output strides the source 2× (pitch rises with tempo): out[1] samples
        // source position 2, out[10] position 20 — NOT 1 and 10.
        assert!((out[1] - 2.0).abs() < 1e-3, "2× stride → pitch shift");
        assert!((out[10] - 20.0).abs() < 1e-3);
    }

    #[test]
    fn retime_audio_trim_selects_the_subrange() {
        // Trim to source frames [10, 20) → sample-frames [100, 200).
        let full = mono_ramp(300);
        let out = retime_audio(&full, &[TimelineSegment::new(10, 20)], FPS, RATE, 1);
        assert_eq!(out.len(), 100);
        assert!(
            (out[0] - 100.0).abs() < 1e-3,
            "starts at the trimmed in-point"
        );
        assert!((out[99] - 199.0).abs() < 1e-3);
    }

    #[test]
    fn retime_audio_concatenates_segments() {
        // Two real-time slices [0,10) + [20,30) → 100 + 100 sample-frames.
        let full = mono_ramp(300);
        let out = retime_audio(
            &full,
            &[TimelineSegment::new(0, 10), TimelineSegment::new(20, 30)],
            FPS,
            RATE,
            1,
        );
        assert_eq!(out.len(), 200);
        assert!((out[0] - 0.0).abs() < 1e-3, "first slice starts at 0");
        assert!(
            (out[100] - 200.0).abs() < 1e-3,
            "second slice jumps to sample-frame 200"
        );
    }

    #[test]
    fn retime_audio_preserves_stereo_interleave() {
        // Stereo: left = i, right = 1000 + i, interleaved (no int→float cast).
        let frames = 300usize;
        let mut full = Vec::with_capacity(frames * 2);
        let mut v = 0.0f32;
        for _ in 0..frames {
            full.push(v);
            full.push(1000.0 + v);
            v += 1.0;
        }
        let out = retime_audio(&full, &[TimelineSegment::new(0, 30)], FPS, RATE, 2);
        assert_eq!(out.len(), 600);
        assert!((out[0] - 0.0).abs() < 1e-3, "L0");
        assert!((out[1] - 1000.0).abs() < 1e-3, "R0");
        assert!((out[2] - 1.0).abs() < 1e-3, "L1");
        assert!((out[3] - 1001.0).abs() < 1e-3, "R1");
    }

    #[test]
    fn cursor_for_frame_hides_a_parked_pointer_unless_clicked() {
        use edit::style::CursorConfig;
        use edit::telemetry::CursorSample;

        // A pointer that hasn't emitted a new sample for the whole ~0.4 s
        // window is parked.
        let parked = [CursorSample::new(0, 0.5, 0.5)];
        let cfg = CursorConfig::default(); // hide_static = true
        assert!(
            cursor_for_frame(&parked, &cfg, 30, 100, false).is_none(),
            "parked + no ripple ⇒ hidden"
        );
        // A live click ripple keeps the parked pointer visible.
        assert!(
            cursor_for_frame(&parked, &cfg, 30, 100, true).is_some(),
            "parked + active ripple ⇒ shown"
        );
        // With hide_static off, a parked pointer always shows.
        let mut on = cfg;
        on.hide_static = false;
        assert!(cursor_for_frame(&parked, &on, 30, 100, false).is_some());
        // A moving pointer always shows, ripple or not.
        let moving = [
            CursorSample::new(88, 0.0, 0.0),
            CursorSample::new(100, 1.0, 1.0),
        ];
        assert!(cursor_for_frame(&moving, &cfg, 30, 100, false).is_some());
    }
}
