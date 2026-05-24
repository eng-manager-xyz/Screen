//! `RecordingScene` — composes the recorder's two visible streams
//! (full-screen capture + circular webcam bubble) into a single
//! `Stage` ready to render to a `wgpu::TextureView` (M-EXPORT.0 of
//! M-RECORD-EXPORT).
//!
//! Reusable composition primitive: the recorder feeds it new BGRA
//! frames from the per-channel capture pipelines; the future editor
//! preview lane will instantiate the same scene from decoded
//! recording frames. Keeping composition inside `wisp` (rather than
//! in `screen-app`) keeps the recorder + editor visually consistent
//! and lets the storybook drive snapshot tests without booting Tauri.
//!
//! ## Layout
//!
//! - **Screen frame** is the fullscreen background — a `Sprite`
//!   anchored centre, scaled to fill NDC `[-1, 1]²`.
//! - **Camera frame** is a smaller `Sprite` inside a `Container` whose
//!   `clip` is a [`MaskShape::Circle`]. Default position is the
//!   bottom-right corner with a small margin; the [`CamLayout`]
//!   struct exposes this so the future editor can re-anchor.
//!
//! ```admonish important title="Latest-frame-wins"
//! `set_screen_frame` / `set_camera_frame` overwrite the GPU
//! texture in place — no queueing. The encoder reads at its own
//! frame rate (30 fps in M-EXPORT.1's default) regardless of source
//! FPS; uploads from faster sources just write the most recent
//! frame, slower sources show the last upload until a new one
//! lands.
//! ```

use glam::Vec2;

use crate::application::Application;
use crate::color::Color;
use crate::render::{RenderStats, Renderer};
use crate::scene::Stage;
use crate::scene::clip::MaskShape;
use crate::scene::container::Container;
use crate::scene::node::NodeId;
use crate::scene::sprite::Sprite;
use crate::scene::transform::Transform;
use crate::texture::video_texture::VideoTexture;

/// Position + size of the circular webcam bubble within the composed
/// frame. All coordinates are NDC — `(-1, -1)` bottom-left,
/// `(1, 1)` top-right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CamLayout {
    /// Centre of the circle in NDC.
    pub center: Vec2,
    /// Radius in NDC units. The default `0.18` makes the bubble
    /// roughly 18% of the viewport's shorter side, matching the
    /// Screen Studio convention.
    pub radius: f32,
}

impl CamLayout {
    /// Bottom-right corner with a 24 px-equivalent NDC margin
    /// (~`0.05` in `[-1, 1]` space) and a radius of `0.18` (~18% of
    /// viewport).
    pub const BOTTOM_RIGHT: Self = Self {
        center: Vec2::new(0.74, -0.74),
        radius: 0.20,
    };

    /// Bottom-left variant — the current default. Mirrors the
    /// Screen Studio bottom-left cam placement convention; flip to
    /// [`Self::BOTTOM_RIGHT`] for right-handed presenters.
    pub const BOTTOM_LEFT: Self = Self {
        center: Vec2::new(-0.74, -0.74),
        radius: 0.20,
    };

    /// Top-left variant.
    pub const TOP_LEFT: Self = Self {
        center: Vec2::new(-0.74, 0.74),
        radius: 0.20,
    };
}

impl Default for CamLayout {
    fn default() -> Self {
        Self::BOTTOM_LEFT
    }
}

/// Dimensions for one of the composed input streams. Used at
/// [`RecordingScene::new`] time to allocate the backing
/// [`VideoTexture`]s; subsequent `set_*_frame` calls must pass a
/// BGRA buffer matching `width * height * 4`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamDimensions {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
}

impl StreamDimensions {
    /// Construct from width + height.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Expected `bgra` byte length for `set_*_frame`.
    #[must_use]
    pub fn byte_len(self) -> usize {
        (self.width as usize) * (self.height as usize) * 4
    }
}

/// Copy a packed BGRA buffer (`width * height * 4` bytes, no row
/// padding) into a freshly-allocated `Vec<u8>` with rows reversed so
/// the output is bottom-up.
///
/// wisp's `Sprite` vertex shader maps texture UV `(0, 0)` to the
/// bottom-left of the rendered NDC quad (the same "+y flip" the
/// sprite vs glyphon convention bullet in CLAUDE.md flags), so
/// `VideoTexture::upload_bgra` of a standard top-down BGRA image
/// renders upside-down. The `set_*_frame` methods on [`RecordingScene`]
/// run this helper so callers can pass top-down BGRA (the universal
/// `CoreVideo` / `GStreamer` / `Canvas2D` convention).
fn flip_bgra_rows_top_down_to_bottom_up(src: &[u8], width: u32, height: u32) -> Vec<u8> {
    // Caller (RecordingScene::set_*_frame) asserts src.len() == width * height * 4
    // before calling, so the slice indexing below stays in bounds.
    let row_bytes = (width as usize) * 4;
    let h = height as usize;
    let mut out = Vec::with_capacity(row_bytes * h);
    for row in (0..h).rev() {
        let start = row * row_bytes;
        out.extend_from_slice(&src[start..start + row_bytes]);
    }
    out
}

/// Two-stream composition: fullscreen screen-capture + circular
/// webcam bubble. Owns the `Stage` + the per-stream `VideoTexture`s.
///
/// The struct itself doesn't render — pass [`Self::stage`] to
/// [`Renderer::render_stage`] or call the [`Self::render`] convenience
/// wrapper.
pub struct RecordingScene {
    stage: Stage,
    screen_video: VideoTexture,
    cam_video: VideoTexture,
    screen_dims: StreamDimensions,
    cam_dims: StreamDimensions,
    cam_layout: CamLayout,
    screen_sprite: NodeId,
    cam_container: NodeId,
    cam_sprite: NodeId,
}

impl std::fmt::Debug for RecordingScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `Stage` doesn't derive `Debug` (would require Debug on
        // every Node variant); skip it here. The other fields are
        // enough for debugging the layout / dimensions.
        f.debug_struct("RecordingScene")
            .field("screen_dims", &self.screen_dims)
            .field("cam_dims", &self.cam_dims)
            .field("cam_layout", &self.cam_layout)
            .field("nodes_in_stage", &self.stage.len())
            .finish_non_exhaustive()
    }
}

impl RecordingScene {
    /// Build a fresh scene. Allocates two `VideoTexture`s at the
    /// requested dimensions (the host re-uploads BGRA bytes via
    /// [`Self::set_screen_frame`] / [`Self::set_camera_frame`] on
    /// every captured frame); inserts the corresponding sprites
    /// into the stage with the cam wrapped in a circular-clip
    /// container.
    ///
    /// # Panics
    ///
    /// Panics if either dimension pair is zero — the wgpu texture
    /// allocator rejects zero-size textures and we'd prefer the
    /// trap fire at construction.
    #[must_use]
    pub fn new(
        app: &Application,
        screen_dims: StreamDimensions,
        cam_dims: StreamDimensions,
        cam_layout: CamLayout,
    ) -> Self {
        assert!(
            screen_dims.width > 0 && screen_dims.height > 0,
            "RecordingScene: screen dims must be non-zero (got {}×{})",
            screen_dims.width,
            screen_dims.height,
        );
        assert!(
            cam_dims.width > 0 && cam_dims.height > 0,
            "RecordingScene: cam dims must be non-zero (got {}×{})",
            cam_dims.width,
            cam_dims.height,
        );

        let mut stage = Stage::new();
        let screen_video = VideoTexture::new(app, screen_dims.width, screen_dims.height);
        let cam_video = VideoTexture::new(app, cam_dims.width, cam_dims.height);

        // Fullscreen screen sprite: anchored centre, scaled to fill
        // the full NDC `[-1, 1]` viewport. Sprite local rect is
        // `[0, 1]²` so unit scale is 1×1 NDC; we want 2×2.
        let mut screen_sprite =
            Sprite::from_texture(screen_video.texture().clone()).with_anchor(Vec2::splat(0.5));
        screen_sprite.container.transform = Transform {
            scale: Vec2::splat(2.0),
            ..Transform::IDENTITY
        };
        let screen_sprite_id = stage
            .add_child(stage.root(), screen_sprite)
            .expect("Stage root is alive at scene construction");

        // Circular cam container. The container's clip is the NDC
        // circle; its transform positions the bubble; the child
        // sprite fills the container's local-space at unit scale.
        let cam_container = Container {
            clip: Some(MaskShape::circle(cam_layout.center, cam_layout.radius)),
            ..Container::default()
        };
        let cam_container_id = stage
            .add_child(stage.root(), cam_container)
            .expect("Stage root is alive at scene construction");

        // Cam sprite — anchored centre at the same NDC position as
        // the circle, scaled to a square bounding box matching the
        // circle's diameter. The clip on the parent container masks
        // the sprite into a circle at render-time.
        let mut cam_sprite =
            Sprite::from_texture(cam_video.texture().clone()).with_anchor(Vec2::splat(0.5));
        cam_sprite.container.transform = Transform {
            position: cam_layout.center,
            scale: Vec2::splat(cam_layout.radius * 2.0),
            ..Transform::IDENTITY
        };
        let cam_sprite_id = stage
            .add_child(cam_container_id, cam_sprite)
            .expect("cam container is alive immediately after add_child");

        Self {
            stage,
            screen_video,
            cam_video,
            screen_dims,
            cam_dims,
            cam_layout,
            screen_sprite: screen_sprite_id,
            cam_container: cam_container_id,
            cam_sprite: cam_sprite_id,
        }
    }

    /// Upload the latest screen-capture frame. `bgra` is **top-down**
    /// packed BGRA8 (the standard `CoreVideo` / `GStreamer`
    /// convention) and `bgra.len()` must equal
    /// `screen_dims().byte_len()`. The method flips rows internally
    /// to match wisp's sprite convention before uploading — see
    /// [`flip_bgra_rows_top_down_to_bottom_up`].
    ///
    /// # Panics
    ///
    /// Panics with `"VideoTexture::upload_bgra: byte length mismatch"`
    /// if `bgra.len()` doesn't match the configured screen dimensions.
    pub fn set_screen_frame(&mut self, app: &Application, bgra: &[u8]) {
        assert_eq!(
            bgra.len(),
            self.screen_dims.byte_len(),
            "VideoTexture::upload_bgra: byte length mismatch"
        );
        let flipped = flip_bgra_rows_top_down_to_bottom_up(
            bgra,
            self.screen_dims.width,
            self.screen_dims.height,
        );
        self.screen_video.upload_bgra(app, &flipped);
    }

    /// Upload the latest camera frame. `bgra` is **top-down** packed
    /// BGRA8 (same convention as [`Self::set_screen_frame`]) and
    /// `bgra.len()` must equal `cam_dims().byte_len()`.
    ///
    /// # Panics
    ///
    /// Panics with `"VideoTexture::upload_bgra: byte length mismatch"`
    /// if `bgra.len()` doesn't match the configured cam dimensions.
    pub fn set_camera_frame(&mut self, app: &Application, bgra: &[u8]) {
        assert_eq!(
            bgra.len(),
            self.cam_dims.byte_len(),
            "VideoTexture::upload_bgra: byte length mismatch"
        );
        let flipped =
            flip_bgra_rows_top_down_to_bottom_up(bgra, self.cam_dims.width, self.cam_dims.height);
        self.cam_video.upload_bgra(app, &flipped);
    }

    /// Render the composed scene into `view`. Thin wrapper around
    /// [`Renderer::render_stage`] — callers that need the
    /// [`RenderStats`] can use that directly via [`Self::stage`].
    pub fn render(
        &self,
        renderer: &Renderer,
        app: &Application,
        view: &wgpu::TextureView,
        clear: Color,
    ) -> RenderStats {
        renderer.render_stage(app, view, clear, &self.stage)
    }

    /// Borrow the scene graph (e.g. to call
    /// [`Renderer::render_stage`] directly).
    #[must_use]
    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    /// Mutable borrow of the underlying stage — exposed so future
    /// extensions (annotations, cursor overlays, watermarks) can
    /// attach extra nodes without monkey-patching this type.
    #[must_use]
    pub fn stage_mut(&mut self) -> &mut Stage {
        &mut self.stage
    }

    /// Configured screen dimensions.
    #[must_use]
    pub fn screen_dims(&self) -> StreamDimensions {
        self.screen_dims
    }

    /// Configured camera dimensions.
    #[must_use]
    pub fn cam_dims(&self) -> StreamDimensions {
        self.cam_dims
    }

    /// Active camera layout (centre + radius).
    #[must_use]
    pub fn cam_layout(&self) -> CamLayout {
        self.cam_layout
    }

    /// `NodeId` of the fullscreen screen sprite. Exposed so callers
    /// (storybook stories, future editor) can tweak the sprite's
    /// container without rebuilding the scene.
    #[must_use]
    pub fn screen_sprite_id(&self) -> NodeId {
        self.screen_sprite
    }

    /// `NodeId` of the cam container (the one holding the clip).
    /// Modifying this container's transform moves the bubble.
    #[must_use]
    pub fn cam_container_id(&self) -> NodeId {
        self.cam_container
    }

    /// `NodeId` of the cam sprite (inside the cam container).
    #[must_use]
    pub fn cam_sprite_id(&self) -> NodeId {
        self.cam_sprite
    }

    /// Toggle the camera bubble's visibility. Hiding the cam container
    /// prevents the cam sprite from rendering at all — useful when the
    /// recording session has the camera channel disabled (no upload
    /// happens, so `VideoTexture` would sample whatever wgpu's
    /// `create_texture` left in memory; not guaranteed to be zero).
    pub fn set_camera_visible(&mut self, visible: bool) {
        if let Some(node) = self.stage.get_mut(self.cam_container) {
            node.container_mut().visible = visible;
        }
    }

    /// Toggle the screen sprite's visibility. Same rationale as
    /// [`Self::set_camera_visible`] for the screen channel.
    pub fn set_screen_visible(&mut self, visible: bool) {
        if let Some(node) = self.stage.get_mut(self.screen_sprite) {
            node.container_mut().visible = visible;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppConfig;

    fn boot() -> Application {
        pollster::block_on(Application::new(AppConfig::default())).expect("init")
    }

    #[test]
    fn cam_layout_default_is_bottom_left() {
        assert_eq!(CamLayout::default(), CamLayout::BOTTOM_LEFT);
    }

    #[test]
    fn cam_layout_constants_are_in_ndc_range() {
        for layout in [
            CamLayout::BOTTOM_RIGHT,
            CamLayout::BOTTOM_LEFT,
            CamLayout::TOP_LEFT,
        ] {
            assert!(layout.center.x.abs() <= 1.0);
            assert!(layout.center.y.abs() <= 1.0);
            assert!(layout.radius > 0.0 && layout.radius <= 1.0);
        }
    }

    #[test]
    fn stream_dims_byte_len_matches_w_h_4() {
        assert_eq!(StreamDimensions::new(64, 32).byte_len(), 64 * 32 * 4);
        assert_eq!(
            StreamDimensions::new(1920, 1080).byte_len(),
            1920 * 1080 * 4
        );
    }

    #[test]
    fn new_inserts_three_nodes_under_root() {
        let app = boot();
        let scene = RecordingScene::new(
            &app,
            StreamDimensions::new(128, 72),
            StreamDimensions::new(64, 64),
            CamLayout::default(),
        );
        // Root + screen sprite + cam container + cam sprite = 4
        // total. (Stage::len includes the root.)
        assert_eq!(scene.stage().len(), 4);
        assert_eq!(scene.screen_dims().width, 128);
        assert_eq!(scene.cam_dims().height, 64);
    }

    #[test]
    fn cam_container_carries_circle_clip() {
        let app = boot();
        let scene = RecordingScene::new(
            &app,
            StreamDimensions::new(64, 64),
            StreamDimensions::new(32, 32),
            CamLayout::BOTTOM_RIGHT,
        );
        let cam_node = scene
            .stage()
            .get(scene.cam_container_id())
            .expect("cam container alive");
        let container: &Container = cam_node.container();
        let clip = container.clip.expect("cam container has clip");
        match clip {
            MaskShape::Circle { center, radius } => {
                assert_eq!(center, CamLayout::BOTTOM_RIGHT.center);
                // CamLayout constants are exact bit-for-bit equal
                // to the consts they were copied from — direct
                // comparison is safe, but clippy can't see that.
                assert!((radius - CamLayout::BOTTOM_RIGHT.radius).abs() < 1e-6);
            }
            other => panic!("expected Circle, got {other:?}"),
        }
    }

    #[test]
    fn set_screen_frame_round_trips_correct_byte_count() {
        let app = boot();
        let mut scene = RecordingScene::new(
            &app,
            StreamDimensions::new(32, 32),
            StreamDimensions::new(16, 16),
            CamLayout::default(),
        );
        let buf = vec![128u8; 32 * 32 * 4];
        scene.set_screen_frame(&app, &buf);
        // No panic = success; VideoTexture asserts the count.
    }

    #[test]
    fn set_camera_frame_round_trips_correct_byte_count() {
        let app = boot();
        let mut scene = RecordingScene::new(
            &app,
            StreamDimensions::new(32, 32),
            StreamDimensions::new(16, 16),
            CamLayout::default(),
        );
        let buf = vec![64u8; 16 * 16 * 4];
        scene.set_camera_frame(&app, &buf);
    }

    #[test]
    #[should_panic(expected = "byte length mismatch")]
    fn set_screen_frame_panics_on_wrong_byte_count() {
        let app = boot();
        let mut scene = RecordingScene::new(
            &app,
            StreamDimensions::new(32, 32),
            StreamDimensions::new(16, 16),
            CamLayout::default(),
        );
        scene.set_screen_frame(&app, &[0u8; 100]);
    }

    #[test]
    #[should_panic(expected = "screen dims must be non-zero")]
    fn new_panics_on_zero_screen_dim() {
        let app = boot();
        let _ = RecordingScene::new(
            &app,
            StreamDimensions::new(0, 32),
            StreamDimensions::new(16, 16),
            CamLayout::default(),
        );
    }

    #[test]
    #[should_panic(expected = "cam dims must be non-zero")]
    fn new_panics_on_zero_cam_dim() {
        let app = boot();
        let _ = RecordingScene::new(
            &app,
            StreamDimensions::new(64, 64),
            StreamDimensions::new(0, 16),
            CamLayout::default(),
        );
    }

    #[test]
    fn flip_helper_reverses_row_order() {
        // 4 pixels × 2 rows = 8 bytes/row × 2 = 16 bytes.
        // Wait — BGRA = 4 bytes/pixel, so 4 × 4 = 16 bytes/row × 2 = 32 total.
        let row_0 = [0xAAu8; 16];
        let row_1 = [0xCCu8; 16];
        let mut src = Vec::with_capacity(32);
        src.extend_from_slice(&row_0);
        src.extend_from_slice(&row_1);
        let out = flip_bgra_rows_top_down_to_bottom_up(&src, 4, 2);
        assert_eq!(out.len(), 32);
        assert_eq!(
            &out[0..16],
            &row_1,
            "row 1 (originally bottom) lands first in bottom-up output"
        );
        assert_eq!(&out[16..32], &row_0, "row 0 (originally top) lands last");
    }

    #[test]
    fn flip_helper_is_identity_for_single_row() {
        let row = [0x42u8; 16];
        let out = flip_bgra_rows_top_down_to_bottom_up(&row, 4, 1);
        assert_eq!(out, row);
    }

    #[test]
    fn set_camera_visible_toggles_cam_container_visibility() {
        let app = boot();
        let mut scene = RecordingScene::new(
            &app,
            StreamDimensions::new(32, 32),
            StreamDimensions::new(16, 16),
            CamLayout::default(),
        );
        // Default: visible.
        let cam_id = scene.cam_container_id();
        assert!(scene.stage().get(cam_id).unwrap().container().visible);

        scene.set_camera_visible(false);
        assert!(!scene.stage().get(cam_id).unwrap().container().visible);

        scene.set_camera_visible(true);
        assert!(scene.stage().get(cam_id).unwrap().container().visible);
    }

    #[test]
    fn set_screen_visible_toggles_screen_sprite_visibility() {
        let app = boot();
        let mut scene = RecordingScene::new(
            &app,
            StreamDimensions::new(32, 32),
            StreamDimensions::new(16, 16),
            CamLayout::default(),
        );
        let screen_id = scene.screen_sprite_id();
        assert!(scene.stage().get(screen_id).unwrap().container().visible);
        scene.set_screen_visible(false);
        assert!(!scene.stage().get(screen_id).unwrap().container().visible);
    }

    #[test]
    fn stage_mut_allows_extension() {
        let app = boot();
        let mut scene = RecordingScene::new(
            &app,
            StreamDimensions::new(32, 32),
            StreamDimensions::new(16, 16),
            CamLayout::default(),
        );
        let before = scene.stage().len();
        let root = scene.stage().root();
        let extra = scene
            .stage_mut()
            .add_child(root, Container::default())
            .expect("add_child");
        assert_eq!(scene.stage().len(), before + 1);
        // Use the returned NodeId so it's not dead-code.
        let _ = extra;
    }
}
