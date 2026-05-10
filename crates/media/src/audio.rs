//! Audio sample + chunk data model.
//!
//! Lands in **M-MEDIA.3 (AUT-99)**. This file is the scaffolded module so
//! the crate compiles before that chunk.
//!
//! Planned surface:
//!
//! ```text
//! AudioFormat {
//!   sample_rate: u32,
//!   channels:    u8,
//!   sample_format: SampleFormat,   // F32, I16, U8…
//! }
//!
//! AudioChunk {
//!   format: AudioFormat,
//!   samples: Vec<f32>,             // normalized
//!   pts_seconds: f64,
//!   duration_seconds: f64,
//! }
//! ```
//!
//! Validation: `samples.len() % channels == 0` and
//! `duration_seconds == samples.len() / channels / sample_rate`. Invalid
//! shapes return [`crate::Error::InvalidChunkShape`] (forthcoming).
