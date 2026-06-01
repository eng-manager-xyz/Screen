//! `VideoEncoder` trait + `OutputFormat` enum + per-OS GStreamer
//! pipeline builders (M-EXPORT.1 of M-RECORD-EXPORT; live variant
//! M-QUAL.1).
//!
//! [`LiveGstreamerEncoder`] (M-QUAL.1) is the [`VideoEncoder`] impl —
//! **streaming**: it spawns the encode pipeline up front and streams
//! BGRA frames into its stdin, so only *compressed* video lands on
//! disk during capture. Bounding the on-disk footprint to the encoded
//! bitrate (raw BGRA is `w × h × 4 × fps` — >1 GB/s at Retina) is the
//! prerequisite for native-resolution capture.
//!
//! ```admonish important title="CLI-pipe, not gstreamer-rs"
//! The encoder doesn't link `gstreamer-rs` — it streams over the
//! child's stdin (`fdsrc fd=0`) rather than via `appsrc`, keeping the
//! project's "CLI-pipe over Rust bindings" convention (no compile-time
//! libgstreamer dep, no Windows-build breakage). A programmatic
//! `appsrc` pipeline remains a possible future swap behind the trait.
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
//! macOS is the hot path for M-RECORD-EXPORT; Win/Linux encoder
//! strings are present so the cross-OS build + the arg-builder unit
//! tests pass, but the runtime spawn returns
//! `EncodeError::Unsupported` outside macOS until those ports land.

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread::JoinHandle;

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

    /// Maximum encodable edge length (px) for this format's hardware
    /// video encoder, or `None` when the encoder imposes no hard
    /// dimension cap.
    ///
    /// Empirically probed on Apple Silicon (M1, GStreamer 1.26.8):
    /// VideoToolbox's `vtenc_h264_hw` rejects caps negotiation
    /// (`not-negotiated (-4)`, then the pipeline refuses to preroll)
    /// the instant *either* edge exceeds 4096 — the H.264 Level 5.2
    /// frame-side ceiling. It is a *per-axis* cap, not a pixel-area
    /// cap (`4096×4096` succeeds while the smaller `5120×1440` fails).
    /// `vtenc_h265_hw` accepts at least `8192×4320` (HEVC Level 6).
    /// The software WebM encoders (`vp9enc` / `svtav1enc`) have no
    /// hard cap — they only get slower. See AUT-334.
    #[must_use]
    pub const fn max_encode_edge(self) -> Option<u32> {
        match self {
            Self::Mp4H264Aac => Some(4096),
            Self::Mp4H265Aac => Some(8192),
            Self::WebmVp9Opus | Self::WebmAv1Opus => None,
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

/// Clamp `(width, height)` so neither edge exceeds `format`'s hardware
/// encoder limit ([`OutputFormat::max_encode_edge`]), **preserving the
/// aspect ratio** and keeping both edges even (H.264 / HEVC require
/// mod-2 dimensions).
///
/// Returns the input — even-rounded — unchanged when it already fits,
/// or when the format has no hard cap (the software WebM encoders).
/// When a clamp is needed, both edges are scaled by the *same* factor
/// `max_edge / max(width, height)`, so the frame is shrunk uniformly
/// and never stretched or squished: the screen content keeps its shape
/// and the camera bubble stays a perfect circle. Integer arithmetic
/// throughout (`u64` intermediate) — deterministic, no float casts.
///
/// This is the fix for AUT-334: a 5K display fed `vtenc_h264_hw` a
/// 5120-wide frame, which fails caps negotiation and discards the
/// whole recording. The live scratch is always H.264, so the recorder
/// clamps capture dims to the H.264 limit before any pipeline starts.
///
/// # Examples
///
/// ```
/// use media::encode::{fit_within_encoder_limits, OutputFormat};
/// // 5K and 6K displays (both 16:9) clamp to exactly 4096×2304 for H.264:
/// assert_eq!(fit_within_encoder_limits(5120, 2880, OutputFormat::Mp4H264Aac), (4096, 2304));
/// assert_eq!(fit_within_encoder_limits(6016, 3384, OutputFormat::Mp4H264Aac), (4096, 2304));
/// // Already within the limit → unchanged:
/// assert_eq!(fit_within_encoder_limits(3840, 2160, OutputFormat::Mp4H264Aac), (3840, 2160));
/// // H.265's higher ceiling keeps full 5K:
/// assert_eq!(fit_within_encoder_limits(5120, 2880, OutputFormat::Mp4H265Aac), (5120, 2880));
/// ```
#[must_use]
pub fn fit_within_encoder_limits(width: u32, height: u32, format: OutputFormat) -> (u32, u32) {
    let to_even = |v: u32| (v & !1u32).max(2);
    let Some(max_edge) = format.max_encode_edge() else {
        return (to_even(width), to_even(height));
    };
    let longest = width.max(height);
    if longest <= max_edge {
        return (to_even(width), to_even(height));
    }
    // Uniform downscale: the longest edge becomes exactly `max_edge`,
    // the shorter edge shrinks by the same ratio. `u64` intermediate
    // avoids overflow for 8K-class inputs; `try_from` keeps it cast-free.
    let scale = |v: u32| -> u32 {
        let scaled = u64::from(v) * u64::from(max_edge) / u64::from(longest);
        to_even(u32::try_from(scaled).unwrap_or(max_edge))
    };
    (scale(width), scale(height))
}

/// Longer-edge bound of the recording resolution cap (the "1080p" cap).
pub const RECORDING_MAX_LONG_EDGE: u32 = 1920;
/// Shorter-edge bound of the recording resolution cap.
pub const RECORDING_MAX_SHORT_EDGE: u32 = 1080;

/// Cap capture/encode dims to a 1080p box (`1920×1080`), aspect-preserving
/// and orientation-agnostic: the longer edge is bounded to
/// [`RECORDING_MAX_LONG_EDGE`] and the shorter to [`RECORDING_MAX_SHORT_EDGE`],
/// both edges scaled by the *same* factor (never stretched — the camera
/// bubble stays circular) and rounded down to even (H.264 chroma
/// subsampling). Dims already inside the box are returned unchanged (never
/// upscaled), only evened. Integer arithmetic throughout.
///
/// Recording at native Retina resolution is too heavy for the live
/// compose → GPU read-back → encode loop to sustain the configured frame
/// rate; the encoder timestamps frames by *count* at that rate, so an
/// under-delivering loop makes the recording play fast. Capping to 1080p
/// keeps the pipeline comfortably real-time regardless of monitor, which
/// is the deliberate tradeoff (reliable 1080p over a fast, half-lost 5K).
///
/// # Examples
///
/// ```
/// use media::encode::cap_recording_dims;
/// assert_eq!(cap_recording_dims(5120, 2880), (1920, 1080)); // 5K 16:9 → 1080p
/// assert_eq!(cap_recording_dims(3840, 2160), (1920, 1080)); // 4K 16:9 → 1080p
/// assert_eq!(cap_recording_dims(2880, 1800), (1728, 1080)); // 16:10 → fits the box
/// assert_eq!(cap_recording_dims(1920, 1080), (1920, 1080)); // already 1080p
/// assert_eq!(cap_recording_dims(1280, 720), (1280, 720));   // smaller → unchanged
/// assert_eq!(cap_recording_dims(1080, 1920), (1080, 1920)); // portrait → long edge 1920
/// ```
#[must_use]
pub fn cap_recording_dims(width: u32, height: u32) -> (u32, u32) {
    let to_even = |v: u32| (v & !1u32).max(2);
    let long = width.max(height);
    let short = width.min(height);
    if long <= RECORDING_MAX_LONG_EDGE && short <= RECORDING_MAX_SHORT_EDGE {
        return (to_even(width), to_even(height));
    }
    // Uniform downscale fitting BOTH bounds — pick the tighter ratio so
    // neither edge exceeds its cap. `u64` intermediate avoids overflow.
    let scale = |v: u32| -> u32 {
        let by_long = u64::from(v) * u64::from(RECORDING_MAX_LONG_EDGE) / u64::from(long);
        let by_short = u64::from(v) * u64::from(RECORDING_MAX_SHORT_EDGE) / u64::from(short);
        to_even(u32::try_from(by_long.min(by_short)).unwrap_or(RECORDING_MAX_SHORT_EDGE))
    };
    (scale(width), scale(height))
}

/// Failure modes for the encoder.
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
/// [`LiveGstreamerEncoder`] impl streams each pushed frame into a
/// `gst-launch-1.0` child and remuxes in the audio at `finalize`.
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

// ---- M-QUAL.1 — live (streaming) video encoder ----------------------

/// Live (streaming) video encoder. Spawns the encode pipeline up front
/// and streams BGRA frames into its stdin, so the only video on disk
/// during capture is *already compressed*. That bounds the scratch
/// footprint to the encoded bitrate instead of the raw firehose
/// (`width × height × 4 × fps` ≈ 250 MB/s at 1080p, >1 GB/s at Retina)
/// — the prerequisite for capturing at native resolution.
///
/// Audio is still buffered to a small raw `.f32.scratch` (≈0.4 MB/s)
/// and muxed in at [`finalize`](VideoEncoder::finalize) via
/// [`build_remux_args`] (the video leg is stream-copied, not
/// re-encoded). Keeping audio out of the live pipeline lets the
/// recorder feed a single fd (stdin) with no extra fd plumbing.
///
/// Real recording is macOS-only; the pipeline strings are cross-OS so
/// the unit tests + clippy pass everywhere, but the spawn only
/// succeeds where the encoder element + `gst-launch-1.0` exist.
#[derive(Debug)]
pub struct LiveGstreamerEncoder {
    config: EncoderConfig,
    /// Compressed video-only intermediate the live child writes to.
    /// Remuxed with audio at finalize, then deleted.
    video_intermediate_path: PathBuf,
    /// Raw F32LE audio scratch — encoded to the container codec at
    /// finalize.
    audio_scratch_path: PathBuf,
    /// The live `gst-launch-1.0` child (stdin → intermediate).
    video_child: Child,
    /// Write end of the child's stdin; `None` once finalize closes it.
    video_stdin: Option<ChildStdin>,
    /// Drains the child's stderr on a side thread so a chatty pipeline
    /// can't deadlock on a full stderr pipe; joined at finalize for
    /// diagnostics.
    stderr_drain: Option<JoinHandle<String>>,
    audio_writer: BufWriter<File>,
    expected_video_bytes_per_frame: usize,
    frames_pushed: u64,
    audio_chunks_pushed: u64,
}

impl LiveGstreamerEncoder {
    /// Spawn the live video-encode pipeline + open the audio scratch.
    ///
    /// # Errors
    ///
    /// - [`EncodeError::InvalidConfig`] — any of width / height /
    ///   framerate / channels / sample_rate is zero.
    /// - [`EncodeError::Unsupported`] — no encoder wired for (format, OS).
    /// - [`EncodeError::Spawn`] — `gst-launch-1.0` missing from PATH.
    /// - [`EncodeError::Io`] — the audio scratch can't be created.
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

        let video_intermediate_path = scratch_path(&config.output_path, ".live-video.scratch");
        let audio_scratch_path = scratch_path(&config.output_path, ".f32.scratch");

        // Build + spawn the live video pipeline before opening the
        // audio scratch so an Unsupported/Spawn failure doesn't leave a
        // dangling file.
        let video_args = build_live_video_args(&config, &video_intermediate_path)?;
        let mut child = Command::new("gst-launch-1.0")
            .args(&video_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| EncodeError::Spawn {
                source: err,
                path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".into()),
            })?;

        let video_stdin = child.stdin.take();
        let stderr_drain = child.stderr.take().map(|mut stderr| {
            std::thread::spawn(move || {
                let mut buf = String::new();
                let _ = stderr.read_to_string(&mut buf);
                buf
            })
        });

        let audio_file = File::create(&audio_scratch_path)?;
        let expected_video_bytes_per_frame = (config.width as usize) * (config.height as usize) * 4;

        Ok(Self {
            config,
            video_intermediate_path,
            audio_scratch_path,
            video_child: child,
            video_stdin,
            stderr_drain,
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

    /// Borrow the resolved encoder config.
    #[must_use]
    pub fn config(&self) -> &EncoderConfig {
        &self.config
    }
}

impl VideoEncoder for LiveGstreamerEncoder {
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
        let Some(stdin) = self.video_stdin.as_mut() else {
            return Err(EncodeError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "live encoder stdin already closed",
            )));
        };
        // A full pipe blocks here — natural backpressure if the HW
        // encoder can't keep up with the compose framerate.
        stdin.write_all(bgra)?;
        self.frames_pushed = self.frames_pushed.saturating_add(1);
        Ok(())
    }

    fn push_audio_chunk(
        &mut self,
        samples: &[f32],
        _pts: std::time::Duration,
    ) -> Result<(), EncodeError> {
        for sample in samples {
            self.audio_writer.write_all(&sample.to_le_bytes())?;
        }
        self.audio_chunks_pushed = self.audio_chunks_pushed.saturating_add(1);
        Ok(())
    }

    fn finalize(mut self: Box<Self>) -> Result<PathBuf, EncodeError> {
        // 1. Close the live child's stdin → fdsrc sees EOF → EOS →
        //    mp4mux writes its moov atom → child exits.
        drop(self.video_stdin.take());
        let status = self.video_child.wait()?;
        let stderr = self
            .stderr_drain
            .take()
            .and_then(|h| h.join().ok())
            .unwrap_or_default();
        if !status.success() {
            let _ = std::fs::remove_file(&self.video_intermediate_path);
            let _ = std::fs::remove_file(&self.audio_scratch_path);
            return Err(EncodeError::PipelineFailed {
                exit: status.code(),
                stderr,
            });
        }

        // 2. Flush the audio scratch so the remux reads complete data.
        //    The File handle itself closes when `self` drops at the end
        //    of finalize; the flushed bytes are already visible to the
        //    gst child reading the same path (can't move it out — Drop).
        self.audio_writer.flush()?;

        let has_video = self.frames_pushed > 0;
        let has_audio = self.audio_chunks_pushed > 0;

        // 3. Produce the final container.
        if has_video && !has_audio {
            // Video-only: the live intermediate already IS the final
            // container — move it into place (no remux / re-encode).
            // The scratch sits next to the output (same dir), so the
            // rename stays on one filesystem; copy is a paranoia
            // fallback.
            if std::fs::rename(&self.video_intermediate_path, &self.config.output_path).is_err() {
                std::fs::copy(&self.video_intermediate_path, &self.config.output_path)?;
                let _ = std::fs::remove_file(&self.video_intermediate_path);
            }
            let _ = std::fs::remove_file(&self.audio_scratch_path);
        } else {
            let args = build_remux_args(
                &self.config,
                &self.video_intermediate_path,
                &self.audio_scratch_path,
                has_video,
                has_audio,
            );
            tracing::info!(
                output = %self.config.output_path.display(),
                format = ?self.config.format,
                video_frames = self.frames_pushed,
                audio_chunks = self.audio_chunks_pushed,
                remux_args = ?args,
                "LiveGstreamerEncoder::finalize remuxing"
            );
            let output = Command::new("gst-launch-1.0")
                .args(&args)
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
            let _ = std::fs::remove_file(&self.video_intermediate_path);
            let _ = std::fs::remove_file(&self.audio_scratch_path);
        }

        Ok(self.config.output_path.clone())
    }
}

impl Drop for LiveGstreamerEncoder {
    fn drop(&mut self) {
        // `finalize()` already took stdin + waited the child. On an
        // early drop (error / cancel path) close stdin and kill the
        // child so we don't orphan a gst-launch feeding a dead pipe
        // (mirrors decode::gstreamer_pipe's drop-kill).
        if self.video_stdin.take().is_some() {
            let _ = self.video_child.kill();
            let _ = self.video_child.wait();
        }
    }
}

/// Build the `gst-launch-1.0` argv for the **live video-only** encode
/// (M-QUAL.1). Frames arrive on the child's stdin (`fd=0`) as raw
/// BGRA; the pipeline encodes them straight to a compressed
/// video-only container at `intermediate`. The audio leg is handled
/// separately at finalize ([`build_remux_args`]).
///
/// Shape: `-q -e fdsrc fd=0 ! rawvideoparse format=bgra width=W
/// height=H framerate=F/1 ! videoconvert ! <encoder> ! <parser> !
/// <mux> ! filesink location=<intermediate>`.
///
/// # Errors
///
/// [`EncodeError::Unsupported`] if the (format, OS) combo has no wired
/// encoder.
pub fn build_live_video_args(
    config: &EncoderConfig,
    intermediate: &Path,
) -> Result<Vec<String>, EncodeError> {
    let (video_encoder_elements, mux_element) =
        encoder_and_mux_elements(config.format, std::env::consts::OS)?;

    // `-e` makes gst-launch forward EOS on a stop signal; the EOF from
    // a closed stdin already triggers a clean EOS, but `-e` keeps the
    // moov-atom-on-stop guarantee if we ever stop via signal instead.
    let mut args: Vec<String> = vec![
        "-q".to_string(),
        "-e".to_string(),
        "fdsrc".to_string(),
        "fd=0".to_string(),
        "!".to_string(),
        "rawvideoparse".to_string(),
        "format=bgra".to_string(),
        format!("width={}", config.width),
        format!("height={}", config.height),
        format!("framerate={}/1", config.framerate),
        "!".to_string(),
        "videoconvert".to_string(),
        "!".to_string(),
    ];
    for elem in &video_encoder_elements {
        // An encoder entry may carry properties (e.g. `vtenc_h264_hw
        // allow-frame-reordering=false`); push each whitespace-separated
        // token as its own argv item so gst-launch parses them as an
        // element + its properties rather than one bogus element name.
        for token in elem.split_whitespace() {
            args.push(token.to_string());
        }
        args.push("!".to_string());
    }
    args.push(mux_to_parser(config.format).to_string());
    args.push("!".to_string());
    args.push(mux_element.to_string());
    args.push("!".to_string());
    args.push("filesink".to_string());
    args.push(format!("location={}", intermediate.display()));
    Ok(args)
}

/// Build the `gst-launch-1.0` argv for the **finalize remux**
/// (M-QUAL.1). The live video `intermediate` is stream-copied (no
/// re-encode) and the raw F32LE `audio_scratch` is encoded to the
/// container's audio codec, both muxed into `config.output_path`.
///
/// `has_video` / `has_audio` gate the legs. In the normal recording
/// case both are true; a video-only recording skips this entirely (the
/// intermediate is moved into place by `finalize`), so the
/// `!has_video` arm only exists for the (unusual) audio-only case.
///
/// Shape (video + audio): `-q -e <mux> name=mux ! filesink
/// location=<output>  filesrc location=<intermediate> ! <demux> !
/// <parser> ! mux.  filesrc location=<audio> ! rawaudioparse … !
/// audioconvert ! audioresample ! <audio-encoder> ! mux.`
#[must_use]
pub fn build_remux_args(
    config: &EncoderConfig,
    intermediate: &Path,
    audio_scratch: &Path,
    has_video: bool,
    has_audio: bool,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-q".to_string(),
        "-e".to_string(),
        mux_element_for(config.format).to_string(),
        "name=mux".to_string(),
        "!".to_string(),
        "filesink".to_string(),
        format!("location={}", config.output_path.display()),
    ];
    if has_video {
        args.extend([
            "filesrc".to_string(),
            format!("location={}", intermediate.display()),
            "!".to_string(),
            demux_for(config.format).to_string(),
            "!".to_string(),
            mux_to_parser(config.format).to_string(),
            "!".to_string(),
            "mux.".to_string(),
        ]);
    }
    if has_audio {
        args.extend([
            "filesrc".to_string(),
            format!("location={}", audio_scratch.display()),
            "!".to_string(),
            "rawaudioparse".to_string(),
            "pcm-format=f32le".to_string(),
            format!("sample-rate={}", config.sample_rate),
            format!("num-channels={}", config.channels),
            "!".to_string(),
            "audioconvert".to_string(),
            "!".to_string(),
            "audioresample".to_string(),
            "!".to_string(),
            audio_encoder_element(config.format).to_string(),
            "!".to_string(),
            "mux.".to_string(),
        ]);
    }
    args
}

// ---- ED.21 — edited-export audio (decode → retime → remux) ----------

/// Build the `gst-launch-1.0` argv that decodes `source`'s audio track to
/// raw **interleaved F32LE** on stdout at `sample_rate` / `channels` (ED.21).
///
/// The edited export has no `.f32` scratch at edit time (the audio lives
/// inside the source MP4), so the per-segment retime can't be expressed as a
/// live capture. Instead we decode the whole source audio to raw samples with
/// this one pass, slice + retime it per the project's segments in pure Rust
/// (`screen_app::editor_export::retime_audio`), then feed the result to the
/// encoder's audio scratch via [`VideoEncoder::push_audio_chunk`] — so the
/// existing [`build_remux_args`] finalize muxes it exactly as for a live
/// recording. GStreamer owns the intake; Rust owns the edit arithmetic.
///
/// Shape: `-q filesrc location=SRC ! decodebin ! audioconvert ! audioresample
/// ! audio/x-raw,format=F32LE,rate=R,channels=C,layout=interleaved ! fdsink
/// fd=1` — the input mirror of the `fdsink fd=1` decode pattern the `decode`
/// crate uses for video.
#[must_use]
pub fn build_audio_decode_args(source: &Path, sample_rate: u32, channels: u8) -> Vec<String> {
    vec![
        "-q".to_string(),
        "filesrc".to_string(),
        format!("location={}", source.display()),
        "!".to_string(),
        "decodebin".to_string(),
        "!".to_string(),
        "audioconvert".to_string(),
        "!".to_string(),
        "audioresample".to_string(),
        "!".to_string(),
        format!(
            "audio/x-raw,format=F32LE,rate={sample_rate},channels={channels},layout=interleaved"
        ),
        "!".to_string(),
        "fdsink".to_string(),
        "fd=1".to_string(),
    ]
}

/// Decode `source`'s audio track to interleaved F32LE samples at
/// `sample_rate` / `channels` (ED.21). Returns an **empty** `Vec` when the
/// source has no audio track (probed via [`scratch_has_audio`]) — the caller
/// then exports video-only.
///
/// # Errors
///
/// [`EncodeError::Spawn`] if `gst-launch-1.0` isn't on PATH;
/// [`EncodeError::PipelineFailed`] if the decode pipeline exits non-zero.
pub fn decode_source_audio_f32(
    source: &Path,
    sample_rate: u32,
    channels: u8,
) -> Result<Vec<f32>, EncodeError> {
    if !scratch_has_audio(source) {
        return Ok(Vec::new());
    }
    let args = build_audio_decode_args(source, sample_rate, channels);
    let output = Command::new("gst-launch-1.0")
        .args(&args)
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
    // Reinterpret the raw F32LE byte stream as `f32` samples (drop any
    // trailing partial sample the pipe may leave on an abrupt EOS).
    let samples = output
        .stdout
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    Ok(samples)
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

// ---- M-SAVE.2 — MP4 → WebM transcode (deferred export) --------------

/// Transcode an existing video file (the MP4/H.264 recording scratch)
/// into a VP9 + Opus `.webm` at `output`. Drives the Save panel's
/// "WebM" export: the recorder always captures to an MP4/H.264 scratch
/// (the canonical intermediate), and a WebM export re-encodes it here.
///
/// Probes `input` for an audio track via [`scratch_has_audio`] and
/// includes the Opus leg only when one is present — a screen-only
/// recording's scratch has no audio track, and wiring an audio branch
/// to a `decodebin` pad that never appears would hang `webmmux`
/// waiting for EOS on it.
///
/// VP9 has no Apple HW encoder, so this uses the `vp9enc` software
/// encoder (same as the live WebM encode path) — a short clip takes a
/// few seconds. **Call it off the main thread** (the recorder runs it
/// via `spawn_blocking`).
///
/// # Errors
///
/// - [`EncodeError::InvalidConfig`] — `input` doesn't exist.
/// - [`EncodeError::Spawn`] — `gst-launch-1.0` missing from PATH.
/// - [`EncodeError::PipelineFailed`] — the transcode exited non-zero.
pub fn transcode_to_webm(input: &Path, output: &Path) -> Result<(), EncodeError> {
    if !input.exists() {
        return Err(EncodeError::InvalidConfig(format!(
            "transcode input does not exist: {}",
            input.display()
        )));
    }
    let has_audio = scratch_has_audio(input);
    let args = build_webm_transcode_args(input, output, has_audio);

    tracing::info!(
        input = %input.display(),
        output = %output.display(),
        has_audio,
        "transcode_to_webm: spawning gst-launch-1.0"
    );

    let result = Command::new("gst-launch-1.0")
        .args(&args)
        .output()
        .map_err(|err| EncodeError::Spawn {
            source: err,
            path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".into()),
        })?;

    if result.status.success() {
        Ok(())
    } else {
        Err(EncodeError::PipelineFailed {
            exit: result.status.code(),
            stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
        })
    }
}

/// Build the gst-launch argv for the MP4 → WebM transcode. Split out
/// so tests assert the pipeline shape without spawning gst. The Opus
/// audio leg is present only when `has_audio` (see
/// [`transcode_to_webm`] for why).
///
/// Shape: `filesrc ! decodebin name=d  webmmux name=mux ! filesink
/// d. ! queue ! videoconvert ! vp9enc ! mux.  [d. ! queue !
/// audioconvert ! audioresample ! opusenc ! mux.]`
#[must_use]
pub fn build_webm_transcode_args(input: &Path, output: &Path, has_audio: bool) -> Vec<String> {
    // `-e` forces EOS so the muxer writes its cues on completion;
    // `-q` quiets per-buffer chatter.
    let mut args = vec![
        "-q".to_string(),
        "-e".to_string(),
        "filesrc".to_string(),
        format!("location={}", input.display()),
        "!".to_string(),
        "decodebin".to_string(),
        "name=d".to_string(),
        // Muxer + sink declared up front so the `mux.` back-references
        // below resolve at parse time.
        "webmmux".to_string(),
        "name=mux".to_string(),
        "!".to_string(),
        "filesink".to_string(),
        format!("location={}", output.display()),
        // Video leg.
        "d.".to_string(),
        "!".to_string(),
        "queue".to_string(),
        "!".to_string(),
        "videoconvert".to_string(),
        "!".to_string(),
        "vp9enc".to_string(),
        "!".to_string(),
        "mux.".to_string(),
    ];
    if has_audio {
        args.extend(
            [
                "d.",
                "!",
                "queue",
                "!",
                "audioconvert",
                "!",
                "audioresample",
                "!",
                "opusenc",
                "!",
                "mux.",
            ]
            .into_iter()
            .map(String::from),
        );
    }
    args
}

/// Probe `input` for an audio track via `gst-discoverer-1.0`. Returns
/// `true` only when an audio stream is reported; any failure (binary
/// missing, probe error, no audio) returns `false`, so the transcode
/// falls back to a video-only pipeline rather than hang on a
/// `decodebin` audio pad that never fires.
#[must_use]
pub fn scratch_has_audio(input: &Path) -> bool {
    let Ok(output) = Command::new("gst-discoverer-1.0").arg(input).output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    // gst-discoverer prints `Audio #0: …` per stream + an `audio:` line
    // in the topology — either marks an audio track.
    let lower = String::from_utf8_lossy(&output.stdout).to_lowercase();
    lower.contains("audio #") || lower.contains("audio:")
}

/// Per-(format, OS) video encoder element(s) + muxer element, used by
/// [`build_live_video_args`] so the encoder coverage table lives in
/// one place.
///
/// # Errors
///
/// [`EncodeError::Unsupported`] for any (format, OS) without a wired
/// encoder.
fn encoder_and_mux_elements(
    format: OutputFormat,
    os: &str,
) -> Result<(Vec<&'static str>, &'static str), EncodeError> {
    let pair = match (format, os) {
        // `allow-frame-reordering=false` disables B-frames so PTS == DTS.
        // The live scratch is re-demuxed at finalize to mux in the audio
        // track, and VideoToolbox's default B-frame reordering produces
        // buffers that demux with no PTS — mp4mux then rejects them
        // ("Could not multiplex stream / Buffer has no PTS"), silently
        // failing the finalize of EVERY recording that has audio (the
        // video-only path escapes it by renaming the scratch, not
        // re-muxing). B-frames are negligible for screen content and add
        // encode latency, so disabling them is the right call regardless.
        (OutputFormat::Mp4H264Aac, "macos") => {
            (vec!["vtenc_h264_hw allow-frame-reordering=false"], "mp4mux")
        }
        (OutputFormat::Mp4H265Aac, "macos") => {
            (vec!["vtenc_h265_hw allow-frame-reordering=false"], "mp4mux")
        }
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
    Ok(pair)
}

/// Audio encoder element for the container family.
fn audio_encoder_element(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Mp4H264Aac | OutputFormat::Mp4H265Aac => "avenc_aac",
        OutputFormat::WebmVp9Opus | OutputFormat::WebmAv1Opus => "opusenc",
    }
}

/// Muxer element for the container family (OS-independent).
fn mux_element_for(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Mp4H264Aac | OutputFormat::Mp4H265Aac => "mp4mux",
        OutputFormat::WebmVp9Opus | OutputFormat::WebmAv1Opus => "webmmux",
    }
}

/// Demuxer element that reads the live video intermediate back for the
/// finalize remux.
fn demux_for(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Mp4H264Aac | OutputFormat::Mp4H265Aac => "qtdemux",
        OutputFormat::WebmVp9Opus | OutputFormat::WebmAv1Opus => "matroskademux",
    }
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

    // ---- shared arg-builder test config ----

    fn test_config(format: OutputFormat) -> EncoderConfig {
        EncoderConfig::for_output(PathBuf::from("/tmp/test.x"), format)
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

    // ---- M-SAVE.2 — WebM transcode argv ----

    #[test]
    fn webm_transcode_args_video_only_omits_audio_leg() {
        let args =
            build_webm_transcode_args(Path::new("/tmp/in.mp4"), Path::new("/tmp/out.webm"), false);
        // filesrc → decodebin → vp9enc → webmmux → filesink
        assert!(args.iter().any(|a| a == "filesrc"));
        assert!(args.iter().any(|a| a == "location=/tmp/in.mp4"));
        assert!(args.iter().any(|a| a == "decodebin"));
        assert!(args.iter().any(|a| a == "vp9enc"));
        assert!(args.iter().any(|a| a == "webmmux"));
        assert!(args.iter().any(|a| a == "location=/tmp/out.webm"));
        // No audio leg for a video-only scratch.
        assert!(!args.iter().any(|a| a == "opusenc"));
        assert!(!args.iter().any(|a| a == "audioconvert"));
        // Exactly one decodebin back-reference (video only).
        assert_eq!(args.iter().filter(|a| a.as_str() == "d.").count(), 1);
    }

    #[test]
    fn webm_transcode_args_with_audio_includes_opus_leg() {
        let args =
            build_webm_transcode_args(Path::new("/tmp/in.mp4"), Path::new("/tmp/out.webm"), true);
        assert!(args.iter().any(|a| a == "vp9enc"));
        assert!(args.iter().any(|a| a == "opusenc"));
        assert!(args.iter().any(|a| a == "audioconvert"));
        assert!(args.iter().any(|a| a == "audioresample"));
        // Two decodebin back-references: video + audio legs.
        assert_eq!(args.iter().filter(|a| a.as_str() == "d.").count(), 2);
    }

    #[test]
    fn transcode_to_webm_rejects_missing_input() {
        let result = transcode_to_webm(
            Path::new("/tmp/definitely-not-a-real-scratch.mp4"),
            Path::new("/tmp/out.webm"),
        );
        assert!(matches!(result, Err(EncodeError::InvalidConfig(_))));
    }

    // ---- M-QUAL.1 — live video + remux args ----

    #[test]
    fn live_video_args_stream_from_stdin_with_caps() {
        if std::env::consts::OS != "macos"
            && std::env::consts::OS != "windows"
            && std::env::consts::OS != "linux"
        {
            return;
        }
        let cfg = test_config(OutputFormat::Mp4H264Aac);
        let args =
            build_live_video_args(&cfg, Path::new("/tmp/inter.scratch")).expect("supported OS");
        // Frames arrive on the child's stdin.
        assert!(args.iter().any(|a| a == "fdsrc"));
        assert!(args.iter().any(|a| a == "fd=0"));
        // Raw BGRA caps reflect the config.
        assert!(args.iter().any(|a| a == "format=bgra"));
        assert!(args.iter().any(|a| a.starts_with("width=1920")));
        assert!(args.iter().any(|a| a.starts_with("height=1080")));
        assert!(args.iter().any(|a| a.starts_with("framerate=30/1")));
        // mp4 container + h264 parser, written to the intermediate.
        assert!(args.iter().any(|a| a == "mp4mux"));
        assert!(args.iter().any(|a| a == "h264parse"));
        assert!(args.iter().any(|a| a == "location=/tmp/inter.scratch"));
        // No audio leg in the live pipeline — audio is muxed at finalize.
        assert!(!args.iter().any(|a| a == "audioconvert"));
    }

    #[test]
    fn live_h264_disables_frame_reordering_so_the_scratch_remuxes() {
        // macOS vtenc must disable B-frame reordering: the live H.264
        // scratch is re-demuxed at finalize to mux in audio, and reordered
        // frames demux with no PTS → mp4mux rejects them, failing the
        // finalize of every audio recording. The encoder element + its
        // property must each be their own argv token.
        if std::env::consts::OS != "macos" {
            return;
        }
        let cfg = test_config(OutputFormat::Mp4H264Aac);
        let args = build_live_video_args(&cfg, Path::new("/tmp/v.scratch")).expect("macos");
        assert!(
            args.iter().any(|a| a == "vtenc_h264_hw"),
            "encoder element is its own token"
        );
        assert!(
            args.iter().any(|a| a == "allow-frame-reordering=false"),
            "B-frame reordering off so the scratch demuxes with PTS for the finalize remux"
        );
    }

    #[test]
    fn remux_args_copy_video_and_encode_audio() {
        let cfg = test_config(OutputFormat::Mp4H264Aac);
        let args = build_remux_args(
            &cfg,
            Path::new("/tmp/inter.scratch"),
            Path::new("/tmp/a.f32.scratch"),
            true,
            true,
        );
        // Video leg: demux the intermediate + parse (stream-copy, no re-encode).
        assert!(args.iter().any(|a| a == "qtdemux"));
        assert!(args.iter().any(|a| a == "h264parse"));
        assert!(args.iter().any(|a| a == "location=/tmp/inter.scratch"));
        // The video is copied — there must be NO video encoder element.
        assert!(!args.iter().any(|a| a == "vtenc_h264_hw"));
        // Audio leg: raw F32 → AAC.
        assert!(args.iter().any(|a| a == "rawaudioparse"));
        assert!(args.iter().any(|a| a == "pcm-format=f32le"));
        assert!(args.iter().any(|a| a == "avenc_aac"));
        // Mux + sink to the final output.
        assert!(args.iter().any(|a| a == "mp4mux"));
        assert!(args.iter().any(|a| a == "location=/tmp/test.x"));
    }

    #[test]
    fn audio_decode_args_pipe_raw_f32le_to_stdout() {
        // ED.21: the edited-export audio intake decodes the source's audio to
        // raw interleaved F32LE on stdout at the encoder's caps, so the
        // per-segment retime can slice it in Rust.
        let args = build_audio_decode_args(Path::new("/tmp/source.mp4"), 48_000, 2);
        assert_eq!(args.first().map(String::as_str), Some("-q"));
        assert!(args.iter().any(|a| a == "location=/tmp/source.mp4"));
        assert!(args.iter().any(|a| a == "decodebin"));
        assert!(args.iter().any(|a| a == "audioconvert"));
        assert!(args.iter().any(|a| a == "audioresample"));
        // Caps must match the encoder's audio scratch (F32LE @ 48k stereo)
        // so the finalize remux's rawaudioparse reads them correctly.
        assert!(args.iter().any(|a| a.contains("format=F32LE")
            && a.contains("rate=48000")
            && a.contains("channels=2")
            && a.contains("layout=interleaved")));
        // Raw samples come out on stdout (fd 1), the input mirror of the
        // video decode pattern.
        assert!(args.iter().any(|a| a == "fdsink"));
        assert!(args.iter().any(|a| a == "fd=1"));
        // It's a decode, not an encode — no encoder / mux elements.
        assert!(!args.iter().any(|a| a == "avenc_aac"));
        assert!(!args.iter().any(|a| a == "mp4mux"));
    }

    #[test]
    fn remux_args_video_only_omits_audio_leg() {
        let cfg = test_config(OutputFormat::Mp4H264Aac);
        let args = build_remux_args(
            &cfg,
            Path::new("/tmp/inter.scratch"),
            Path::new("/tmp/a.f32.scratch"),
            true,
            false,
        );
        assert!(args.iter().any(|a| a == "qtdemux"));
        assert!(!args.iter().any(|a| a == "avenc_aac"));
        assert!(!args.iter().any(|a| a == "rawaudioparse"));
    }

    /// Anti-regression mirror of
    /// [`pipeline_args_never_use_legacy_pcm_f32le_token`] for the
    /// remux audio leg — `format=pcm-f32le` is invalid on
    /// `rawaudioparse` and silently drops every audio recording.
    #[test]
    fn remux_args_never_use_legacy_pcm_f32le_token() {
        let cfg = test_config(OutputFormat::Mp4H264Aac);
        let args = build_remux_args(
            &cfg,
            Path::new("/tmp/inter.scratch"),
            Path::new("/tmp/a.f32.scratch"),
            true,
            true,
        );
        assert!(!args.iter().any(|a| a == "format=pcm-f32le"));
        assert!(args.iter().any(|a| a == "pcm-format=f32le"));
    }

    #[test]
    fn encoder_and_mux_elements_rejects_unknown_os() {
        let result = encoder_and_mux_elements(OutputFormat::Mp4H264Aac, "plan9");
        assert!(matches!(result, Err(EncodeError::Unsupported { .. })));
    }

    #[test]
    fn encoder_and_mux_elements_maps_each_format() {
        // Per-format encoder/mux/audio selection on the current OS
        // (retains the coverage the removed batch pipeline_args tests
        // had). Other OSes' match arms are checked at compile time.
        let os = std::env::consts::OS;
        if os != "macos" && os != "windows" && os != "linux" {
            return;
        }
        for (fmt, want_mux, want_audio) in [
            (OutputFormat::Mp4H264Aac, "mp4mux", "avenc_aac"),
            (OutputFormat::Mp4H265Aac, "mp4mux", "avenc_aac"),
            (OutputFormat::WebmVp9Opus, "webmmux", "opusenc"),
            (OutputFormat::WebmAv1Opus, "webmmux", "opusenc"),
        ] {
            let (encoders, mux) = encoder_and_mux_elements(fmt, os).expect("supported OS");
            assert!(!encoders.is_empty(), "{fmt:?}: needs an encoder element");
            assert_eq!(mux, want_mux, "{fmt:?}: muxer");
            assert_eq!(
                audio_encoder_element(fmt),
                want_audio,
                "{fmt:?}: audio encoder"
            );
        }
    }

    #[test]
    fn live_encoder_new_rejects_zero_dimensions() {
        // Validation runs before any gst spawn, so this needs no
        // gstreamer on PATH (runs on every CI OS).
        let mut cfg = test_config(OutputFormat::Mp4H264Aac);
        cfg.width = 0;
        let result = LiveGstreamerEncoder::new(cfg);
        assert!(matches!(result, Err(EncodeError::InvalidConfig(_))));
    }

    // ---- AUT-334 — encoder-limit clamp (aspect-preserving) ----

    #[test]
    fn max_encode_edge_is_codec_specific() {
        assert_eq!(OutputFormat::Mp4H264Aac.max_encode_edge(), Some(4096));
        assert_eq!(OutputFormat::Mp4H265Aac.max_encode_edge(), Some(8192));
        assert_eq!(OutputFormat::WebmVp9Opus.max_encode_edge(), None);
        assert_eq!(OutputFormat::WebmAv1Opus.max_encode_edge(), None);
    }

    #[test]
    fn fit_within_limits_passes_through_when_within_h264_cap() {
        // ≤4096 on both edges → unchanged (even inputs stay identical).
        for (w, h) in [
            (1920, 1080),
            (3840, 2160),
            (4096, 2160),
            (4096, 2304),
            (4096, 4096),
        ] {
            assert_eq!(
                fit_within_encoder_limits(w, h, OutputFormat::Mp4H264Aac),
                (w, h),
                "{w}x{h} is within the H.264 cap and must pass through"
            );
        }
    }

    #[test]
    fn fit_within_limits_downscales_real_over_4k_displays_to_4096x2304() {
        // Every real >4K display is 16:9 (5K / 6K / 8K), so each clamps
        // to exactly 4096×2304 with zero aspect drift.
        for (w, h) in [(5120, 2880), (6016, 3384), (7680, 4320)] {
            let (cw, ch) = fit_within_encoder_limits(w, h, OutputFormat::Mp4H264Aac);
            assert!(cw <= 4096 && ch <= 4096, "{w}x{h} -> {cw}x{ch} exceeds cap");
            assert_eq!(cw % 2, 0, "width must be even");
            assert_eq!(ch % 2, 0, "height must be even");
            assert_eq!((cw, ch), (4096, 2304), "{w}x{h} should clamp to 4096x2304");
        }
    }

    #[test]
    fn fit_within_limits_clamps_longest_edge_and_keeps_ratio() {
        // Ultrawide (21:9), portrait, and a tall sliver — the longest
        // edge becomes exactly 4096; the other shrinks by the same
        // factor. Even-flooring may drift the shorter edge by <1px;
        // assert the aspect skew stays sub-pixel via cross-multiply.
        for (w, h) in [(5120, 2160), (2880, 5120), (5120, 1440)] {
            let (cw, ch) = fit_within_encoder_limits(w, h, OutputFormat::Mp4H264Aac);
            assert!(cw <= 4096 && ch <= 4096, "{w}x{h} -> {cw}x{ch} exceeds cap");
            assert_eq!(cw.max(ch), 4096, "longest edge scales to the 4096 cap");
            assert_eq!(cw % 2, 0);
            assert_eq!(ch % 2, 0);
            // |w·ch − h·cw| is the aspect error scaled by the source's
            // longest edge; keep it under ~2px of drift.
            let drift = (i64::from(w) * i64::from(ch) - i64::from(h) * i64::from(cw)).abs();
            assert!(
                drift <= 2 * i64::from(w.max(h)),
                "{w}x{h} -> {cw}x{ch} skews the aspect ratio"
            );
        }
    }

    #[test]
    fn fit_within_limits_h265_keeps_5k_and_caps_at_8192() {
        // HEVC's 8192 ceiling keeps full 5K and 8K; only >8192 clamps.
        assert_eq!(
            fit_within_encoder_limits(5120, 2880, OutputFormat::Mp4H265Aac),
            (5120, 2880)
        );
        assert_eq!(
            fit_within_encoder_limits(8192, 4320, OutputFormat::Mp4H265Aac),
            (8192, 4320)
        );
        let (cw, ch) = fit_within_encoder_limits(10240, 4320, OutputFormat::Mp4H265Aac);
        assert!(cw <= 8192 && ch <= 8192);
        assert_eq!(cw, 8192, "longest edge clamps to the HEVC 8192 cap");
    }

    #[test]
    fn fit_within_limits_never_clamps_software_webm() {
        // libvpx / SVT-AV1 accept any size (no negotiation cap).
        assert_eq!(
            fit_within_encoder_limits(5120, 2880, OutputFormat::WebmVp9Opus),
            (5120, 2880)
        );
        assert_eq!(
            fit_within_encoder_limits(7680, 4320, OutputFormat::WebmAv1Opus),
            (7680, 4320)
        );
    }

    #[test]
    fn fit_within_limits_evens_odd_input() {
        // Odd dims shouldn't occur post-`sanitize_dims`, but the encoder
        // requires mod-2, so the clamp floors to even defensively.
        let (cw, ch) = fit_within_encoder_limits(4097, 2161, OutputFormat::Mp4H264Aac);
        assert!(cw <= 4096 && ch <= 4096);
        assert_eq!(cw % 2, 0);
        assert_eq!(ch % 2, 0);
    }
}
