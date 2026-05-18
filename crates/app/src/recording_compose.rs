//! `RecordingCompose` — single-threaded wisp render + wgpu readback
//! pump for M-PIX.5 of M-RECORD-EXPORT-REAL-PIXELS.
//!
//! Owns the wisp `Application` + `Renderer` + `RecordingScene` +
//! the BGRA `RenderTexture` target. On each tick:
//!
//! 1. Pull the latest BGRA from the camera + screen
//!    [`crate::recording::FrameSlot`]s (latest-frame-wins).
//! 2. Upload to the scene's `VideoTexture`s.
//! 3. `Renderer::render_stage` into the BGRA `RenderTexture`.
//! 4. `RenderTexture::read_pixels` to pull the composed BGRA bytes
//!    back to CPU (handles the staging buffer + row-stride math).
//!
//! Wgpu objects all live on this single thread — no Send / Sync
//! gymnastics. The thread that owns `RecordingCompose` is also the
//! thread that pushes frames into the encoder (M-PIX.6); the two
//! steps are fused so there's no inter-thread channel between
//! "BGRA produced" and "BGRA encoded."

use wisp::application::{AppConfig, Application};
use wisp::color::Color;
use wisp::recording::{CamLayout, RecordingScene, StreamDimensions};
use wisp::render::Renderer;
use wisp::texture::render_texture::RenderTexture;

use crate::recording::FrameSlot;

/// Result of a single compose tick. `bytes` is BGRA8 packed
/// (no row padding) sized exactly `width * height * 4`.
#[derive(Debug)]
pub struct ComposedFrame {
    /// Tightly-packed BGRA8 bytes.
    pub bytes: Vec<u8>,
    /// Frame dimensions in pixels (matches the encoder caps).
    pub width: u32,
    /// Frame dimensions in pixels.
    pub height: u32,
}

/// Owner of the wisp + wgpu pipeline. Single-thread. Construct on
/// the encoder feed thread + call [`Self::compose_frame`] once per
/// frame.
pub struct RecordingCompose {
    app: Application,
    renderer: Renderer,
    scene: RecordingScene,
    target: RenderTexture,
    width: u32,
    height: u32,
    /// Tracks whether we've ever uploaded a camera frame. Used so
    /// the scene doesn't render a stale-empty `VideoTexture` cam
    /// rect when the user disabled the cam channel.
    has_camera_frame: bool,
    /// Same for screen.
    has_screen_frame: bool,
}

impl RecordingCompose {
    /// Boot a fresh wisp `Application` + allocate the BGRA
    /// `RenderTexture` for the configured output resolution.
    ///
    /// # Errors
    ///
    /// Returns the underlying [`wisp::Error`] if the adapter / device
    /// can't be created. On macOS dev machines this should always
    /// succeed.
    pub fn new(
        width: u32,
        height: u32,
        screen_dims: StreamDimensions,
        cam_dims: StreamDimensions,
    ) -> Result<Self, wisp::Error> {
        let app = pollster::block_on(Application::new(AppConfig {
            width,
            height,
            ..AppConfig::default()
        }))?;
        let renderer = Renderer::new(&app, wisp::wgpu::TextureFormat::Bgra8UnormSrgb)?;
        let scene = RecordingScene::new(&app, screen_dims, cam_dims, CamLayout::default());
        let target = RenderTexture::with_format(
            &app,
            width,
            height,
            wisp::wgpu::TextureFormat::Bgra8UnormSrgb,
        );

        Ok(Self {
            app,
            renderer,
            scene,
            target,
            width,
            height,
            has_camera_frame: false,
            has_screen_frame: false,
        })
    }

    /// Pull the latest BGRA from each slot, upload to the scene,
    /// render to the target, read back via `RenderTexture::read_pixels`.
    ///
    /// Returns `None` when neither slot has ever been written
    /// to — caller should skip pushing anything to the encoder
    /// this tick.
    pub fn compose_frame(
        &mut self,
        camera_slot: &FrameSlot,
        screen_slot: &FrameSlot,
    ) -> Option<ComposedFrame> {
        let cam = camera_slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        let screen = screen_slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();

        if let Some(bytes) = cam {
            let expected = self.scene.cam_dims().byte_len();
            if bytes.len() == expected {
                self.scene.set_camera_frame(&self.app, &bytes);
                self.has_camera_frame = true;
            } else {
                tracing::trace!(
                    got = bytes.len(),
                    expected,
                    "compose: cam frame size mismatch, dropping"
                );
            }
        }
        if let Some(bytes) = screen {
            let expected = self.scene.screen_dims().byte_len();
            if bytes.len() == expected {
                self.scene.set_screen_frame(&self.app, &bytes);
                self.has_screen_frame = true;
            } else {
                tracing::trace!(
                    got = bytes.len(),
                    expected,
                    "compose: screen frame size mismatch, dropping"
                );
            }
        }

        if !self.has_camera_frame && !self.has_screen_frame {
            return None;
        }

        let _stats = self.renderer.render_stage(
            &self.app,
            self.target.view(),
            Color::BLACK,
            self.scene.stage(),
        );

        let bytes = self.target.read_pixels(&self.app);
        Some(ComposedFrame {
            bytes,
            width: self.width,
            height: self.height,
        })
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
    use std::sync::{Arc, Mutex};

    fn test_dims() -> (StreamDimensions, StreamDimensions) {
        (StreamDimensions::new(64, 64), StreamDimensions::new(32, 32))
    }

    #[test]
    fn new_allocates_compose_pipeline() {
        let (screen, cam) = test_dims();
        let compose = RecordingCompose::new(64, 64, screen, cam).expect("init");
        assert_eq!(compose.dimensions(), (64, 64));
    }

    #[test]
    fn compose_frame_returns_none_when_slots_empty() {
        let (screen, cam) = test_dims();
        let mut compose = RecordingCompose::new(64, 64, screen, cam).expect("init");
        let cam_slot: FrameSlot = Arc::new(Mutex::new(None));
        let screen_slot: FrameSlot = Arc::new(Mutex::new(None));
        assert!(compose.compose_frame(&cam_slot, &screen_slot).is_none());
    }

    #[test]
    fn compose_frame_returns_bgra_after_screen_upload() {
        let (screen, cam) = test_dims();
        let mut compose = RecordingCompose::new(64, 64, screen, cam).expect("init");
        let cam_slot: FrameSlot = Arc::new(Mutex::new(None));
        let screen_slot: FrameSlot = Arc::new(Mutex::new(Some(vec![128u8; 64 * 64 * 4])));
        let composed = compose
            .compose_frame(&cam_slot, &screen_slot)
            .expect("frame produced");
        assert_eq!(composed.width, 64);
        assert_eq!(composed.height, 64);
        // BGRA packed = width * height * 4
        assert_eq!(composed.bytes.len(), 64 * 64 * 4);
    }

    #[test]
    fn compose_frame_drops_size_mismatched_uploads() {
        let (screen, cam) = test_dims();
        let mut compose = RecordingCompose::new(64, 64, screen, cam).expect("init");
        let cam_slot: FrameSlot = Arc::new(Mutex::new(Some(vec![0u8; 99])));
        let screen_slot: FrameSlot = Arc::new(Mutex::new(None));
        assert!(compose.compose_frame(&cam_slot, &screen_slot).is_none());
    }

    #[test]
    fn compose_frame_clears_slots_on_each_call() {
        let (screen, cam) = test_dims();
        let mut compose = RecordingCompose::new(64, 64, screen, cam).expect("init");
        let cam_slot: FrameSlot = Arc::new(Mutex::new(None));
        let screen_slot: FrameSlot = Arc::new(Mutex::new(Some(vec![64u8; 64 * 64 * 4])));
        let _ = compose.compose_frame(&cam_slot, &screen_slot);
        assert!(cam_slot.lock().unwrap().is_none());
        assert!(screen_slot.lock().unwrap().is_none());
    }
}
