//! Shared GStreamer probe + pipeline helpers.
//!
//! Lands in **M-MEDIA.1 (AUT-97)**. This file is the scaffolded module so
//! the crate compiles before that chunk.
//!
//! Planned surface:
//!
//! ```text
//! GStreamerProbe {
//!   gst_launch:    Option<PathBuf>,
//!   gst_discover:  Option<PathBuf>,
//!   path_snapshot: String,
//!   plugins:       HashMap<String, bool>,
//! }
//!
//! probe()                  -> GStreamerProbe   // structured diagnostic.
//! is_available()           -> bool             // shortcut for skip-guards.
//! check_plugin(name: &str) -> bool             // optional plugin checks.
//! ```
//!
//! The existing `decode::gstreamer_pipe::gstreamer_available()` is the
//! seed; M-MEDIA.1 moves it here and refactors its callers (decode +
//! preview + app integration tests) to consume the structured form.
//! Production errors still include `PATH=...` so CI mysteries are
//! debuggable.
