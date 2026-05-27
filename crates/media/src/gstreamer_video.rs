//! GStreamer video capture (M-MEDIA.6 / AUT-102) — CLI-pipe pattern.
//!
//! Spawns `gst-launch-1.0` with a pipeline that emits raw BGRA frames
//! on stdout, then chunks the byte stream into [`VideoFrame`]s (the
//! same type `decode` uses).
//!
//! # Pipeline
//!
//! ```text
//! videotestsrc is-live=false
//!   ! videoconvert
//!   ! video/x-raw,format=BGRA,width=W,height=H,framerate=F/1
//!   ! fdsink fd=1
//! ```
//!
//! AUT-102 only asks for the `videotestsrc` path. M-MEDIA.16 (live
//! webcam) will add an `autovideosrc` variant; M-MEDIA.17 (playback
//! harness) will add a `filesrc ! decodebin` variant.

use std::io::{ErrorKind, Read};
use std::process::{Child, ChildStdout, Command, Stdio};

use crate::clock::MediaTime;
use crate::video::VideoFrame;

/// Failure modes for the GStreamer video capture pipe.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `gst-launch-1.0` could not be launched. The `PATH` snapshot in
    /// the message makes CI diagnoses easier.
    #[error("failed to spawn `gst-launch-1.0`: {source} (PATH={path})")]
    Spawn {
        /// The OS-level reason the spawn failed.
        #[source]
        source: std::io::Error,
        /// `$PATH` at the moment of failure.
        path: String,
    },
    /// Stdout was not piped.
    #[error("child stdout was not piped")]
    NoStdout,
    /// I/O error while reading from the child's stdout.
    #[error("read error: {0}")]
    Io(#[from] std::io::Error),
    /// Pipeline ended before the requested frame was read.
    #[error("video pipeline ended after {frames_read} frames")]
    EndOfStream {
        /// Frames actually delivered before EOF.
        frames_read: u64,
    },
    /// Dimensions or framerate would produce zero-byte frames.
    #[error("invalid format: width={width} height={height} framerate={framerate} fps")]
    InvalidFormat {
        /// Requested width in pixels.
        width: u32,
        /// Requested height in pixels.
        height: u32,
        /// Requested framerate in fps.
        framerate: f64,
    },
    /// The picker handed us a `camera_id` that no longer matches any
    /// device on the host — typically the camera was unplugged
    /// between `list_cameras()` and `from_camera()`. Callers should
    /// re-enumerate and re-prompt (M-CAM.4 / AUT — see
    /// `milestone-2-record-and-export.md`).
    #[error("camera id `{id}` not present on this host (was the camera unplugged?)")]
    CameraNotFound {
        /// The id the caller passed in.
        id: String,
    },
}

/// Streaming video capture wrapping a `gst-launch-1.0` child process.
pub struct GstreamerVideoCapture {
    child: Child,
    stdout: ChildStdout,
    width: u32,
    height: u32,
    framerate: f64,
    next_index: u64,
    /// Pre-allocated scratch buffer for the raw bytes of one frame.
    raw_buffer: Vec<u8>,
}

impl std::fmt::Debug for GstreamerVideoCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GstreamerVideoCapture")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("framerate", &self.framerate)
            .field("frames_emitted", &self.next_index)
            .finish_non_exhaustive()
    }
}

impl GstreamerVideoCapture {
    /// Build a capture from the **default OS camera** via gst's
    /// `autovideosrc` (M-CAM.0 / AUT-254).
    ///
    /// On macOS `autovideosrc` routes to `avfvideosrc`; on Linux
    /// `v4l2src`; on Windows `mfvideosrc`. Caller picks the output
    /// dimensions + framerate — gst's `videoconvert` step resizes /
    /// converts whatever the camera natively produces.
    ///
    /// ```admonish important
    /// **macOS gotcha:** `avfvideosrc` requires
    /// `NSCameraUsageDescription` in the bundled app's Info.plist.
    /// Without it the gst pipeline fails with a misleading "device
    /// busy" error AND the OS permission prompt never shows. See
    /// `crates/app/tauri.conf.json` for the project's declaration.
    /// ```
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidFormat`] if any dimension is zero.
    /// - [`Error::Spawn`] if `gst-launch-1.0` isn't on `PATH`.
    /// - [`Error::NoStdout`] if the child's stdout pipe is missing
    ///   (shouldn't happen — we request it explicitly).
    ///
    /// Cross-OS behaviour: on a host without a default camera the
    /// pipeline spawns but `next_frame` returns
    /// [`Error::EndOfStream`] as soon as the OS denies access /
    /// reports no device. Integration tests should call
    /// [`default_camera_available`] first and skip cleanly.
    pub fn from_default_camera(width: u32, height: u32, framerate: u32) -> Result<Self, Error> {
        if width == 0 || height == 0 || framerate == 0 {
            return Err(Error::InvalidFormat {
                width,
                height,
                framerate: f64::from(framerate),
            });
        }
        let caps = format!(
            "video/x-raw,format=BGRA,width={width},height={height},framerate={framerate}/1"
        );
        let mut cmd = Command::new("gst-launch-1.0");
        cmd.args(["-q", "autovideosrc"])
            .args(live_camera_tail_args(&caps))
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().map_err(|source| Error::Spawn {
            source,
            path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".into()),
        })?;
        let stdout = child.stdout.take().ok_or(Error::NoStdout)?;
        Ok(Self {
            child,
            stdout,
            width,
            height,
            framerate: f64::from(framerate),
            next_index: 0,
            raw_buffer: Vec::new(),
        })
    }

    /// Build a capture pinned to a *specific* camera resolved by its
    /// stable id (M-CAM.4). Re-probes `list_cameras()` at call time to
    /// turn `camera_id` into the OS-native source element + props
    /// (`avfvideosrc device-index=N` on macOS, `mfvideosrc
    /// device-path=...` on Windows, `v4l2src device=...` on Linux).
    ///
    /// If the camera is in the enumeration but the parser couldn't
    /// extract a `gst_source` for it (unusual — would indicate a
    /// gst-device-monitor output format the parser didn't recognise),
    /// falls back to `autovideosrc` with a `tracing::warn` so the user
    /// still sees *some* camera. The picker just won't be honored.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidFormat`] if any dimension is zero.
    /// - [`Error::CameraNotFound`] if no enumerated camera matches
    ///   `camera_id` (typical: the camera was unplugged between
    ///   `list_cameras()` and `from_camera()`).
    /// - [`Error::Spawn`] if `gst-launch-1.0` isn't on `PATH`.
    /// - [`Error::NoStdout`] if the child's stdout pipe is missing.
    pub fn from_camera(
        camera_id: &str,
        width: u32,
        height: u32,
        framerate: u32,
    ) -> Result<Self, Error> {
        if width == 0 || height == 0 || framerate == 0 {
            return Err(Error::InvalidFormat {
                width,
                height,
                framerate: f64::from(framerate),
            });
        }
        let device = crate::camera::find_by_id(camera_id).ok_or_else(|| Error::CameraNotFound {
            id: camera_id.to_string(),
        })?;
        let source_tokens: Vec<String> = if let Some(ref s) = device.gst_source {
            s.split_whitespace().map(str::to_string).collect()
        } else {
            tracing::warn!(
                camera_id = %camera_id,
                label = %device.label,
                "from_camera: device enumerated but `gst_source` was None — falling back to autovideosrc; \
                 per-device routing will NOT pin to this physical camera"
            );
            vec!["autovideosrc".to_string()]
        };
        let caps = format!(
            "video/x-raw,format=BGRA,width={width},height={height},framerate={framerate}/1"
        );
        let mut cmd = Command::new("gst-launch-1.0");
        cmd.arg("-q");
        for tok in &source_tokens {
            cmd.arg(tok);
        }
        cmd.args(live_camera_tail_args(&caps))
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        tracing::info!(
            camera_id = %camera_id,
            label = %device.label,
            source = %source_tokens.join(" "),
            "from_camera: spawning gst-launch with pinned source"
        );
        let mut child = cmd.spawn().map_err(|source| Error::Spawn {
            source,
            path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".into()),
        })?;
        let stdout = child.stdout.take().ok_or(Error::NoStdout)?;
        Ok(Self {
            child,
            stdout,
            width,
            height,
            framerate: f64::from(framerate),
            next_index: 0,
            raw_buffer: Vec::new(),
        })
    }

    /// Build a capture from `videotestsrc` at the given dimensions +
    /// framerate. The default `videotestsrc` pattern is the SMPTE
    /// colorbars — useful for visual smoke checks because every frame
    /// is visually distinct (animated subpattern).
    pub fn test_source(width: u32, height: u32, framerate: u32) -> Result<Self, Error> {
        if width == 0 || height == 0 || framerate == 0 {
            return Err(Error::InvalidFormat {
                width,
                height,
                framerate: f64::from(framerate),
            });
        }
        let caps = format!(
            "video/x-raw,format=BGRA,width={width},height={height},framerate={framerate}/1"
        );
        let mut cmd = Command::new("gst-launch-1.0");
        cmd.args([
            "-q",
            "videotestsrc",
            "is-live=false",
            "!",
            "videoconvert",
            "!",
            &caps,
            "!",
            "fdsink",
            "fd=1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
        let mut child = cmd.spawn().map_err(|source| Error::Spawn {
            source,
            path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".into()),
        })?;
        let stdout = child.stdout.take().ok_or(Error::NoStdout)?;
        Ok(Self {
            child,
            stdout,
            width,
            height,
            framerate: f64::from(framerate),
            next_index: 0,
            raw_buffer: Vec::new(),
        })
    }

    /// Width × height the captured frames will carry.
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Framerate the pipeline emits.
    #[must_use]
    pub fn framerate(&self) -> f64 {
        self.framerate
    }

    /// Cumulative frames emitted across `next_frame` calls.
    #[must_use]
    pub fn frames_emitted(&self) -> u64 {
        self.next_index
    }

    /// Read one BGRA frame. PTS is computed from the frame index and
    /// the captured framerate.
    ///
    /// # Errors
    ///
    /// - [`Error::Io`] on read failures.
    /// - [`Error::EndOfStream`] if the pipeline ends mid-frame.
    pub fn next_frame(&mut self) -> Result<VideoFrame, Error> {
        let need = usize::try_from(self.width)
            .expect("width fits usize")
            .checked_mul(usize::try_from(self.height).expect("height fits usize"))
            .and_then(|n| n.checked_mul(4))
            .ok_or(Error::InvalidFormat {
                width: self.width,
                height: self.height,
                framerate: self.framerate,
            })?;
        if self.raw_buffer.len() < need {
            self.raw_buffer.resize(need, 0);
        }
        let slice = &mut self.raw_buffer[..need];
        let mut read = 0;
        while read < need {
            match self.stdout.read(&mut slice[read..]) {
                Ok(0) => {
                    return Err(Error::EndOfStream {
                        frames_read: self.next_index,
                    });
                }
                Ok(n) => read += n,
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(Error::Io(e)),
            }
        }
        let frame = VideoFrame {
            width: self.width,
            height: self.height,
            bgra: slice.to_vec(),
            pts_seconds: MediaTime::from_frame(self.next_index, self.framerate).as_seconds(),
            frame_index: self.next_index,
        };
        self.next_index = self.next_index.saturating_add(1);
        Ok(frame)
    }
}

fn live_camera_tail_args(caps: &str) -> [&str; 12] {
    [
        "!",
        "videoconvert",
        "!",
        // M-QUAL.3 — center-crop the webcam's native 16:9 (or other)
        // frame to 1:1 BEFORE scaling to the square preview caps, so
        // the circular bubble shows an undistorted face. Without this
        // `videoscale` squishes the full frame into the square (a
        // horizontally-compressed face). `aspectratiocrop` is in
        // gst-plugins-good, shipped with every `gstreamer` install.
        "aspectratiocrop",
        "aspect-ratio=1/1",
        "!",
        "videoscale",
        "!",
        caps,
        "!",
        "fdsink",
        "fd=1",
    ]
}

impl Drop for GstreamerVideoCapture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Best-effort probe for "is there at least one video capture device
/// the OS will hand us?" (M-CAM.0 / AUT-254).
///
/// Spawns `gst-device-monitor-1.0 Video/Source` with a short timeout
/// and parses the output for at least one device line. Returns `false`
/// if the binary isn't on `PATH`, returns `false` if no devices are
/// listed — never panics. Integration tests use this to skip cleanly
/// on a host without a webcam.
#[must_use]
pub fn default_camera_available() -> bool {
    let output = Command::new("gst-device-monitor-1.0")
        .args(["Video/Source"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // The text format has `Device found:` lines, one per
            // device. Any match = at least one camera.
            stdout.contains("Device found:")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_dimensions_rejected_at_construction() {
        assert!(matches!(
            GstreamerVideoCapture::test_source(0, 360, 30),
            Err(Error::InvalidFormat { width: 0, .. })
        ));
        assert!(matches!(
            GstreamerVideoCapture::test_source(640, 0, 30),
            Err(Error::InvalidFormat { height: 0, .. })
        ));
        assert!(matches!(
            GstreamerVideoCapture::test_source(640, 360, 0),
            Err(Error::InvalidFormat { framerate, .. }) if framerate.abs() < 1e-9
        ));
    }

    #[test]
    fn capture_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<GstreamerVideoCapture>();
    }

    #[test]
    fn live_camera_pipeline_crops_then_scales_before_square_caps() {
        let caps = "video/x-raw,format=BGRA,width=720,height=720,framerate=30/1";
        let args = live_camera_tail_args(caps);
        let crop_pos = args
            .iter()
            .position(|arg| *arg == "aspectratiocrop")
            .expect("live camera pipeline should center-crop to square (M-QUAL.3)");
        let scale_pos = args
            .iter()
            .position(|arg| *arg == "videoscale")
            .expect("live camera pipeline should include videoscale");
        let caps_pos = args
            .iter()
            .position(|arg| arg.starts_with("video/x-raw"))
            .expect("live camera pipeline should include raw caps");

        // Order matters: crop the native frame to 1:1 first (undistorted
        // face), THEN scale to the square preview caps. Cropping after
        // the squish-scale would be too late.
        assert!(
            crop_pos < scale_pos && scale_pos < caps_pos,
            "expected aspectratiocrop → videoscale → caps, got {args:?}"
        );
        // The crop must request a 1:1 aspect, else it's a no-op.
        assert!(
            args.contains(&"aspect-ratio=1/1"),
            "aspectratiocrop must target 1/1: {args:?}"
        );
    }
}
