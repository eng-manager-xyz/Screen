//! Audio histogram quantization — turns `AudioChunk` samples into
//! design-friendly rectangle bars for timeline / dope-sheet visualization.
//!
//! Lands in **M-MEDIA.8 (AUT-104)**. This file is the scaffolded module so
//! the crate compiles before that chunk.
//!
//! Planned surface:
//!
//! ```text
//! AudioHistogram {
//!   bucket_duration: MediaDuration,
//!   bars:            Vec<AudioBar>,
//! }
//!
//! AudioBar {
//!   start_time: MediaTime,
//!   duration:   MediaDuration,
//!   peak:       f32,      // max |sample| in the bucket
//!   rms:        f32,      // root-mean-square over the bucket
//! }
//!
//! quantize(chunk, bucket_ms) -> AudioHistogram
//! ```
//!
//! Default bucket sizes target the 20–50 ms range (matches dope-sheet
//! readability). 10 ms is supported for fine-grained tests. M-MEDIA.9
//! converts an `AudioHistogram` into Wisp-renderable `WaveformBarRect`
//! geometry.
