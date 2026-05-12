//! `decode` — video decode → BGRA frames, trait-based.
//!
//! # Why a trait
//!
//! The recorder decodes MP4s through `GStreamer` today (the workspace's
//! single media stack — see CLAUDE.md "Stack" section and AUT-144).
//! Multiple backends still exist as a contract: the [`gstreamer_pipe`]
//! CLI-subprocess implementation, a future in-process `gstreamer-rs`
//! bindings implementation for encode (`appsrc`-fed), and the [`mock`]
//! deterministic test source. The *consumer* (wisp's
//! `VideoTexture::upload_bgra` path) is uniform: it wants a stream of
//! BGRA frames at known dimensions, ticked at known timestamps.
//!
//! [`VideoStream`] is that uniform contract. Any implementation can be
//! swapped in at runtime without changing the player loop.
//!
//! # Layers
//!
//! - [`VideoStream`] — pull-based interface returning [`VideoFrame`] values.
//! - [`mock::MockVideoStream`] — deterministic test source. No external
//!   deps; useful for tests and as the harness target for examples until
//!   M-DEC.2 wires in a real codec.
//!
//! # Quick start
//!
//! ```rust
//! use decode::{VideoFrame, VideoStream, mock::MockVideoStream};
//!
//! let mut stream = MockVideoStream::scrolling_gradient(64, 36, 8);
//! let frame: VideoFrame = stream.next_frame().expect("first frame");
//! assert_eq!(frame.width, 64);
//! assert_eq!(frame.bgra.len(), (64 * 36 * 4) as usize);
//! ```

pub mod gstreamer_pipe;
pub mod mock;

/// One decoded frame in BGRA8 layout, the canonical wgpu input format for
/// per-frame texture uploads.
#[derive(Clone)]
pub struct VideoFrame {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Tightly packed BGRA bytes, row-major. Length must equal
    /// `(width * height * 4) as usize`.
    pub bgra: Vec<u8>,
    /// Presentation timestamp in seconds since the start of the stream.
    pub pts_seconds: f64,
    /// Zero-based frame index (monotonic across the stream).
    pub frame_index: u64,
}

impl std::fmt::Debug for VideoFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoFrame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bgra_len", &self.bgra.len())
            .field("pts_seconds", &self.pts_seconds)
            .field("frame_index", &self.frame_index)
            .finish()
    }
}

/// Pull-based source of decoded video frames.
///
/// Implementations may be lazy (stream from disk on each `next_frame`) or
/// eager (decode the whole stream up front). The contract is just:
/// successive calls return successive frames; `None` signals end-of-stream.
///
/// Frame dimensions are reported by [`width`](Self::width) /
/// [`height`](Self::height) and must remain constant across the stream
/// (we don't support resolution changes mid-playback in v1).
pub trait VideoStream {
    /// Frame width in pixels. Constant across the stream.
    fn width(&self) -> u32;

    /// Frame height in pixels. Constant across the stream.
    fn height(&self) -> u32;

    /// Source frame rate (frames per second). Used by the player loop to
    /// time uploads; backends without a known fixed rate may return their
    /// nominal rate (e.g. `AVFoundation`'s `nominalFrameRate` on the track).
    fn frame_rate(&self) -> f32;

    /// Pull the next frame. Returns `None` at end-of-stream.
    fn next_frame(&mut self) -> Option<VideoFrame>;

    /// Total frame count, if known up-front. `None` for live streams.
    fn frame_count_hint(&self) -> Option<u64> {
        None
    }
}
