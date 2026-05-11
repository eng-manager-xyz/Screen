//! Video frame surface — re-exports `decode`'s `VideoFrame` + `VideoStream`
//! so M-MEDIA consumers don't pull `decode` directly.
//!
//! The frame model is locked at the `decode` crate (BGRA8, per-frame PTS,
//! frame index, dimensions). M-MEDIA chunks .6 / .13 / .16 add capture
//! pipelines that emit through this contract; M-MEDIA.12 adds the Wisp
//! upload path.

pub use decode::{VideoFrame, VideoStream};
