//! Real video decode via the `gst-launch-1.0` and `gst-discoverer-1.0` CLIs.
//!
//! Mirrors the spirit of [`crate::mock::MockVideoStream`] but reads bytes
//! from a real `GStreamer` pipeline:
//!
//! ```text
//! filesrc location=<path>
//!   ! decodebin
//!   ! videoconvert
//!   ! video/x-raw,format=BGRA
//!   ! fdsink fd=1
//! ```
//!
//! # Trade-offs
//!
//! - **+** Zero compile-time integration with libgstreamer; works against
//!   any system `GStreamer` the user installs (`brew install gstreamer`,
//!   `apt install gstreamer1.0-tools`, …). No FFI surface.
//! - **+** `GStreamer` is LGPL — friendlier than `FFmpeg`'s GPL-or-LGPL split,
//!   no licensing entanglement for our binary either way.
//! - **+** Hardware decode picks up automatically when `GStreamer`'s
//!   `vah264dec` / `vtdec` / `nvh264dec` plugins are present.
//! - **−** Per-process fork; for the player loop that's one fork for the
//!   whole stream, not per-frame.
//! - **−** Assumes `gst-launch-1.0` and `gst-discoverer-1.0` on `PATH`.
//!   Reported as [`Error::Spawn`] rather than a panic.
//!
//! For a proper Rust-bound integration via `gstreamer-rs` see M-DEC.3+;
//! the [`crate::VideoStream`] trait makes it a swap-in replacement.

use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::{VideoFrame, VideoStream};

/// Streams BGRA frames from a video file by piping it through `gst-launch-1.0`.
pub struct GstreamerPipeStream {
    child: Child,
    width: u32,
    height: u32,
    frame_rate: f32,
    frame_count: Option<u64>,
    next_index: u64,
    /// Pre-sized buffer — `width * height * 4` bytes, reused per frame.
    frame_buffer: Vec<u8>,
}

/// Failure modes for the gstreamer-pipe decoder.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `gst-launch-1.0` or `gst-discoverer-1.0` could not be launched.
    /// Almost always means the `GStreamer` CLI tools aren't on `PATH`.
    /// The error message includes the current `PATH` value so CI logs
    /// surface the exact lookup state for diagnosis.
    #[error(
        "failed to spawn `{cmd}`: {source} (is `GStreamer` installed and on PATH? \
         current PATH={path})"
    )]
    Spawn {
        /// The command we tried to spawn.
        cmd: &'static str,
        /// The OS-level reason the spawn failed.
        #[source]
        source: std::io::Error,
        /// Snapshot of `PATH` at the moment of failure.
        path: String,
    },
    /// `gst-discoverer-1.0` returned non-parseable output.
    #[error("gst-discoverer output unparseable: {0}")]
    DiscoverParse(String),
    /// `gst-discoverer-1.0` returned an error exit code.
    #[error("gst-discoverer failed for `{path}`: {stderr}")]
    DiscoverFailed {
        /// The video path we asked about.
        path: PathBuf,
        /// Captured stderr.
        stderr: String,
    },
    /// I/O error while reading the pipe.
    #[error("pipe read error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Per-stream metadata, as reported by `gst-discoverer-1.0`.
#[derive(Debug, Clone)]
pub struct VideoMetadata {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Reported frame rate (fps).
    pub frame_rate: f32,
    /// Total frame count if reported. Most container formats don't carry
    /// this directly; we derive it from `duration × frame_rate` when both
    /// are available.
    pub frame_count: Option<u64>,
}

/// Whether both `gst-launch-1.0` and `gst-discoverer-1.0` are on `PATH`
/// and runnable. Useful as a runtime guard for tests + production code
/// that wants to gracefully degrade when `GStreamer` isn't installed.
///
/// Empirically: on GitHub Actions Ubuntu runners, the
/// `gstreamer1.0-tools` apt package installs successfully BUT the
/// resulting binaries are sometimes not findable from later cargo
/// nextest test processes (root cause: TBD). Tests that depend on
/// these binaries should call this and skip if it returns `false`,
/// matching the pattern in `crates/decode/tests/gstreamer_integration.rs`.
/// Note: prefer `media::gstreamer::is_available` (M-MEDIA.1) for new
/// code — it returns a structured `media::gstreamer::GStreamerProbe`
/// with `PATH` snapshot + per-binary version + per-plugin presence, which
/// is much easier to debug in CI than a bare `bool`. This helper is kept
/// for backwards compatibility with existing decode / preview / app
/// integration tests; cutover happens lazily as those tests are touched.
#[must_use]
pub fn gstreamer_available() -> bool {
    Command::new("gst-launch-1.0")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
        && Command::new("gst-discoverer-1.0")
            .arg("--version")
            .output()
            .is_ok_and(|out| out.status.success())
}

impl GstreamerPipeStream {
    /// Probe `path` with `gst-discoverer-1.0` and return the metadata,
    /// without starting a decode.
    pub fn probe(path: &Path) -> Result<VideoMetadata> {
        let uri = file_uri(path);
        let output = Command::new("gst-discoverer-1.0")
            .args(["-v", &uri])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|source| Error::Spawn {
                cmd: "gst-discoverer-1.0",
                source,
                path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".into()),
            })?;

        if !output.status.success() {
            return Err(Error::DiscoverFailed {
                path: path.to_path_buf(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        parse_discoverer(&String::from_utf8_lossy(&output.stdout))
    }

    /// Open `path` for streaming decode. Spawns `gst-launch-1.0`
    /// immediately; frames are pulled via [`VideoStream::next_frame`].
    pub fn open(path: &Path) -> Result<Self> {
        let meta = Self::probe(path)?;
        let frame_size = (meta.width as usize) * (meta.height as usize) * 4;

        // Build the `GStreamer` pipeline. `decodebin` handles container demux
        // and codec detection automatically; `videoconvert` ensures we end
        // up at BGRA regardless of the source colour format.
        let location = path
            .to_str()
            .ok_or_else(|| Error::DiscoverParse(format!("non-UTF-8 path: {}", path.display())))?;
        let pipeline = format!(
            "filesrc location={location} ! decodebin ! videoconvert \
             ! video/x-raw,format=BGRA ! fdsink fd=1 sync=false"
        );

        let child = Command::new("gst-launch-1.0")
            .args(["-q", "--no-position"])
            .args(pipeline.split_whitespace())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| Error::Spawn {
                cmd: "gst-launch-1.0",
                source,
                path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".into()),
            })?;

        Ok(Self {
            child,
            width: meta.width,
            height: meta.height,
            frame_rate: meta.frame_rate,
            frame_count: meta.frame_count,
            next_index: 0,
            frame_buffer: vec![0u8; frame_size],
        })
    }
}

impl VideoStream for GstreamerPipeStream {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn frame_rate(&self) -> f32 {
        self.frame_rate
    }

    fn frame_count_hint(&self) -> Option<u64> {
        self.frame_count
    }

    fn next_frame(&mut self) -> Option<VideoFrame> {
        let stdout = self.child.stdout.as_mut()?;
        if let Err(err) = stdout.read_exact(&mut self.frame_buffer) {
            if err.kind() != ErrorKind::UnexpectedEof {
                tracing::warn!(?err, "gstreamer pipe read error");
            }
            return None;
        }
        let pts = f64::from(u32::try_from(self.next_index).unwrap_or(u32::MAX))
            / f64::from(self.frame_rate);
        let frame = VideoFrame {
            width: self.width,
            height: self.height,
            bgra: self.frame_buffer.clone(),
            pts_seconds: pts,
            frame_index: self.next_index,
        };
        self.next_index += 1;
        Some(frame)
    }
}

impl Drop for GstreamerPipeStream {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn file_uri(path: &Path) -> String {
    if let Ok(canonical) = path.canonicalize() {
        format!("file://{}", canonical.display())
    } else {
        format!("file://{}", path.display())
    }
}

/// Parse `gst-discoverer-1.0 -v` output for the first video stream.
///
/// The tool prints a tree-shaped report; we only care about a few lines.
/// Robust enough for typical MP4/MOV/MKV inputs; truly weird containers
/// can be hand-probed and the metadata fed in via a future explicit
/// constructor.
fn parse_discoverer(text: &str) -> Result<VideoMetadata> {
    let mut width: Option<u32> = None;
    let mut height: Option<u32> = None;
    let mut frame_rate: Option<f32> = None;
    let mut duration_seconds: Option<f64> = None;
    let mut in_video = false;

    for raw in text.lines() {
        let line = raw.trim();

        // Container-level duration applies to the whole file.
        if let Some(rest) = line.strip_prefix("Duration:") {
            duration_seconds = parse_clock(rest.trim());
        }

        // The discoverer prints `video: ...` headers per video stream;
        // we lock onto the first and capture its width/height/fps.
        if line.starts_with("video:") || line.starts_with("video #") {
            in_video = true;
            continue;
        }
        if in_video && (line.starts_with("audio:") || line.starts_with("subtitle:")) {
            in_video = false;
        }
        if !in_video {
            continue;
        }

        if let Some(rest) = line.strip_prefix("Width:") {
            width = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("Height:") {
            height = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("Frame rate:") {
            frame_rate = parse_rational(rest.trim());
        }
    }

    let width = width.ok_or_else(|| Error::DiscoverParse("missing Width".into()))?;
    let height = height.ok_or_else(|| Error::DiscoverParse("missing Height".into()))?;
    let frame_rate = frame_rate.ok_or_else(|| Error::DiscoverParse("missing Frame rate".into()))?;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "frame counts in practice fit in u64; rounding is acceptable for a hint"
    )]
    let frame_count = duration_seconds.map(|d| (d * f64::from(frame_rate)).round() as u64);

    Ok(VideoMetadata {
        width,
        height,
        frame_rate,
        frame_count,
    })
}

/// Parses `gst-discoverer-1.0` rationals like `30/1` or `30000/1001`.
fn parse_rational(text: &str) -> Option<f32> {
    let (num_s, den_s) = text.split_once('/')?;
    let num: f32 = num_s.trim().parse().ok()?;
    let den: f32 = den_s.trim().parse().ok()?;
    if den == 0.0 {
        return None;
    }
    Some(num / den)
}

/// Parses `gst-discoverer-1.0` `Duration:` clocks like `0:00:00.266666666`.
fn parse_clock(text: &str) -> Option<f64> {
    let mut parts = text.split(':');
    let h: f64 = parts.next()?.trim().parse().ok()?;
    let m: f64 = parts.next()?.trim().parse().ok()?;
    let s: f64 = parts.next()?.trim().parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typical_discoverer_output() {
        // Trimmed reproduction of `gst-discoverer-1.0 -v` output.
        let raw = "\
Properties:
  Duration: 0:00:00.266666666
  Seekable: yes
  container: Quicktime
    video: H.264 (High Profile)
        Width: 480
        Height: 270
        Frame rate: 30/1
        Pixel aspect ratio: 1/1
    audio: MPEG-4 AAC
        Channels: 2
";
        let m = parse_discoverer(raw).expect("parse");
        assert_eq!(m.width, 480);
        assert_eq!(m.height, 270);
        assert!((m.frame_rate - 30.0).abs() < 1e-6);
        // Duration × fps = 0.2667 × 30 = 8.0 → 8 frames.
        assert_eq!(m.frame_count, Some(8));
    }

    #[test]
    fn parses_ntsc_rational_frame_rate() {
        let raw = "video: H.264\n    Width: 640\n    Height: 480\n    Frame rate: 30000/1001\n";
        let m = parse_discoverer(raw).expect("parse");
        assert!((m.frame_rate - 29.97).abs() < 0.01, "got {}", m.frame_rate);
    }

    #[test]
    fn missing_dimensions_is_error() {
        let raw = "video: foo\n    Frame rate: 30/1\n";
        assert!(parse_discoverer(raw).is_err());
    }

    #[test]
    fn audio_block_does_not_pollute_video_metadata() {
        // If we only saw audio dimensions, we should NOT surface them as
        // video dims. (None of the audio lines we generate match Width/Height
        // anyway, but the boundary detection matters.)
        let raw = "\
audio: AAC
    Width: 999
    Height: 999
video: H.264
    Width: 320
    Height: 240
    Frame rate: 24/1
";
        let m = parse_discoverer(raw).expect("parse");
        assert_eq!(m.width, 320);
        assert_eq!(m.height, 240);
    }

    #[test]
    fn rational_with_zero_denominator_is_none() {
        assert!(parse_rational("30/0").is_none());
    }

    #[test]
    fn clock_parses_hms() {
        let v = parse_clock("0:00:00.266666666").unwrap();
        assert!((v - 0.2667).abs() < 1e-3, "{v}");
        let v2 = parse_clock("1:02:03.5").unwrap();
        assert!((v2 - 3723.5).abs() < 1e-6);
    }
}
