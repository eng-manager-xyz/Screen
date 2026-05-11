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
}
