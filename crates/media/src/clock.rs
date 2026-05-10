//! Timestamp + clock model — shared timeline for audio, video, cursor, and
//! `wisp` overlays.
//!
//! Lands in **M-MEDIA.2 (AUT-98)**. This file is the scaffolded module so
//! the crate compiles before that chunk.
//!
//! Planned surface:
//!
//! ```text
//! MediaTime       — point on the timeline (ns-resolution f64-friendly).
//! MediaDuration   — interval between two MediaTime values.
//! MediaClock      — authoritative single timeline; assigns timestamps
//!                   to incoming chunks and frames.
//! Timestamped<T>  — generic wrapper attaching a MediaTime to T.
//! ```
//!
//! Conversion helpers will cover:
//! - sample-index ↔ MediaTime at a given sample rate (e.g. 48 kHz);
//! - frame-index ↔ MediaTime at a given frame rate (e.g. 30 fps);
//! - second-precision arithmetic for user-facing labels.
