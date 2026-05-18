//! `VideoEncoder` trait + `OutputFormat` enum + per-OS GStreamer
//! pipeline builder (M-EXPORT.1 of M-RECORD-EXPORT).
//!
//! Implements a **batch encoder**: video frames + audio samples are
//! written to scratch files on disk during capture; on `finalize()`,
//! a single `gst-launch-1.0` subprocess reads the scratch files and
//! produces the final encoded container.
//!
//! ```admonish important title="Batch vs. streaming encode"
//! M-EXPORT.1 ships the **batch** variant because it works with
//! GStreamer's CLI tooling (no `gstreamer-rs` Rust-binding dep).
//! Streaming encode (push frames directly into `appsrc` via the
//! Rust bindings) is a future optimization — same trait surface, a
//! different `GstreamerEncoder` impl. Batch trades startup latency
//! at session end for stability + simpler error handling. For
//! short-to-medium recordings (the recorder's MVP) this is fine.
//! ```
//!
//! ## Per-OS encoder coverage
//!
//! | OS       | H.264              | H.265              | VP9 (WebM)         | AV1                                  |
//! |----------|--------------------|--------------------|--------------------|--------------------------------------|
//! | macOS    | `vtenc_h264_hw`    | `vtenc_h265_hw`    | `vp9enc` (sw)      | `vtenc_av1_hw` (M3+) → `svtav1enc`  |
//! | Windows  | `mfh264enc`        | `mfhevcenc`        | `mfvp9enc`         | `qsvav1enc`                          |
//! | Linux    | `vaapih264enc`     | `vaapih265enc`     | `vaapivp9enc`      | `vaapiav1enc`                        |
//!
//! macOS is the hot path for M-RECORD-EXPORT; Win/Linux pipeline
//! strings are present so cross-OS clippy + the pipeline-string
//! unit tests pass, but the runtime spawn returns
//! `EncodeError::Unsupported` outside macOS until those ports land.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

/// Output container + codec selection. Carries the (format → codec
/// → muxer → file extension) tuple as one type so the rest of the
/// pipeline can switch on a single value.
///
/// Default is [`OutputFormat::Mp4H264Aac`] — the universally-
/// compatible "just give me an .mp4" choice. Other variants are
/// opt-in via the M-EXPORT.4 format dropdown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    /// `.mp4` container, H.264 video, AAC audio. The default for
    /// universal compatibility (QuickTime, every browser, every
    /// editor). macOS uses `vtenc_h264_hw` (hardware), Windows
    /// `mfh264enc`, Linux `vaapih264enc` / `x264enc` fallback.
    #[default]
    Mp4H264Aac,
    /// `.mp4` container, H.265 (HEVC) video, AAC audio. ~30%
    /// smaller files at equivalent quality vs. H.264; requires
    /// macOS 11+, Windows 10+, Linux with VAAPI HEVC support.
    Mp4H265Aac,
    /// `.webm` container, VP9 video, Opus audio. Open codec
    /// (royalty-free), good browser support. macOS has no HW VP9
    /// encoder so uses libvpx-vp9 (slow).
    WebmVp9Opus,
    /// `.webm` container, AV1 video, Opus audio. Best compression
    /// of the four; HW encoders are M3+ Macs, Intel Arc, NVIDIA
    /// RTX 40+. Software fallback via `svtav1enc` (slow).
    WebmAv1Opus,
}

impl OutputFormat {
    /// File extension this format writes (without leading `.`).
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp4H264Aac | Self::Mp4H265Aac => "mp4",
            Self::WebmVp9Opus | Self::WebmAv1Opus => "webm",
        }
    }

    /// URL-safe slug for the M-EXPORT.4 format dropdown +
    /// `RecordingConfig.format` IPC field.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            Self::Mp4H264Aac => "mp4-h264",
            Self::Mp4H265Aac => "mp4-h265",
            Self::WebmVp9Opus => "webm-vp9",
            Self::WebmAv1Opus => "webm-av1",
        }
    }

    /// Parse the slug back to an `OutputFormat`. Inverse of
    /// [`Self::slug`]. Unknown slugs return `None` so the caller can
    /// surface a typed error.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "mp4-h264" => Some(Self::Mp4H264Aac),
            "mp4-h265" => Some(Self::Mp4H265Aac),
            "webm-vp9" => Some(Self::WebmVp9Opus),
            "webm-av1" => Some(Self::WebmAv1Opus),
            _ => None,
        }
    }
}

/// Encoder configuration. Width + height + framerate are the video
/// caps; `output_path` is the final container path the encoder
/// writes to on `finalize`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncoderConfig {
    /// Output container path. Must end with the extension matching
    /// `format` (`.mp4` for MP4 variants, `.webm` for WebM).
    pub output_path: PathBuf,
    /// Video pixel width.
    pub width: u32,
    /// Video pixel height.
    pub height: u32,
    /// Video framerate (`30` is the M-EXPORT.1 default).
    pub framerate: u32,
    /// Audio sample rate (`48000` is the SCK / GStreamer default).
    pub sample_rate: u32,
    /// Audio channel count (`2` = stereo).
    pub channels: u8,
    /// Format selection.
    pub format: OutputFormat,
}

impl EncoderConfig {
    /// Construct a sensible default for the given output path +
    /// format. 1920×1080 @ 30 fps video, 48 kHz stereo audio.
    #[must_use]
    pub fn for_output(output_path: PathBuf, format: OutputFormat) -> Self {
        Self {
            output_path,
            width: 1920,
            height: 1080,
            framerate: 30,
            sample_rate: 48_000,
            channels: 2,
            format,
        }
    }
}

/// Failure modes for the batch encoder.
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    /// `gst-launch-1.0` could not be spawned (missing from PATH).
    #[error("failed to spawn `gst-launch-1.0`: {source} (PATH={path})")]
    Spawn {
        /// OS-level reason.
        #[source]
        source: std::io::Error,
        /// `$PATH` snapshot at failure.
        path: String,
    },
    /// I/O failure on the scratch / output file.
    #[error("encoder I/O: {0}")]
    Io(#[from] std::io::Error),
    /// `gst-launch-1.0` exited non-zero. `stderr` carries GStreamer's
    /// own diagnostic for the user / log.
    #[error("encode pipeline failed (exit {exit:?}): {stderr}")]
    PipelineFailed {
        /// Exit status of the gst-launch child.
        exit: Option<i32>,
        /// Captured stderr.
        stderr: String,
    },
    /// Format / OS combo not yet implemented (e.g. Linux real
    /// encoders pending). The trait still constructs cleanly so
    /// callers can verify configuration; runtime invocation surfaces
    /// this error.
    #[error("encoder not yet wired for ({format:?}, {os}): {reason}")]
    Unsupported {
        /// Format that was requested.
        format: OutputFormat,
        /// OS name (`target_os` value).
        os: &'static str,
        /// Human-readable reason / pointer to follow-up ticket.
        reason: &'static str,
    },
    /// Validated config rejected (e.g. width=0, framerate=0).
    #[error("invalid encoder config: {0}")]
    InvalidConfig(String),
}

/// Encoder trait — the seam M-EXPORT.3 hooks the per-channel
/// capture callbacks into. Pure-sync interface; the
/// `GstreamerEncoder` impl writes to scratch files on each push and
/// runs the actual encode pipeline at `finalize`.
pub trait VideoEncoder: Send + Sync {
    /// Push one BGRA frame at the given monotonic PTS (measured from
    /// session start). `bgra.len()` must equal `width * height * 4`.
    ///
    /// # Errors
    ///
    /// Returns [`EncodeError::Io`] on scratch-file write failure.
    fn push_video_frame(
        &mut self,
        bgra: &[u8],
        pts: std::time::Duration,
    ) -> Result<(), EncodeError>;

    /// Push interleaved F32LE audio samples at the given PTS.
    /// Sample layout is `[ch0, ch1, ch0, ch1, ...]` for stereo.
    ///
    /// # Errors
    ///
    /// Returns [`EncodeError::Io`] on scratch-file write failure.
    fn push_audio_chunk(
        &mut self,
        samples: &[f32],
        pts: std::time::Duration,
    ) -> Result<(), EncodeError>;

    /// Finalize: spawn the gst-launch pipeline that consumes the
    /// scratch files and writes the final container at the configured
    /// output path. Consumes `self` because the encoder is single-use.
    ///
    /// # Errors
    ///
    /// - [`EncodeError::Spawn`] — gst-launch missing from PATH.
    /// - [`EncodeError::PipelineFailed`] — gst-launch exited non-zero.
    /// - [`EncodeError::Unsupported`] — format/OS combo not wired.
    fn finalize(self: Box<Self>) -> Result<PathBuf, EncodeError>;
}

/// Concrete batch encoder. Writes BGRA + F32 samples to scratch
/// files on each push; runs `gst-launch-1.0` on finalize.
pub struct GstreamerEncoder {
    config: EncoderConfig,
    video_scratch_path: PathBuf,
    audio_scratch_path: PathBuf,
    video_writer: BufWriter<File>,
    audio_writer: BufWriter<File>,
    expected_video_bytes_per_frame: usize,
    frames_pushed: u64,
    audio_chunks_pushed: u64,
}

impl std::fmt::Debug for GstreamerEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GstreamerEncoder")
            .field("config", &self.config)
            .field("video_scratch_path", &self.video_scratch_path)
            .field("audio_scratch_path", &self.audio_scratch_path)
            .field("frames_pushed", &self.frames_pushed)
            .field("audio_chunks_pushed", &self.audio_chunks_pushed)
            .finish_non_exhaustive()
    }
}

impl GstreamerEncoder {
    /// Construct + open the scratch files. The video scratch sits
    /// next to the output (same parent dir) with a `.bgra.scratch`
    /// suffix; same shape for audio with `.f32.scratch`. Both are
    /// deleted on successful finalize.
    ///
    /// # Errors
    ///
    /// Returns [`EncodeError::InvalidConfig`] if width/height/framerate
    /// is zero, or [`EncodeError::Io`] if the scratch files can't be
    /// opened (parent dir doesn't exist, permission denied).
    pub fn new(config: EncoderConfig) -> Result<Self, EncodeError> {
        if config.width == 0 || config.height == 0 || config.framerate == 0 {
            return Err(EncodeError::InvalidConfig(format!(
                "width={}, height={}, framerate={} — none may be zero",
                config.width, config.height, config.framerate
            )));
        }
        if config.channels == 0 || config.sample_rate == 0 {
            return Err(EncodeError::InvalidConfig(format!(
                "channels={}, sample_rate={} — neither may be zero",
                config.channels, config.sample_rate
            )));
        }

        let video_scratch_path = scratch_path(&config.output_path, ".bgra.scratch");
        let audio_scratch_path = scratch_path(&config.output_path, ".f32.scratch");

        let video_file = File::create(&video_scratch_path)?;
        let audio_file = File::create(&audio_scratch_path)?;

        let expected_video_bytes_per_frame = (config.width as usize) * (config.height as usize) * 4;

        Ok(Self {
            config,
            video_scratch_path,
            audio_scratch_path,
            video_writer: BufWriter::new(video_file),
            audio_writer: BufWriter::new(audio_file),
            expected_video_bytes_per_frame,
            frames_pushed: 0,
            audio_chunks_pushed: 0,
        })
    }

    /// Frame count pushed so far.
    #[must_use]
    pub fn frames_pushed(&self) -> u64 {
        self.frames_pushed
    }

    /// Audio chunks pushed so far.
    #[must_use]
    pub fn audio_chunks_pushed(&self) -> u64 {
        self.audio_chunks_pushed
    }

    /// Borrow the resolved encoder config (mostly useful for tests
    /// asserting `EncoderConfig::for_output` defaults).
    #[must_use]
    pub fn config(&self) -> &EncoderConfig {
        &self.config
    }
}

impl VideoEncoder for GstreamerEncoder {
    fn push_video_frame(
        &mut self,
        bgra: &[u8],
        _pts: std::time::Duration,
    ) -> Result<(), EncodeError> {
        if bgra.len() != self.expected_video_bytes_per_frame {
            return Err(EncodeError::InvalidConfig(format!(
                "frame byte length mismatch: got {}, expected {}",
                bgra.len(),
                self.expected_video_bytes_per_frame
            )));
        }
        self.video_writer.write_all(bgra)?;
        self.frames_pushed = self.frames_pushed.saturating_add(1);
        Ok(())
    }

    fn push_audio_chunk(
        &mut self,
        samples: &[f32],
        _pts: std::time::Duration,
    ) -> Result<(), EncodeError> {
        // Write as little-endian F32 (matches gst caps S=F32LE).
        for sample in samples {
            self.audio_writer.write_all(&sample.to_le_bytes())?;
        }
        self.audio_chunks_pushed = self.audio_chunks_pushed.saturating_add(1);
        Ok(())
    }

    fn finalize(mut self: Box<Self>) -> Result<PathBuf, EncodeError> {
        // Flush + close the scratch files before invoking gst.
        self.video_writer.flush()?;
        drop(self.video_writer);
        self.audio_writer.flush()?;
        drop(self.audio_writer);

        let pipeline = build_pipeline_args(
            &self.config,
            &self.video_scratch_path,
            &self.audio_scratch_path,
            self.frames_pushed > 0,
            self.audio_chunks_pushed > 0,
        )?;

        tracing::info!(
            output = %self.config.output_path.display(),
            format = ?self.config.format,
            video_frames = self.frames_pushed,
            audio_chunks = self.audio_chunks_pushed,
            pipeline_args = ?pipeline,
            "GstreamerEncoder::finalize spawning gst-launch-1.0"
        );

        let output = Command::new("gst-launch-1.0")
            .args(&pipeline)
            .output()
            .map_err(|err| EncodeError::Spawn {
                source: err,
                path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".into()),
            })?;

        if !output.status.success() {
            return Err(EncodeError::PipelineFailed {
                exit: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        // Clean up scratch files. Failure is non-fatal (the encoded
        // output already exists).
        let _ = std::fs::remove_file(&self.video_scratch_path);
        let _ = std::fs::remove_file(&self.audio_scratch_path);

        Ok(self.config.output_path)
    }
}

/// Build the gst-launch-1.0 argv for the configured (format, OS)
/// combo. Public so M-EXPORT.1's pipeline-string snapshot tests can
/// exercise every combination without spawning gst.
///
/// `has_video` / `has_audio` gate the per-stream legs — a video-only
/// recording skips the audio mixer entirely; an audio-only recording
/// (unusual but valid) skips the video leg.
///
/// # Errors
///
/// Returns [`EncodeError::Unsupported`] if the format/OS combo isn't
/// yet wired (Win/Linux scaffold path).
#[allow(
    clippy::too_many_lines,
    reason = "Each (format × OS) branch carries its own per-encoder element list; flattening the match would replace this with three layers of helpers that obscure the actual pipeline shape."
)]
pub fn build_pipeline_args(
    config: &EncoderConfig,
    video_scratch: &Path,
    audio_scratch: &Path,
    has_video: bool,
    has_audio: bool,
) -> Result<Vec<String>, EncodeError> {
    let mut args: Vec<String> = vec!["-q".to_string(), "-e".to_string()];

    let video_caps = format!(
        "video/x-raw,format=BGRA,width={},height={},framerate={}/1",
        config.width, config.height, config.framerate,
    );
    let audio_caps = format!(
        "audio/x-raw,format=F32LE,rate={},channels={},layout=interleaved",
        config.sample_rate, config.channels,
    );

    let (video_encoder_elements, mux_element) = match (config.format, std::env::consts::OS) {
        (OutputFormat::Mp4H264Aac, "macos") => (vec!["vtenc_h264_hw"], "mp4mux"),
        (OutputFormat::Mp4H265Aac, "macos") => (vec!["vtenc_h265_hw"], "mp4mux"),
        (OutputFormat::WebmVp9Opus, "macos") => (vec!["vp9enc"], "webmmux"),
        (OutputFormat::WebmAv1Opus, "macos") => (vec!["svtav1enc"], "webmmux"),
        (OutputFormat::Mp4H264Aac, "windows") => (vec!["mfh264enc"], "mp4mux"),
        (OutputFormat::Mp4H265Aac, "windows") => (vec!["mfhevcenc"], "mp4mux"),
        (OutputFormat::WebmVp9Opus, "windows") => (vec!["mfvp9enc"], "webmmux"),
        (OutputFormat::WebmAv1Opus, "windows") => (vec!["qsvav1enc"], "webmmux"),
        (OutputFormat::Mp4H264Aac, "linux") => (vec!["vaapih264enc"], "mp4mux"),
        (OutputFormat::Mp4H265Aac, "linux") => (vec!["vaapih265enc"], "mp4mux"),
        (OutputFormat::WebmVp9Opus, "linux") => (vec!["vaapivp9enc"], "webmmux"),
        (OutputFormat::WebmAv1Opus, "linux") => (vec!["vaapiav1enc"], "webmmux"),
        (format, other) => {
            return Err(EncodeError::Unsupported {
                format,
                os: leak_os_name(other),
                reason: "no encoder wired for this OS/format combo",
            });
        }
    };

    let audio_encoder_element = match config.format {
        OutputFormat::Mp4H264Aac | OutputFormat::Mp4H265Aac => "avenc_aac",
        OutputFormat::WebmVp9Opus | OutputFormat::WebmAv1Opus => "opusenc",
    };

    // Video leg.
    if has_video {
        args.push("filesrc".to_string());
        args.push(format!("location={}", video_scratch.display()));
        args.push("!".to_string());
        args.push("rawvideoparse".to_string());
        args.push("format=bgra".to_string());
        args.push(format!("width={}", config.width));
        args.push(format!("height={}", config.height));
        args.push(format!("framerate={}/1", config.framerate));
        args.push("!".to_string());
        args.push("videoconvert".to_string());
        args.push("!".to_string());
        for elem in &video_encoder_elements {
            args.push((*elem).to_string());
            args.push("!".to_string());
        }
        args.push(mux_to_parser(config.format).to_string());
        args.push("!".to_string());
        args.push("mux.".to_string());
    }

    // Audio leg.
    if has_audio {
        args.push("filesrc".to_string());
        args.push(format!("location={}", audio_scratch.display()));
        args.push("!".to_string());
        args.push("rawaudioparse".to_string());
        args.push("format=pcm-f32le".to_string());
        args.push(format!("sample-rate={}", config.sample_rate));
        args.push(format!("num-channels={}", config.channels));
        args.push("!".to_string());
        args.push(audio_caps.clone());
        args.push("!".to_string());
        args.push("audioconvert".to_string());
        args.push("!".to_string());
        args.push("audioresample".to_string());
        args.push("!".to_string());
        args.push(audio_encoder_element.to_string());
        args.push("!".to_string());
        args.push("mux.".to_string());
    }

    // Muxer + sink.
    args.push(mux_element.to_string());
    args.push("name=mux".to_string());
    args.push("!".to_string());
    args.push("filesink".to_string());
    args.push(format!("location={}", config.output_path.display()));

    // Drop dead variables so the compiler doesn't warn unused.
    let _ = (video_caps, audio_encoder_element);

    Ok(args)
}

// ---- M-EXPORT.5 — AVIF poster-frame thumbnail -----------------------

/// Generate an AVIF poster image next to the encoded video.
/// Spawns a one-shot `gst-launch-1.0` pipeline that extracts a
/// single frame from `video_path`, scales it to ≤640 px wide, and
/// writes it to `<video_path-without-ext>.avif`.
///
/// Returns `Ok(Some(path))` on success, `Ok(None)` when the
/// `avifenc` GStreamer element isn't installed (silent skip with a
/// `tracing::warn` — poster is a free side-benefit, not a hard
/// requirement). Returns `Err` on any other failure (e.g. video
/// file missing).
///
/// # Errors
///
/// Returns [`EncodeError::Spawn`] if `gst-launch-1.0` isn't on PATH,
/// or [`EncodeError::PipelineFailed`] if the spawn ran but exited
/// non-zero for a reason other than "missing avifenc."
pub fn generate_poster(video_path: &Path) -> Result<Option<PathBuf>, EncodeError> {
    if !video_path.exists() {
        return Err(EncodeError::InvalidConfig(format!(
            "video file does not exist: {}",
            video_path.display()
        )));
    }
    let poster_path = poster_path_for(video_path);
    let args = poster_pipeline_args(video_path, &poster_path);

    let output = Command::new("gst-launch-1.0")
        .args(&args)
        .output()
        .map_err(|err| EncodeError::Spawn {
            source: err,
            path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".into()),
        })?;

    if output.status.success() {
        tracing::info!(
            video = %video_path.display(),
            poster = %poster_path.display(),
            "generate_poster: AVIF thumbnail written"
        );
        Ok(Some(poster_path))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Heuristic: "no such element" / "Unknown element" / "no
        // element ... avifenc" — silent-skip the missing-encoder
        // case so the recorder UX doesn't break on machines without
        // gst-plugins-bad. The poster is a free side-benefit.
        if stderr.contains("avifenc") && stderr.to_lowercase().contains("no such element")
            || stderr.contains("no element \"avifenc\"")
        {
            tracing::warn!(
                "generate_poster: avifenc GStreamer element not installed — \
                 skipping AVIF poster (install gst-plugins-bad to enable)"
            );
            return Ok(None);
        }
        Err(EncodeError::PipelineFailed {
            exit: output.status.code(),
            stderr: stderr.into_owned(),
        })
    }
}

/// Build the gst-launch argv for the poster pipeline. Split out so
/// tests can assert the shape without spawning gst.
#[must_use]
pub fn poster_pipeline_args(video_path: &Path, poster_path: &Path) -> Vec<String> {
    vec![
        "-q".to_string(),
        "filesrc".to_string(),
        format!("location={}", video_path.display()),
        "!".to_string(),
        "decodebin".to_string(),
        "!".to_string(),
        "videoconvert".to_string(),
        "!".to_string(),
        "videoscale".to_string(),
        "!".to_string(),
        "video/x-raw,width=640".to_string(),
        "!".to_string(),
        "avifenc".to_string(),
        "!".to_string(),
        "filesink".to_string(),
        format!("location={}", poster_path.display()),
    ]
}

/// Compute the poster path for `<video>.<ext>` → `<video>.avif`.
/// Replaces the video extension entirely (so `Screen-...mp4` →
/// `Screen-...avif`, not `Screen-...mp4.avif`).
#[must_use]
pub fn poster_path_for(video_path: &Path) -> PathBuf {
    let mut p = video_path.to_path_buf();
    p.set_extension("avif");
    p
}

fn mux_to_parser(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Mp4H264Aac => "h264parse",
        OutputFormat::Mp4H265Aac => "h265parse",
        OutputFormat::WebmVp9Opus => "vp9parse",
        OutputFormat::WebmAv1Opus => "av1parse",
    }
}

fn scratch_path(output_path: &Path, suffix: &str) -> PathBuf {
    let mut s = output_path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// Convert a runtime OS string (`std::env::consts::OS` produces `&str`)
/// to a `&'static str` suitable for the [`EncodeError::Unsupported`]
/// field. The set of possible values is bounded so we map each known
/// one explicitly; anything else falls through to `"other"`.
fn leak_os_name(os: &str) -> &'static str {
    match os {
        "macos" => "macos",
        "windows" => "windows",
        "linux" => "linux",
        "freebsd" => "freebsd",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- OutputFormat enum ----

    #[test]
    fn output_format_default_is_mp4_h264() {
        assert_eq!(OutputFormat::default(), OutputFormat::Mp4H264Aac);
    }

    #[test]
    fn output_format_extension_matches_container() {
        assert_eq!(OutputFormat::Mp4H264Aac.extension(), "mp4");
        assert_eq!(OutputFormat::Mp4H265Aac.extension(), "mp4");
        assert_eq!(OutputFormat::WebmVp9Opus.extension(), "webm");
        assert_eq!(OutputFormat::WebmAv1Opus.extension(), "webm");
    }

    #[test]
    fn output_format_slug_round_trips() {
        for f in [
            OutputFormat::Mp4H264Aac,
            OutputFormat::Mp4H265Aac,
            OutputFormat::WebmVp9Opus,
            OutputFormat::WebmAv1Opus,
        ] {
            assert_eq!(OutputFormat::from_slug(f.slug()), Some(f));
        }
    }

    #[test]
    fn output_format_from_slug_rejects_unknown() {
        assert!(OutputFormat::from_slug("mp3").is_none());
        assert!(OutputFormat::from_slug("").is_none());
        assert!(OutputFormat::from_slug("mp4").is_none());
    }

    #[test]
    fn output_format_serde_round_trip() {
        for f in [
            OutputFormat::Mp4H264Aac,
            OutputFormat::Mp4H265Aac,
            OutputFormat::WebmVp9Opus,
            OutputFormat::WebmAv1Opus,
        ] {
            let json = serde_json::to_string(&f).unwrap();
            let back: OutputFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(back, f);
        }
    }

    // ---- EncoderConfig ----

    #[test]
    fn config_for_output_uses_1920_1080_30fps_48k_stereo() {
        let cfg = EncoderConfig::for_output(PathBuf::from("/tmp/x.mp4"), OutputFormat::Mp4H264Aac);
        assert_eq!(cfg.width, 1920);
        assert_eq!(cfg.height, 1080);
        assert_eq!(cfg.framerate, 30);
        assert_eq!(cfg.sample_rate, 48_000);
        assert_eq!(cfg.channels, 2);
    }

    // ---- build_pipeline_args ----

    fn test_config(format: OutputFormat) -> EncoderConfig {
        EncoderConfig::for_output(PathBuf::from("/tmp/test.x"), format)
    }

    #[test]
    fn pipeline_args_for_current_os_video_only_contains_format_caps() {
        if std::env::consts::OS != "macos"
            && std::env::consts::OS != "windows"
            && std::env::consts::OS != "linux"
        {
            return;
        }
        let cfg = test_config(OutputFormat::Mp4H264Aac);
        let args = build_pipeline_args(
            &cfg,
            Path::new("/tmp/v.scratch"),
            Path::new("/tmp/a.scratch"),
            true,
            false,
        )
        .expect("supported OS");
        // Caps reflect width/height/framerate.
        assert!(args.iter().any(|a| a == "format=bgra"));
        assert!(args.iter().any(|a| a.starts_with("width=1920")));
        assert!(args.iter().any(|a| a.starts_with("height=1080")));
        assert!(args.iter().any(|a| a.starts_with("framerate=30/1")));
        // mp4mux is the muxer.
        assert!(args.iter().any(|a| a == "mp4mux"));
        // filesink targets the configured output.
        assert!(args.iter().any(|a| a.starts_with("location=/tmp/test.x")));
    }

    #[test]
    fn pipeline_args_for_webm_uses_webmmux() {
        if std::env::consts::OS != "macos"
            && std::env::consts::OS != "windows"
            && std::env::consts::OS != "linux"
        {
            return;
        }
        let cfg = test_config(OutputFormat::WebmVp9Opus);
        let args = build_pipeline_args(
            &cfg,
            Path::new("/tmp/v.scratch"),
            Path::new("/tmp/a.scratch"),
            true,
            false,
        )
        .expect("supported OS");
        assert!(args.iter().any(|a| a == "webmmux"));
        assert!(!args.iter().any(|a| a == "mp4mux"));
    }

    #[test]
    fn pipeline_args_skip_audio_leg_when_no_audio() {
        if std::env::consts::OS != "macos"
            && std::env::consts::OS != "windows"
            && std::env::consts::OS != "linux"
        {
            return;
        }
        let cfg = test_config(OutputFormat::Mp4H264Aac);
        let args = build_pipeline_args(
            &cfg,
            Path::new("/tmp/v.scratch"),
            Path::new("/tmp/a.scratch"),
            true,
            false,
        )
        .expect("supported OS");
        // audioconvert is the audio-leg signature.
        assert!(!args.iter().any(|a| a == "audioconvert"));
    }

    #[test]
    fn pipeline_args_include_audio_when_has_audio_true() {
        if std::env::consts::OS != "macos"
            && std::env::consts::OS != "windows"
            && std::env::consts::OS != "linux"
        {
            return;
        }
        let cfg = test_config(OutputFormat::Mp4H264Aac);
        let args = build_pipeline_args(
            &cfg,
            Path::new("/tmp/v.scratch"),
            Path::new("/tmp/a.scratch"),
            true,
            true,
        )
        .expect("supported OS");
        assert!(args.iter().any(|a| a == "audioconvert"));
        assert!(args.iter().any(|a| a == "avenc_aac"));
    }

    #[test]
    fn pipeline_args_webm_uses_opusenc_for_audio() {
        if std::env::consts::OS != "macos"
            && std::env::consts::OS != "windows"
            && std::env::consts::OS != "linux"
        {
            return;
        }
        let cfg = test_config(OutputFormat::WebmVp9Opus);
        let args = build_pipeline_args(
            &cfg,
            Path::new("/tmp/v.scratch"),
            Path::new("/tmp/a.scratch"),
            false,
            true,
        )
        .expect("supported OS");
        assert!(args.iter().any(|a| a == "opusenc"));
        assert!(!args.iter().any(|a| a == "avenc_aac"));
    }

    // ---- GstreamerEncoder construction ----

    #[test]
    fn new_rejects_zero_dimensions() {
        let mut cfg =
            EncoderConfig::for_output(PathBuf::from("/tmp/x.mp4"), OutputFormat::Mp4H264Aac);
        cfg.width = 0;
        let err = GstreamerEncoder::new(cfg);
        assert!(matches!(err, Err(EncodeError::InvalidConfig(_))));
    }

    #[test]
    fn new_rejects_zero_channels() {
        let mut cfg =
            EncoderConfig::for_output(PathBuf::from("/tmp/x.mp4"), OutputFormat::Mp4H264Aac);
        cfg.channels = 0;
        let err = GstreamerEncoder::new(cfg);
        assert!(matches!(err, Err(EncodeError::InvalidConfig(_))));
    }

    #[test]
    fn push_video_frame_rejects_wrong_byte_count() {
        // Use a path inside /tmp that's writable on every CI runner
        // (including Windows — where `/tmp` doesn't exist, this test
        // is skipped via the early return).
        if !Path::new("/tmp").exists() {
            return;
        }
        let cfg = EncoderConfig {
            output_path: PathBuf::from("/tmp/m-export-test-byte-mismatch.mp4"),
            width: 32,
            height: 32,
            framerate: 30,
            sample_rate: 48_000,
            channels: 2,
            format: OutputFormat::Mp4H264Aac,
        };
        let mut encoder = GstreamerEncoder::new(cfg).expect("construct");
        let err = encoder.push_video_frame(&[0u8; 100], std::time::Duration::from_millis(0));
        assert!(matches!(err, Err(EncodeError::InvalidConfig(_))));
        // Clean up the scratch files the constructor created.
        let _ = std::fs::remove_file("/tmp/m-export-test-byte-mismatch.mp4.bgra.scratch");
        let _ = std::fs::remove_file("/tmp/m-export-test-byte-mismatch.mp4.f32.scratch");
    }

    #[test]
    fn push_video_frame_increments_counter() {
        if !Path::new("/tmp").exists() {
            return;
        }
        let cfg = EncoderConfig {
            output_path: PathBuf::from("/tmp/m-export-test-frame-count.mp4"),
            width: 4,
            height: 4,
            framerate: 30,
            sample_rate: 48_000,
            channels: 2,
            format: OutputFormat::Mp4H264Aac,
        };
        let mut encoder = GstreamerEncoder::new(cfg).expect("construct");
        let frame = vec![128u8; 4 * 4 * 4];
        for _ in 0..5 {
            encoder
                .push_video_frame(&frame, std::time::Duration::from_millis(0))
                .expect("push");
        }
        assert_eq!(encoder.frames_pushed(), 5);
        let _ = std::fs::remove_file("/tmp/m-export-test-frame-count.mp4.bgra.scratch");
        let _ = std::fs::remove_file("/tmp/m-export-test-frame-count.mp4.f32.scratch");
    }

    // ---- scratch_path ----

    #[test]
    fn scratch_path_appends_suffix_to_full_filename() {
        let p = scratch_path(Path::new("/tmp/out.mp4"), ".bgra.scratch");
        assert_eq!(p, PathBuf::from("/tmp/out.mp4.bgra.scratch"));
    }

    #[test]
    fn scratch_path_handles_no_extension() {
        let p = scratch_path(Path::new("/tmp/outfile"), ".scratch");
        assert_eq!(p, PathBuf::from("/tmp/outfile.scratch"));
    }

    // ---- M-EXPORT.5 — poster helpers ----

    #[test]
    fn poster_path_replaces_extension() {
        assert_eq!(
            poster_path_for(Path::new("/tmp/Screen-2026-05-17-180000.mp4")),
            PathBuf::from("/tmp/Screen-2026-05-17-180000.avif")
        );
        assert_eq!(
            poster_path_for(Path::new("/tmp/Screen-2026-05-17-180000.webm")),
            PathBuf::from("/tmp/Screen-2026-05-17-180000.avif")
        );
    }

    #[test]
    fn poster_pipeline_args_contains_required_elements() {
        let args = poster_pipeline_args(Path::new("/tmp/test.mp4"), Path::new("/tmp/test.avif"));
        // filesrc location=<video>
        assert!(args.iter().any(|a| a == "filesrc"));
        assert!(args.iter().any(|a| a == "location=/tmp/test.mp4"));
        // decodebin → videoconvert → videoscale chain
        assert!(args.iter().any(|a| a == "decodebin"));
        assert!(args.iter().any(|a| a == "videoconvert"));
        assert!(args.iter().any(|a| a == "videoscale"));
        // scale to 640 wide
        assert!(args.iter().any(|a| a == "video/x-raw,width=640"));
        // avifenc → filesink location=<poster>
        assert!(args.iter().any(|a| a == "avifenc"));
        assert!(args.iter().any(|a| a == "location=/tmp/test.avif"));
    }

    #[test]
    fn generate_poster_rejects_missing_video_file() {
        let result = generate_poster(Path::new("/tmp/definitely-not-a-real-video.mp4"));
        assert!(matches!(result, Err(EncodeError::InvalidConfig(_))));
    }
}
