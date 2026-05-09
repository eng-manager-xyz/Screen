//! Deterministic mock video sources.
//!
//! These implement [`crate::VideoStream`] without any external decoder
//! dependency, so we can:
//!
//! - exercise the player loop end-to-end in tests,
//! - drive wisp examples that prove the GPU path,
//! - keep `cargo check` / `just gate` clean on machines without a system
//!   `FFmpeg` / `AVFoundation` toolchain.

use crate::{VideoFrame, VideoStream};

/// A scrolling-gradient mock: each frame phase-shifts a smooth RGB gradient
/// across the surface. Visually obvious motion; mathematically deterministic.
pub struct MockVideoStream {
    width: u32,
    height: u32,
    frame_rate: f32,
    total_frames: u64,
    next_index: u64,
}

impl MockVideoStream {
    /// Build a scrolling-gradient stream with the given dimensions and
    /// total frame count. Plays at 30 fps.
    #[must_use]
    pub fn scrolling_gradient(width: u32, height: u32, total_frames: u64) -> Self {
        Self {
            width,
            height,
            frame_rate: 30.0,
            total_frames,
            next_index: 0,
        }
    }

    /// Override the frame rate (used by player-pacing tests).
    #[must_use]
    pub fn with_frame_rate(mut self, fps: f32) -> Self {
        self.frame_rate = fps;
        self
    }
}

impl VideoStream for MockVideoStream {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn frame_rate(&self) -> f32 {
        self.frame_rate
    }

    fn frame_count_hint(&self) -> Option<u64> {
        Some(self.total_frames)
    }

    fn next_frame(&mut self) -> Option<VideoFrame> {
        if self.next_index >= self.total_frames {
            return None;
        }
        let frame = synthesize(
            self.width,
            self.height,
            self.next_index,
            self.total_frames,
            self.frame_rate,
        );
        self.next_index += 1;
        Some(frame)
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    reason = "byte values bounded by rem_euclid(256.0); single-letter names match gradient math conventions"
)]
fn synthesize(width: u32, height: u32, idx: u64, total: u64, fps: f32) -> VideoFrame {
    let mut bgra = Vec::with_capacity((width * height * 4) as usize);
    let phase = (idx as f32) / (total.max(1) as f32);
    for y in 0..height {
        for x in 0..width {
            let u = (x as f32) / (width as f32);
            let v = (y as f32) / (height as f32);
            let r = ((u + phase) * 255.0).rem_euclid(256.0) as u8;
            let g = ((v + phase) * 255.0).rem_euclid(256.0) as u8;
            let b = (((u + v + phase) * 0.5) * 255.0).rem_euclid(256.0) as u8;
            // BGRA8 packing: B, G, R, A.
            bgra.push(b);
            bgra.push(g);
            bgra.push(r);
            bgra.push(255);
        }
    }
    VideoFrame {
        width,
        height,
        bgra,
        pts_seconds: f64::from(idx as u32) / f64::from(fps),
        frame_index: idx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_stream_yields_exact_count_then_terminates() {
        let mut s = MockVideoStream::scrolling_gradient(8, 8, 4);
        for expected in 0..4 {
            let f = s.next_frame().expect("frame");
            assert_eq!(f.frame_index, expected);
        }
        assert!(s.next_frame().is_none());
    }

    #[test]
    fn frame_dimensions_match_buffer_length() {
        let mut s = MockVideoStream::scrolling_gradient(16, 9, 1);
        let f = s.next_frame().unwrap();
        assert_eq!(f.width, 16);
        assert_eq!(f.height, 9);
        assert_eq!(f.bgra.len(), (16 * 9 * 4) as usize);
    }

    #[test]
    fn frame_rate_drives_pts() {
        let mut s = MockVideoStream::scrolling_gradient(4, 4, 3).with_frame_rate(60.0);
        let f0 = s.next_frame().unwrap();
        let f1 = s.next_frame().unwrap();
        let f2 = s.next_frame().unwrap();
        assert!((f0.pts_seconds - 0.0).abs() < 1e-9);
        assert!((f1.pts_seconds - (1.0 / 60.0)).abs() < 1e-9);
        assert!((f2.pts_seconds - (2.0 / 60.0)).abs() < 1e-9);
    }

    #[test]
    fn frame_count_hint_matches_total() {
        let s = MockVideoStream::scrolling_gradient(2, 2, 42);
        assert_eq!(s.frame_count_hint(), Some(42));
    }

    #[test]
    fn motion_is_visible_between_frames() {
        // Adjacent frames must differ; a static stream would be a regression.
        let mut s = MockVideoStream::scrolling_gradient(32, 32, 4);
        let f0 = s.next_frame().unwrap();
        let f1 = s.next_frame().unwrap();
        assert_ne!(
            f0.bgra, f1.bgra,
            "scrolling gradient must move between frames"
        );
    }
}
