//! GStreamer audio capture (M-MEDIA.5 / AUT-101) — CLI-pipe pattern.
//!
//! Spawns `gst-launch-1.0` with a pipeline that emits normalized
//! `F32LE` raw audio on stdout, then chunks the byte stream into
//! [`AudioChunk`]s.
//!
//! # Two source modes
//!
//! ```text
//! GstreamerAudioCapture::test_source(format, freq_hz)
//!   → audiotestsrc wave=sine freq=F
//!     ! audioconvert
//!     ! audioresample
//!     ! audio/x-raw,format=F32LE,rate=R,channels=C
//!     ! fdsink fd=1
//!
//! GstreamerAudioCapture::from_file(path, format)
//!   → filesrc location=PATH
//!     ! decodebin
//!     ! audioconvert
//!     ! audioresample
//!     ! audio/x-raw,format=F32LE,rate=R,channels=C
//!     ! fdsink fd=1
//! ```
//!
//! `test_source` is the AUT-101 deliverable. `from_file` is the
//! companion fixture path used by M-MEDIA.8+ integration tests so the
//! histogram + waveform code runs against real audio without
//! depending on a microphone.
//!
//! # Lifecycle
//!
//! `Drop` kills the child + waits — without this, `gst-launch-1.0`
//! keeps decoding into a dropped pipe and burns CPU. Matches the
//! `decode::GstreamerPipeStream` pattern.

use std::io::{ErrorKind, Read};
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};

use crate::audio::{AudioChunk, AudioChunkError, AudioFormat, SampleFormat};
use crate::clock::MediaTime;

/// Failure modes for the GStreamer audio capture pipe.
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
    /// Stdout was not piped — shouldn't happen, indicates a misuse.
    #[error("child stdout was not piped")]
    NoStdout,
    /// I/O error while reading from the child's stdout.
    #[error("read error: {0}")]
    Io(#[from] std::io::Error),
    /// Pipeline ended (EOF) before the requested frames were read.
    #[error("audio pipeline ended after {frames_read} of {frames_requested} frames")]
    EndOfStream {
        /// Frames actually delivered before EOF.
        frames_read: u64,
        /// Frames the caller asked for.
        frames_requested: u64,
    },
    /// Constructed an `AudioChunk` that failed shape validation —
    /// internal bug.
    #[error("internal: built invalid AudioChunk: {0}")]
    InvalidChunk(#[from] AudioChunkError),
    /// Format is not currently supported by the capture pipeline.
    /// Only `SampleFormat::F32` is supported because the pipeline
    /// caps it to `F32LE` explicitly.
    #[error("unsupported sample format {0:?} — only F32 is supported")]
    UnsupportedFormat(SampleFormat),
}

/// Streaming audio capture wrapping a `gst-launch-1.0` child process.
///
/// Each [`Self::next_chunk`] call reads exactly `frames` frames of
/// audio from the child's stdout, packages them as a normalized-`f32`
/// [`AudioChunk`], and assigns the appropriate PTS so the chunks form
/// a contiguous timeline.
pub struct GstreamerAudioCapture {
    child: Child,
    stdout: ChildStdout,
    format: AudioFormat,
    next_frame: u64,
    /// Pre-allocated scratch buffer for the raw bytes of one chunk.
    /// Sized lazily on first `next_chunk`; reused after.
    raw_buffer: Vec<u8>,
}

impl std::fmt::Debug for GstreamerAudioCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GstreamerAudioCapture")
            .field("format", &self.format)
            .field("frames_emitted", &self.next_frame)
            .finish_non_exhaustive()
    }
}

impl GstreamerAudioCapture {
    /// Build a capture from `audiotestsrc` at the given sine
    /// `frequency_hz`. AUT-101's "deterministic GStreamer source"
    /// path.
    pub fn test_source(format: AudioFormat, frequency_hz: f32) -> Result<Self, Error> {
        Self::reject_non_f32(format)?;
        let caps = caps_string(format);
        let freq_arg = format!("freq={frequency_hz}");
        Self::spawn(
            &[
                "-q",
                "audiotestsrc",
                "wave=sine",
                &freq_arg,
                "is-live=false",
                "!",
                "audioconvert",
                "!",
                "audioresample",
                "!",
                &caps,
                "!",
                "fdsink",
                "fd=1",
            ],
            format,
        )
    }

    /// Build a capture by decoding an audio file through GStreamer.
    /// Companion to [`Self::test_source`] for tests that want to
    /// exercise the histogram / waveform code against real audio.
    pub fn from_file(path: &Path, format: AudioFormat) -> Result<Self, Error> {
        Self::reject_non_f32(format)?;
        let caps = caps_string(format);
        let location = format!("location={}", path.display());
        Self::spawn(
            &[
                "-q",
                "filesrc",
                &location,
                "!",
                "decodebin",
                "!",
                "audioconvert",
                "!",
                "audioresample",
                "!",
                &caps,
                "!",
                "fdsink",
                "fd=1",
            ],
            format,
        )
    }

    /// Build a capture from a specific microphone via the
    /// per-OS gst element (M-MIC.1 / AUT-278 + M-MIC.3 / AUT-284).
    ///
    /// - macOS: `osxaudiosrc unique-id=<native_id>`
    /// - Linux: `pulsesrc device=<native_id>`
    /// - Windows: `wasapisrc device=<native_id>`
    ///
    /// When `native_id` is empty (the device didn't expose
    /// `unique-id` in gst-device-monitor output, OR the caller
    /// wants the OS default), the pipeline falls back to
    /// `autoaudiosrc` which opens the OS default mic.
    ///
    /// `format` must use [`SampleFormat::F32`] — the pipeline caps
    /// to `F32LE` explicitly. Non-`F32` rejects at construction (same
    /// shape as [`Self::test_source`]).
    ///
    /// ```admonish note title="Format choice differs from the ticket prose"
    /// AUT-278 originally described the pipeline as
    /// `…audio/x-raw,format=S16LE,…`. The capture infra around this
    /// type is F32-only (see [`Self::reject_non_f32`] and
    /// [`AudioChunk`] normalisation); reusing it required F32LE.
    /// `audioresample` + `audioconvert` in the pipeline handle the
    /// downstream conversion when a future encoder wants S16.
    /// ```
    pub fn from_microphone(
        mic_id: &str,
        native_id: &str,
        format: AudioFormat,
    ) -> Result<Self, Error> {
        Self::reject_non_f32(format)?;
        let caps = caps_string(format);
        let mut args: Vec<String> = vec!["-q".to_string()];
        if let Some((element, prop)) = resolve_mic_element(native_id) {
            let prop_arg = format!("{prop}={native_id}");
            args.push(element.to_string());
            args.push(prop_arg);
            tracing::info!(
                mic_id,
                native_id,
                element,
                sample_rate = format.sample_rate,
                channels = format.channels,
                "from_microphone: spawning gst-launch with per-device element"
            );
        } else {
            args.push("autoaudiosrc".to_string());
            tracing::info!(
                mic_id,
                has_native_id = !native_id.is_empty(),
                sample_rate = format.sample_rate,
                channels = format.channels,
                "from_microphone: spawning gst-launch (autoaudiosrc — OS default)"
            );
        }
        args.extend(
            [
                "!",
                "audioconvert",
                "!",
                "audioresample",
                "!",
                &caps,
                "!",
                "fdsink",
                "fd=1",
            ]
            .iter()
            .map(|s| (*s).to_string()),
        );
        let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
        Self::spawn(&args_ref, format)
    }

    /// Format of the produced chunks (same as the format passed at
    /// construction).
    #[must_use]
    pub fn format(&self) -> AudioFormat {
        self.format
    }

    /// Cumulative frames emitted across `next_chunk` calls. Used by
    /// the sync harness (M-MEDIA.7) to assert against expected
    /// per-source counts.
    #[must_use]
    pub fn frames_emitted(&self) -> u64 {
        self.next_frame
    }

    /// Read exactly `frames` frames of audio. Returns an
    /// [`AudioChunk`] with the PTS pointing at the first sample.
    ///
    /// # Errors
    ///
    /// - [`Error::Io`] on read failures.
    /// - [`Error::EndOfStream`] if the pipeline ends before `frames`
    ///   are delivered (returns the partial count for diagnosis).
    pub fn next_chunk(&mut self, frames: u64) -> Result<AudioChunk, Error> {
        let bytes_per_frame = usize::from(self.format.channels) * 4; // f32 = 4 bytes
        let frames_usize = usize::try_from(frames).expect("frames fits usize");
        let need = frames_usize * bytes_per_frame;
        if self.raw_buffer.len() < need {
            self.raw_buffer.resize(need, 0);
        }
        let slice = &mut self.raw_buffer[..need];
        let mut read = 0;
        while read < need {
            match self.stdout.read(&mut slice[read..]) {
                Ok(0) => {
                    let read_frames = (read / bytes_per_frame) as u64;
                    return Err(Error::EndOfStream {
                        frames_read: self.next_frame + read_frames,
                        frames_requested: self.next_frame + frames,
                    });
                }
                Ok(n) => read += n,
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(Error::Io(e)),
            }
        }

        // Decode little-endian f32 samples.
        let mut samples = Vec::with_capacity(frames_usize * usize::from(self.format.channels));
        for chunk in slice.chunks_exact(4) {
            let arr: [u8; 4] = chunk.try_into().expect("chunks_exact(4) yields [u8; 4]");
            samples.push(f32::from_le_bytes(arr));
        }

        let pts = MediaTime::from_sample(self.next_frame, self.format.sample_rate);
        let out = AudioChunk::new(self.format, samples, pts)?;
        self.next_frame = self.next_frame.saturating_add(frames);
        Ok(out)
    }

    fn spawn(args: &[&str], format: AudioFormat) -> Result<Self, Error> {
        let mut cmd = Command::new("gst-launch-1.0");
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::null());
        let mut child = cmd.spawn().map_err(|source| Error::Spawn {
            source,
            path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".into()),
        })?;
        let stdout = child.stdout.take().ok_or(Error::NoStdout)?;
        Ok(Self {
            child,
            stdout,
            format,
            next_frame: 0,
            raw_buffer: Vec::new(),
        })
    }

    fn reject_non_f32(format: AudioFormat) -> Result<(), Error> {
        if !matches!(format.sample_format, SampleFormat::F32) {
            return Err(Error::UnsupportedFormat(format.sample_format));
        }
        Ok(())
    }
}

impl Drop for GstreamerAudioCapture {
    fn drop(&mut self) {
        // Kill the child + wait; without this gst-launch-1.0 keeps
        // decoding into a dropped pipe.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn caps_string(format: AudioFormat) -> String {
    // Audio raw caps. F32LE means little-endian normalized float —
    // matches AudioChunk::samples shape, no extra conversion.
    format!(
        "audio/x-raw,format=F32LE,rate={rate},channels={channels},layout=interleaved",
        rate = format.sample_rate,
        channels = format.channels,
    )
}

/// Pick the per-OS gst element + property name that takes the
/// device's native identifier (M-MIC.3 / AUT-284). Returns `None`
/// when `native_id` is empty OR when the current target OS isn't
/// one we know an element name for — callers fall back to
/// `autoaudiosrc` in that case.
///
/// | OS      | Element        | Property     |
/// | ------- | -------------- | ------------ |
/// | macOS   | `osxaudiosrc`  | `unique-id`  |
/// | Linux   | `pulsesrc`     | `device`     |
/// | Windows | `wasapisrc`    | `device`     |
///
/// On macOS the string device-selection prop is `unique-id`, NOT
/// `device-uid` — the latter is not a property of `osxaudiosrc` and
/// makes `gst-launch-1.0` reject the pipeline with zero bytes of
/// audio output, which the mic worker observes as `EndOfStream` on
/// the first read.
#[must_use]
pub fn resolve_mic_element(native_id: &str) -> Option<(&'static str, &'static str)> {
    if native_id.is_empty() {
        return None;
    }
    #[cfg(target_os = "macos")]
    {
        Some(("osxaudiosrc", "unique-id"))
    }
    #[cfg(target_os = "linux")]
    {
        Some(("pulsesrc", "device"))
    }
    #[cfg(target_os = "windows")]
    {
        Some(("wasapisrc", "device"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_string_includes_rate_channels_format() {
        let s = caps_string(AudioFormat::stereo_f32(48_000));
        assert!(s.contains("rate=48000"));
        assert!(s.contains("channels=2"));
        assert!(s.contains("format=F32LE"));
    }

    #[test]
    fn non_f32_format_is_rejected_at_construction() {
        let fmt = AudioFormat {
            sample_rate: 48_000,
            channels: 1,
            sample_format: SampleFormat::I16,
        };
        let err = GstreamerAudioCapture::test_source(fmt, 440.0).unwrap_err();
        assert!(matches!(err, Error::UnsupportedFormat(SampleFormat::I16)));
    }

    // Send + Sync — capture can be passed across threads as long as
    // the consumer takes &mut.
    #[test]
    fn capture_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<GstreamerAudioCapture>();
    }

    /// Anti-regression: `osxaudiosrc` has no `device-uid` property
    /// (its string device-selection prop is `unique-id`). The wrong
    /// name makes gst-launch reject the pipeline at parse time and
    /// produces zero audio bytes — observed downstream as a silent
    /// recording with no audio stream in the final `.mp4`.
    #[cfg(target_os = "macos")]
    #[test]
    fn resolve_mic_element_macos_returns_unique_id_not_device_uid() {
        let (element, prop) = resolve_mic_element("BuiltInMicrophoneDevice").expect("macOS arm");
        assert_eq!(element, "osxaudiosrc");
        assert_eq!(prop, "unique-id");
        assert_ne!(prop, "device-uid");
    }

    #[test]
    fn resolve_mic_element_empty_native_id_returns_none() {
        assert!(resolve_mic_element("").is_none());
    }
}
