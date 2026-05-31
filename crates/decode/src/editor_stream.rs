//! Random-access decode for the editor (ED.3 / M-EDIT).
//!
//! [`GstreamerPipeStream`](crate::gstreamer_pipe::GstreamerPipeStream)
//! decodes forward only — fine for the recorder's playback, useless for an
//! editor that scrubs to arbitrary frames. [`EditorVideoStream`] wraps it
//! with frame-indexed seeking plus a bounded decoded-frame cache.
//!
//! ## Seek strategy (CLI-pipe constraint)
//!
//! `gst-launch-1.0` exposes no command-line seek, so a precise jump to a
//! keyframe isn't available the way `gstreamer-rs`'s `seek_simple` would
//! be. The honest v1 therefore **forward-decodes**: a seek forward keeps
//! pulling from the live pipe; a seek *backward* (before the pipe's
//! current position) re-spawns the pipe from frame 0 and decodes up to the
//! target. A bounded LRU of decoded frames makes local scrubbing and
//! repeated access cheap — and export, which walks frames in order, never
//! re-spawns. Swapping in a `gstreamer-rs` `ACCURATE` seek later is a
//! one-site change behind this type (see M-DEC.3+); [`spawn_count`] exists
//! so tests can assert the cache is doing its job.
//!
//! [`spawn_count`]: EditorVideoStream::spawn_count

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::gstreamer_pipe::{GstreamerPipeStream, Result, VideoMetadata};
use crate::{VideoFrame, VideoStream};

/// Default decoded-frame cache capacity (~10 s at 30 fps).
pub const DEFAULT_CACHE_FRAMES: usize = 300;

/// A small LRU cache of decoded frames keyed by frame index.
struct FrameCache {
    map: HashMap<u64, VideoFrame>,
    /// Frame indices in least-recently-used order (LRU at the front).
    order: Vec<u64>,
    capacity: usize,
}

impl FrameCache {
    fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    fn get(&mut self, index: u64) -> Option<VideoFrame> {
        let frame = self.map.get(&index).cloned();
        if frame.is_some() {
            self.touch(index);
        }
        frame
    }

    fn touch(&mut self, index: u64) {
        if let Some(pos) = self.order.iter().position(|&i| i == index) {
            self.order.remove(pos);
        }
        self.order.push(index);
    }

    fn put(&mut self, index: u64, frame: VideoFrame) {
        self.map.insert(index, frame);
        self.touch(index);
        while self.order.len() > self.capacity {
            let evicted = self.order.remove(0);
            self.map.remove(&evicted);
        }
    }
}

/// A frame-indexed, seekable view over a video file, backed by the
/// forward-only [`GstreamerPipeStream`] plus an LRU frame cache.
///
/// Construct with [`EditorVideoStream::open`], then pull any frame by
/// index with [`EditorVideoStream::frame`] (or seek by time with
/// [`EditorVideoStream::seek_to_time`]). Out-of-range indices clamp to the
/// last frame.
pub struct EditorVideoStream {
    path: PathBuf,
    meta: VideoMetadata,
    stream: Option<GstreamerPipeStream>,
    /// Index the underlying forward stream will produce next.
    next_index: u64,
    cache: FrameCache,
    spawn_count: u64,
}

impl std::fmt::Debug for EditorVideoStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorVideoStream")
            .field("path", &self.path)
            .field("meta", &self.meta)
            .field("next_index", &self.next_index)
            .field("spawn_count", &self.spawn_count)
            .finish_non_exhaustive()
    }
}

impl EditorVideoStream {
    /// Probe `path` and prepare a seekable stream with the default cache
    /// size. No decode pipe is spawned until the first [`frame`] call.
    ///
    /// # Errors
    ///
    /// Returns the underlying probe error if `gst-discoverer-1.0` can't
    /// read the file (missing tool, unreadable media).
    ///
    /// [`frame`]: Self::frame
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_with_cache(path, DEFAULT_CACHE_FRAMES)
    }

    /// As [`open`](Self::open) with an explicit cache capacity (frames).
    ///
    /// # Errors
    ///
    /// See [`open`](Self::open).
    pub fn open_with_cache(path: &Path, cache_frames: usize) -> Result<Self> {
        let meta = GstreamerPipeStream::probe(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            meta,
            stream: None,
            next_index: 0,
            cache: FrameCache::new(cache_frames),
            spawn_count: 0,
        })
    }

    /// Frame width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.meta.width
    }

    /// Frame height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.meta.height
    }

    /// Source frame rate (fps).
    #[must_use]
    pub fn frame_rate(&self) -> f32 {
        self.meta.frame_rate
    }

    /// Total frame count, if the container reported a duration.
    #[must_use]
    pub fn frame_count(&self) -> Option<u64> {
        self.meta.frame_count
    }

    /// How many times the underlying decode pipe has been (re)spawned.
    /// Diagnostic — used by tests to assert the cache avoids re-spawns.
    #[must_use]
    pub fn spawn_count(&self) -> u64 {
        self.spawn_count
    }

    fn last_index(&self) -> Option<u64> {
        self.meta.frame_count.map(|count| count.saturating_sub(1))
    }

    /// Return the frame at `index`, decoding (and re-spawning) as needed.
    /// Out-of-range indices clamp to the last frame. Returns `None` only
    /// when the decode pipe can't be spawned or the stream ends early.
    #[allow(
        clippy::must_use_candidate,
        reason = "frame is often called to position the stream and the returned frame ignored; the caller decides whether to use it"
    )]
    pub fn frame(&mut self, index: u64) -> Option<VideoFrame> {
        let index = match self.last_index() {
            Some(last) => index.min(last),
            None => index,
        };

        if let Some(frame) = self.cache.get(index) {
            return Some(frame);
        }

        // Re-spawn when there's no live pipe, or we've already decoded past
        // the target (can't rewind a forward-only pipe).
        if self.stream.is_none() || self.next_index > index {
            self.respawn()?;
        }

        while self.next_index <= index {
            let mut frame = self.stream.as_mut()?.next_frame()?;
            let decoded = self.next_index;
            // Re-stamp index/pts against our own clock (the underlying
            // stream restarts at 0 on each re-spawn).
            frame.frame_index = decoded;
            frame.pts_seconds = f64::from(u32::try_from(decoded).unwrap_or(u32::MAX))
                / f64::from(self.meta.frame_rate);
            self.next_index += 1;
            self.cache.put(decoded, frame.clone());
            if decoded == index {
                return Some(frame);
            }
        }
        self.cache.get(index)
    }

    /// Seek to the frame nearest `time` and return it.
    #[allow(
        clippy::must_use_candidate,
        reason = "seek may be called purely to position the stream"
    )]
    pub fn seek_to_time(&mut self, time: Duration) -> Option<VideoFrame> {
        let raw = (time.as_secs_f64() * f64::from(self.meta.frame_rate)).round();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "raw is clamped non-negative and frame indices fit u64"
        )]
        let index = if raw < 0.0 { 0 } else { raw as u64 };
        self.frame(index)
    }

    fn respawn(&mut self) -> Option<()> {
        let stream = GstreamerPipeStream::open(&self.path).ok()?;
        self.stream = Some(stream);
        self.next_index = 0;
        self.spawn_count += 1;
        Some(())
    }
}
