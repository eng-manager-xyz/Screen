//! Recording session manifest.
//!
//! Lands in **M-MEDIA.20 (AUT-116)**. This file is the scaffolded module
//! so the crate compiles before that chunk.
//!
//! Planned surface:
//!
//! ```text
//! RecordingManifest {
//!   session_id:      Uuid,
//!   created_at:      DateTime<Utc>,
//!   video_tracks:    Vec<VideoTrack>,
//!   audio_tracks:    Vec<AudioTrack>,
//!   cursor_track:    Option<CursorTrack>,
//!   timeline_origin: MediaTime,
//!   duration:        MediaDuration,
//! }
//! ```
//!
//! Serializable (serde JSON + a stable on-disk format). Describes the
//! session — does not render it. The app reopens a recording by loading a
//! manifest + lazily resolving track files relative to the manifest's
//! parent directory.
