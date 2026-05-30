//! `EditorPreview` — composes the frame at the playhead for the editor's
//! preview surface (ED.6 / M-EDIT).
//!
//! Reuses [`RecordingCompose`](crate::recording_compose::RecordingCompose)
//! — the proven recorder compositor — but sourced from a seekable
//! [`EditorVideoStream`](decode::EditorVideoStream) at the editor's
//! playhead ([`EditorPlayer::current_frame`](playback::EditorPlayer::current_frame))
//! rather than from live capture slots. This is the **same compose path**
//! the export pipeline (ED.20) will drive, so preview and export agree
//! frame-for-frame by construction.
//!
//! The recorded clip is already a fully-composited screen frame (the cam
//! bubble, if any, was baked in at record time), so the editor preview
//! shows it full-frame; the cam channel of the underlying scene is unused.
//! The cinematic framing controls (background, padding, rounded corners,
//! shadow) layer on in ED.18. The live `winit` window that presents these
//! composed frames follows the `preview` crate's pattern and is verified
//! manually (it can't run in the headless gate).

use std::sync::{Arc, Mutex};

use decode::EditorVideoStream;
use playback::EditorPlayer;
use wisp::recording::StreamDimensions;

use crate::recording::FrameSlot;
use crate::recording_compose::{ComposedFrame, RecordingCompose};

/// Composes the editor preview frame for a clip of `width × height`.
pub struct EditorPreview {
    compose: RecordingCompose,
    screen_slot: FrameSlot,
    /// Always empty — the editor source is pre-composited, so the scene's
    /// camera channel never renders.
    cam_slot: FrameSlot,
    width: u32,
    height: u32,
}

impl EditorPreview {
    /// Allocate the compose pipeline for a clip of the given dimensions.
    ///
    /// # Errors
    ///
    /// Returns the underlying [`wisp::Error`] if the wgpu device can't be
    /// created.
    pub fn new(width: u32, height: u32) -> Result<Self, wisp::Error> {
        let screen = StreamDimensions::new(width, height);
        // `RecordingScene::new` requires non-zero cam dims even though the
        // cam never renders here; a 2×2 placeholder is the cheapest legal
        // texture. No cam frame is ever uploaded, so it stays hidden.
        let cam = StreamDimensions::new(2, 2);
        let compose = RecordingCompose::new(width, height, screen, cam)?;
        Ok(Self {
            compose,
            screen_slot: Arc::new(Mutex::new(None)),
            cam_slot: Arc::new(Mutex::new(None)),
            width,
            height,
        })
    }

    /// Compose a single source frame. `bgra` is top-down packed BGRA8 of
    /// exactly `width * height * 4` bytes (the decoder's native output);
    /// the scene flips it to wisp's convention internally. Returns `None`
    /// if the byte count doesn't match the configured dimensions.
    #[must_use]
    pub fn render_frame(&mut self, bgra: Vec<u8>) -> Option<ComposedFrame> {
        {
            let mut guard = self
                .screen_slot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *guard = Some(bgra);
        }
        self.compose
            .compose_frame(&self.cam_slot, &self.screen_slot)
    }

    /// Compose the frame at the player's current playhead, pulling it from
    /// the seekable stream. Returns `None` past the end of the stream.
    #[must_use]
    pub fn render_at(
        &mut self,
        stream: &mut EditorVideoStream,
        player: &EditorPlayer,
    ) -> Option<ComposedFrame> {
        let frame = stream.frame(player.current_frame())?;
        self.render_frame(frame.bgra)
    }

    /// Output dimensions in pixels.
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_source_frame_to_composed_bgra() {
        let mut preview = EditorPreview::new(64, 64).expect("init wgpu");
        assert_eq!(preview.dimensions(), (64, 64));
        // A mid-grey source frame composes to a non-empty BGRA buffer of
        // the configured size (proves the wisp render path ran cleanly).
        let composed = preview
            .render_frame(vec![128u8; 64 * 64 * 4])
            .expect("frame composed");
        assert_eq!(composed.width, 64);
        assert_eq!(composed.height, 64);
        assert_eq!(composed.bytes.len(), 64 * 64 * 4);
    }

    #[test]
    fn wrong_sized_frame_is_dropped() {
        let mut preview = EditorPreview::new(64, 64).expect("init wgpu");
        // A byte count that doesn't match 64×64×4 is dropped by the
        // compositor → no composed frame.
        assert!(preview.render_frame(vec![0u8; 100]).is_none());
    }
}
