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
//!   `clip` is a [`MaskShape::Ellipse`] with half-extents
//!   aspect-compensated for the output canvas, so the bubble is a true
//!   circle in *pixels* (an NDC `Circle` would stretch into an ellipse
//!   on a non-square frame). Default position is the bottom-left
//!   corner with a small margin; the [`CamLayout`] struct exposes this
//!   so the future editor can re-anchor.
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
use crate::math::Rect;
use crate::render::{RenderStats, Renderer};
use crate::scene::Stage;
use crate::scene::clip::MaskShape;
use crate::scene::container::Container;
use crate::scene::graphics::{Fill, Graphics, Stroke};
use crate::scene::node::{Node, NodeId};
use crate::scene::sprite::Sprite;
use crate::scene::transform::Transform;
use crate::texture::Texture;
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

/// One click ripple to draw under the cursor (ED.19): an expanding,
/// fading ring centered at a click, in output NDC. The app derives these
/// from the click log (`edit::telemetry::ripples_at`) — `center` is already
/// mapped through the screen transform, `radius` grows with age, `alpha`
/// fades `1 → 0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorRipple {
    /// Ring center in NDC.
    pub center: Vec2,
    /// Ring radius in NDC units.
    pub radius: f32,
    /// Ring opacity, `0.0..=1.0`.
    pub alpha: f32,
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
    /// Editor-only backdrop node (a full-NDC `Graphics` rect drawn behind
    /// the screen). Lazily created on the first `set_background_*` call;
    /// the recorder never touches it, so its scene stays a plain
    /// three-node screen + cam stage.
    backdrop: Option<NodeId>,
    /// Editor-only cursor-overlay node (a `Graphics` pointer + click ripples,
    /// drawn over the framed screen). Lazily created on the first
    /// [`Self::set_cursor`] call; the recorder never touches it (its live
    /// capture already has a cursor baked into the screen frame).
    cursor: Option<NodeId>,
    /// Editor-only drop-shadow node (ED.18): a dark, offset rounded-rect drawn
    /// *behind* the framed screen (Phase 1, like the backdrop), so the offset
    /// sliver reads as a shadow. Lazily created by [`Self::set_frame_shadow`].
    shadow: Option<NodeId>,
    /// Editor-only inset-border node (ED.18): a rounded-rect *stroke* tracing
    /// the frame window, drawn *over* the screen (Phase 2, like the cursor).
    /// Lazily created by [`Self::set_frame_border`].
    border: Option<NodeId>,
    /// Editor-only wallpaper backdrop (ED.18): a full-NDC `Sprite` of a decoded
    /// image, drawn behind everything. A `Sprite` (not `Graphics`) so the
    /// renderer's batch order paints it before the gradient/shadow graphics —
    /// the backmost layer. Mutually exclusive with the gradient/color backdrop
    /// (the app hides one when the other is set). Created by
    /// [`Self::set_background_wallpaper`].
    wallpaper: Option<NodeId>,
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

        // Aspect-compensated bubble geometry. `cam_layout.radius` is in
        // NDC, where `[-1, 1]` spans the FULL output canvas on each
        // axis — so a circle with equal NDC x/y radius renders as an
        // ELLIPSE on a non-square canvas (the recorded-output stretch
        // bug). Convert the radius into per-axis NDC half-extents that
        // are equal in PIXELS: a target pixel radius of
        // `radius * min(w, h) / 2` (≈ `radius` of the shorter side, per
        // the `CamLayout` doc) maps back to `radius * min(w,h)/w` in x
        // and `radius * min(w,h)/h` in y. The output canvas size is
        // `screen_dims` (the screen fills the frame 1:1).
        // Dimensions fit u16 for any real display; `f32::from(u16)` is
        // lossless and sidesteps the `u32 as f32` precision-loss lint
        // (same conversion pattern as `render::mask_texture`).
        let dim_to_f32 = |d: u32| f32::from(u16::try_from(d.min(u32::from(u16::MAX))).unwrap_or(1));
        let canvas_width = dim_to_f32(screen_dims.width);
        let canvas_height = dim_to_f32(screen_dims.height);
        let min_side = canvas_width.min(canvas_height);
        let cam_half_extents = Vec2::new(
            cam_layout.radius * min_side / canvas_width,
            cam_layout.radius * min_side / canvas_height,
        );

        // Cam container — the clip is a true pixel circle (an NDC
        // ellipse with the compensated half-extents); its child sprite
        // fills the same square pixel region so the feed is undistorted.
        let cam_container = Container {
            clip: Some(MaskShape::ellipse(cam_layout.center, cam_half_extents)),
            ..Container::default()
        };
        let cam_container_id = stage
            .add_child(stage.root(), cam_container)
            .expect("Stage root is alive at scene construction");

        // Cam sprite — anchored centre at the bubble position, scaled
        // so its square texture covers a SQUARE PIXEL region (diameter
        // = the bubble's), i.e. NDC scale = `2 * half_extents`. Using
        // `splat(radius*2)` here would re-introduce the stretch. The
        // parent container's elliptical clip masks it to a circle.
        let mut cam_sprite =
            Sprite::from_texture(cam_video.texture().clone()).with_anchor(Vec2::splat(0.5));
        cam_sprite.container.transform = Transform {
            position: cam_layout.center,
            scale: cam_half_extents * 2.0,
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
            backdrop: None,
            cursor: None,
            shadow: None,
            border: None,
            wallpaper: None,
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

    /// Set the screen sprite's local transform. The recorder leaves this at
    /// the base fill-NDC transform (`position 0`, `scale 2`); the editor
    /// (ED.16/ED.15) writes a per-frame crop-then-zoom transform here so the
    /// composed frame punches in / reframes. The screen sprite is anchored
    /// centre, so a `scale` of `2 * z` magnifies the source by `z` about the
    /// frame centre, and `position` shifts the focal point in NDC.
    pub fn set_screen_transform(&mut self, transform: Transform) {
        if let Some(node) = self.stage.get_mut(self.screen_sprite) {
            node.container_mut().transform = transform;
        }
    }

    /// Set (or clear) the screen sprite's clip — the editor's
    /// rounded-corner *frame window* (ED.18). The shape is in fixed output
    /// NDC (the clip is screen-space, not transform-aware), so the window
    /// stays put while the screen transform punches in inside it. Setting a
    /// clip makes the screen a dispatched node that composites *over* the
    /// backdrop. The recorder leaves this `None` (full-bleed, no clip).
    pub fn set_screen_clip(&mut self, shape: Option<MaskShape>) {
        if let Some(node) = self.stage.get_mut(self.screen_sprite) {
            node.container_mut().clip = shape;
        }
    }

    /// Lazily insert (or fetch) the backdrop `Graphics` node. It is added as
    /// a child of root with no clip, so it renders in the advanced-dispatch
    /// Phase 1 (behind the clipped, dispatched screen sprite). The recorder
    /// never calls a `set_background_*` method, so this node is never created
    /// for a recording scene.
    fn backdrop_graphics_mut(&mut self) -> &mut Graphics {
        if self.backdrop.is_none() {
            let root = self.stage.root();
            let id = self
                .stage
                .add_child(root, Graphics::new())
                .expect("Stage root is alive");
            self.backdrop = Some(id);
        }
        let id = self.backdrop.expect("backdrop node just ensured");
        match self.stage.get_mut(id) {
            Some(Node::Graphics(g)) => g,
            _ => unreachable!("backdrop node is always a Graphics"),
        }
    }

    /// Paint the backdrop as a linear gradient between two colors at
    /// `angle_deg` (0 = left→right, 90 = bottom→top), and make it visible.
    /// Editor-only (the framed-screen look behind the rounded canvas).
    pub fn set_background_gradient(&mut self, from: Color, to: Color, angle_deg: f32) {
        let theta = angle_deg.to_radians();
        // Endpoints in primitive-local coords ([-1, 1] for a full-NDC rect).
        let dir = Vec2::new(theta.cos(), theta.sin());
        let g = self.backdrop_graphics_mut();
        g.primitives.clear();
        g.fill(Fill::LinearGradient {
            start: -dir,
            end: dir,
            color_a: from,
            color_b: to,
        });
        g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
        g.container.visible = true;
    }

    /// Paint the backdrop as a flat color fill, and make it visible.
    pub fn set_background_color(&mut self, color: Color) {
        let g = self.backdrop_graphics_mut();
        g.primitives.clear();
        g.fill(Fill::Solid(color));
        g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
        g.container.visible = true;
    }

    /// Toggle the backdrop's visibility. No-op if no backdrop has been set
    /// (the recorder path).
    pub fn set_background_visible(&mut self, visible: bool) {
        if let Some(id) = self.backdrop
            && let Some(node) = self.stage.get_mut(id)
        {
            node.container_mut().visible = visible;
        }
    }

    /// Lazily insert (or fetch) the cursor-overlay `Graphics` node. It carries
    /// a full-NDC clip so it renders in the advanced-dispatch Phase 2 (over
    /// the framed, also-dispatched screen sprite), and is added after the
    /// screen so it composites on top. Editor-only — never created for a
    /// recording scene.
    fn cursor_graphics_mut(&mut self) -> &mut Graphics {
        if self.cursor.is_none() {
            let root = self.stage.root();
            let mut g = Graphics::new();
            g.container.clip = Some(MaskShape::rect(Rect::new(-1.0, -1.0, 2.0, 2.0)));
            let id = self.stage.add_child(root, g).expect("Stage root is alive");
            self.cursor = Some(id);
        }
        let id = self.cursor.expect("cursor node just ensured");
        match self.stage.get_mut(id) {
            Some(Node::Graphics(g)) => g,
            _ => unreachable!("cursor node is always a Graphics"),
        }
    }

    /// Draw the cursor overlay (ED.19): expanding click ripples under a
    /// dark-outlined white pointer at `pointer_ndc`, with the pointer sized by
    /// `half` (NDC half-extent). All coordinates are output NDC — the app maps
    /// the captured normalized position through the screen transform so the
    /// pointer stays glued to its pixel through zoom/crop/padding. Makes the
    /// overlay visible. Editor-only.
    pub fn set_cursor(&mut self, pointer_ndc: Vec2, half: f32, ripples: &[CursorRipple]) {
        let g = self.cursor_graphics_mut();
        g.primitives.clear();
        g.stroke(None);
        // Ripples first (under the pointer): expanding, fading discs.
        for r in ripples {
            g.fill(Fill::Solid(Color::rgba(1.0, 1.0, 1.0, r.alpha)));
            g.draw_ellipse(r.center, Vec2::splat(r.radius));
        }
        // Pointer triangle (CCW), scaled by `half` about its hot-spot tip. A
        // dark, slightly-larger triangle behind a white one reads on any
        // backdrop.
        let arrow = |s: f32| {
            let scale = half * s;
            [
                pointer_ndc + Vec2::new(0.0, 0.0) * scale,
                pointer_ndc + Vec2::new(0.0, -1.7) * scale,
                pointer_ndc + Vec2::new(1.15, -1.05) * scale,
            ]
        };
        g.fill(Fill::Solid(Color::rgb_u8(20, 20, 20)));
        g.draw_polygon(&arrow(1.3));
        g.fill(Fill::Solid(Color::WHITE));
        g.draw_polygon(&arrow(1.0));
        g.container.visible = true;
    }

    /// Toggle the cursor overlay's visibility. No-op if no cursor has been set
    /// (the recorder path).
    pub fn set_cursor_visible(&mut self, visible: bool) {
        if let Some(id) = self.cursor
            && let Some(node) = self.stage.get_mut(id)
        {
            node.container_mut().visible = visible;
        }
    }

    /// Lazily insert (or fetch) the drop-shadow `Graphics` node — an
    /// *unclipped* root child, so it renders in advanced-dispatch Phase 1
    /// (behind the dispatched, clipped screen sprite). Added after the backdrop
    /// so the shadow composites over the backdrop but under the screen.
    fn shadow_graphics_mut(&mut self) -> &mut Graphics {
        if self.shadow.is_none() {
            let root = self.stage.root();
            let id = self
                .stage
                .add_child(root, Graphics::new())
                .expect("Stage root is alive");
            self.shadow = Some(id);
        }
        let id = self.shadow.expect("shadow node just ensured");
        match self.stage.get_mut(id) {
            Some(Node::Graphics(g)) => g,
            _ => unreachable!("shadow node is always a Graphics"),
        }
    }

    /// Draw the framed-screen drop shadow (ED.18): a `color` rounded-rect the
    /// shape of the frame `window`, offset by `offset` (NDC), drawn behind the
    /// screen so only the offset sliver shows. Editor-only — the recorder never
    /// calls this, so its scene has no shadow node.
    pub fn set_frame_shadow(
        &mut self,
        window: Rect,
        corner_radius: f32,
        offset: Vec2,
        color: Color,
    ) {
        let g = self.shadow_graphics_mut();
        g.primitives.clear();
        g.stroke(None);
        g.fill(Fill::Solid(color));
        let shifted = Rect::new(
            window.min.x + offset.x,
            window.min.y + offset.y,
            window.size.x,
            window.size.y,
        );
        g.draw_rounded_rect(shifted, corner_radius);
        g.container.visible = true;
    }

    /// Toggle the drop-shadow's visibility. No-op if unset (recorder path).
    pub fn set_frame_shadow_visible(&mut self, visible: bool) {
        if let Some(id) = self.shadow
            && let Some(node) = self.stage.get_mut(id)
        {
            node.container_mut().visible = visible;
        }
    }

    /// Lazily insert (or fetch) the inset-border `Graphics` node — carries a
    /// full-NDC clip so it dispatches in Phase 2 (over the framed screen),
    /// mirroring the cursor overlay.
    fn border_graphics_mut(&mut self) -> &mut Graphics {
        if self.border.is_none() {
            let root = self.stage.root();
            let mut g = Graphics::new();
            g.container.clip = Some(MaskShape::rect(Rect::new(-1.0, -1.0, 2.0, 2.0)));
            let id = self.stage.add_child(root, g).expect("Stage root is alive");
            self.border = Some(id);
        }
        let id = self.border.expect("border node just ensured");
        match self.stage.get_mut(id) {
            Some(Node::Graphics(g)) => g,
            _ => unreachable!("border node is always a Graphics"),
        }
    }

    /// Draw the inset border (ED.18): a `stroke`d rounded-rect tracing the
    /// frame `window`, over the screen. Transparent fill so only the stroke
    /// shows. Editor-only.
    pub fn set_frame_border(&mut self, window: Rect, corner_radius: f32, stroke: Stroke) {
        let g = self.border_graphics_mut();
        g.primitives.clear();
        g.fill(Fill::Solid(Color::rgba(0.0, 0.0, 0.0, 0.0)));
        g.stroke(Some(stroke));
        g.draw_rounded_rect(window, corner_radius);
        g.container.visible = true;
    }

    /// Toggle the inset border's visibility. No-op if unset (recorder path).
    pub fn set_frame_border_visible(&mut self, visible: bool) {
        if let Some(id) = self.border
            && let Some(node) = self.stage.get_mut(id)
        {
            node.container_mut().visible = visible;
        }
    }

    /// Set the wallpaper backdrop (ED.18) from decoded RGBA8 (`width*height*4`
    /// bytes — the app decodes the image; wisp must not gain an asset path).
    /// A full-NDC `Sprite`, the backmost layer. The caller is responsible for
    /// hiding the gradient/color backdrop (they're mutually exclusive — a
    /// `Sprite` and a `Graphics` backdrop fight the batch order). Rebuilds the
    /// node on each call (wallpaper changes are user-driven + rare).
    ///
    /// # Panics
    ///
    /// Panics if `rgba.len() != width * height * 4` (via `Texture::from_rgba`).
    pub fn set_background_wallpaper(
        &mut self,
        app: &Application,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) {
        let tex = Texture::from_rgba(app, width, height, rgba);
        let mut sprite = Sprite::from_texture(tex).with_anchor(Vec2::splat(0.5));
        sprite.container.transform = Transform {
            scale: Vec2::splat(2.0),
            ..Transform::IDENTITY
        };
        // A Sprite can't swap its texture in place; rebuild the node.
        if let Some(old) = self.wallpaper.take() {
            self.stage.destroy(old);
        }
        let root = self.stage.root();
        let id = self
            .stage
            .add_child(root, sprite)
            .expect("Stage root is alive");
        self.wallpaper = Some(id);
    }

    /// Toggle the wallpaper backdrop's visibility. No-op if unset (recorder
    /// path); also the app's lever for the wallpaper↔gradient mutual exclusion.
    pub fn set_background_wallpaper_visible(&mut self, visible: bool) {
        if let Some(id) = self.wallpaper
            && let Some(node) = self.stage.get_mut(id)
        {
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
        // Recorder isolation (ED.18): a fresh scene has no backdrop node
        // and the screen sprite is un-clipped (full-bleed). The editor's
        // `set_background_*` / `set_screen_clip` are the only mutators, and
        // the recorder never calls them — so a recording scene is
        // bit-identical to pre-ED.18.
        assert!(scene.backdrop.is_none(), "no backdrop until set_background");
        assert!(scene.cursor.is_none(), "no cursor overlay until set_cursor");
        assert!(
            scene.shadow.is_none(),
            "no drop shadow until set_frame_shadow"
        );
        assert!(
            scene.border.is_none(),
            "no inset border until set_frame_border"
        );
        assert!(
            scene.wallpaper.is_none(),
            "no wallpaper until set_background_wallpaper"
        );
        assert!(
            scene
                .stage()
                .get(scene.screen_sprite_id())
                .unwrap()
                .container()
                .clip
                .is_none(),
            "screen sprite is un-clipped in the recorder path"
        );
    }

    #[test]
    fn set_cursor_adds_one_overlay_node_and_toggles() {
        let app = boot();
        let mut scene = RecordingScene::new(
            &app,
            StreamDimensions::new(64, 64),
            StreamDimensions::new(32, 32),
            CamLayout::default(),
        );
        let before = scene.stage().len();
        let ripples = [CursorRipple {
            center: Vec2::new(0.2, -0.3),
            radius: 0.1,
            alpha: 0.5,
        }];
        scene.set_cursor(Vec2::new(0.1, 0.2), 0.08, &ripples);
        // One new node (the cursor Graphics).
        assert_eq!(scene.stage().len(), before + 1);
        let id = scene.cursor.expect("cursor created");
        assert!(scene.stage().get(id).unwrap().container().visible);
        // The node carries a full-NDC clip so it dispatches over the screen.
        assert!(
            scene.stage().get(id).unwrap().container().clip.is_some(),
            "cursor node dispatches (clip set) so it composites on top"
        );
        // A second call reuses the node — no leak.
        scene.set_cursor(Vec2::new(-0.4, 0.5), 0.06, &[]);
        assert_eq!(scene.stage().len(), before + 1, "cursor node reused");
        // Toggle off.
        scene.set_cursor_visible(false);
        assert!(!scene.stage().get(id).unwrap().container().visible);
    }

    #[test]
    fn set_frame_shadow_and_border_add_one_node_each_with_right_dispatch() {
        let app = boot();
        let mut scene = RecordingScene::new(
            &app,
            StreamDimensions::new(64, 64),
            StreamDimensions::new(32, 32),
            CamLayout::default(),
        );
        let before = scene.stage().len();
        let window = Rect::new(-0.8, -0.8, 1.6, 1.6);

        // Shadow: an UNCLIPPED node (Phase 1, behind the screen).
        scene.set_frame_shadow(
            window,
            0.05,
            Vec2::new(0.02, -0.02),
            Color::rgba(0.0, 0.0, 0.0, 0.4),
        );
        assert_eq!(scene.stage().len(), before + 1);
        let sid = scene.shadow.expect("shadow created");
        assert!(scene.stage().get(sid).unwrap().container().visible);
        assert!(
            scene.stage().get(sid).unwrap().container().clip.is_none(),
            "shadow is unclipped (Phase 1, behind the screen)"
        );

        // Border: a full-NDC-clipped node (Phase 2, over the screen).
        scene.set_frame_border(window, 0.05, Stroke::new(0.01, Color::WHITE));
        assert_eq!(scene.stage().len(), before + 2);
        let bid = scene.border.expect("border created");
        assert!(
            scene.stage().get(bid).unwrap().container().clip.is_some(),
            "border dispatches (clip set) so it composites over the screen"
        );

        // Reuse + toggle.
        scene.set_frame_shadow(window, 0.05, Vec2::ZERO, Color::BLACK);
        scene.set_frame_border(window, 0.05, Stroke::new(0.02, Color::BLACK));
        assert_eq!(scene.stage().len(), before + 2, "nodes reused, no leak");
        scene.set_frame_shadow_visible(false);
        scene.set_frame_border_visible(false);
        assert!(!scene.stage().get(sid).unwrap().container().visible);
        assert!(!scene.stage().get(bid).unwrap().container().visible);
    }

    #[test]
    fn set_background_wallpaper_adds_one_sprite_and_rebuilds() {
        let app = boot();
        let mut scene = RecordingScene::new(
            &app,
            StreamDimensions::new(64, 64),
            StreamDimensions::new(32, 32),
            CamLayout::default(),
        );
        let before = scene.stage().len();
        let rgba = vec![120u8; 16 * 16 * 4];
        scene.set_background_wallpaper(&app, 16, 16, &rgba);
        assert_eq!(scene.stage().len(), before + 1, "one wallpaper sprite");
        let first = scene.wallpaper.expect("wallpaper created");
        assert!(scene.stage().get(first).unwrap().container().visible);
        // A second call rebuilds the node (texture can't swap in place) — still
        // exactly one wallpaper node, no leak.
        scene.set_background_wallpaper(&app, 8, 8, &vec![200u8; 8 * 8 * 4]);
        assert_eq!(scene.stage().len(), before + 1, "rebuilt, no leak");
        scene.set_background_wallpaper_visible(false);
        let id = scene.wallpaper.expect("wallpaper present");
        assert!(!scene.stage().get(id).unwrap().container().visible);
    }

    #[test]
    fn set_background_gradient_adds_one_visible_backdrop_node() {
        let app = boot();
        let mut scene = RecordingScene::new(
            &app,
            StreamDimensions::new(64, 64),
            StreamDimensions::new(32, 32),
            CamLayout::default(),
        );
        let before = scene.stage().len();
        scene.set_background_gradient(
            Color::rgb_u8(255, 138, 128),
            Color::rgb_u8(40, 53, 147),
            135.0,
        );
        // Exactly one new node (the backdrop Graphics).
        assert_eq!(scene.stage().len(), before + 1);
        let id = scene.backdrop.expect("backdrop created");
        assert!(scene.stage().get(id).unwrap().container().visible);
        // A second call reuses the node — no leak.
        scene.set_background_color(Color::rgb_u8(10, 20, 30));
        assert_eq!(scene.stage().len(), before + 1, "backdrop node is reused");
        // Toggle visibility off.
        scene.set_background_visible(false);
        assert!(!scene.stage().get(id).unwrap().container().visible);
    }

    #[test]
    fn set_screen_clip_sets_and_clears_the_screen_clip() {
        let app = boot();
        let mut scene = RecordingScene::new(
            &app,
            StreamDimensions::new(64, 64),
            StreamDimensions::new(32, 32),
            CamLayout::default(),
        );
        let window = Rect::new(-0.9, -0.9, 1.8, 1.8);
        scene.set_screen_clip(Some(MaskShape::rounded_rect(window, 0.05)));
        let clip = scene
            .stage()
            .get(scene.screen_sprite_id())
            .unwrap()
            .container()
            .clip
            .expect("clip set");
        assert!(matches!(clip, MaskShape::RoundedRect { .. }));
        scene.set_screen_clip(None);
        assert!(
            scene
                .stage()
                .get(scene.screen_sprite_id())
                .unwrap()
                .container()
                .clip
                .is_none(),
            "clip cleared"
        );
    }

    #[test]
    fn set_screen_transform_updates_the_screen_sprite() {
        let app = boot();
        let mut scene = RecordingScene::new(
            &app,
            StreamDimensions::new(64, 64),
            StreamDimensions::new(32, 32),
            CamLayout::default(),
        );
        // A zoom-style transform (4× = 2× the base fill scale, shifted).
        let t = Transform {
            scale: Vec2::splat(4.0),
            position: Vec2::new(0.5, -0.25),
            ..Transform::IDENTITY
        };
        scene.set_screen_transform(t);
        let node = scene
            .stage()
            .get(scene.screen_sprite_id())
            .expect("screen sprite node");
        assert_eq!(node.container().transform, t);
    }

    #[test]
    fn cam_container_clip_is_circle_on_square_canvas() {
        let app = boot();
        // Square canvas → no aspect compensation → equal half-extents
        // (a true circle, expressed as an Ellipse with hx == hy).
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
            MaskShape::Ellipse {
                center,
                half_extents,
            } => {
                assert_eq!(center, CamLayout::BOTTOM_RIGHT.center);
                assert!(
                    (half_extents.x - half_extents.y).abs() < 1e-6,
                    "square canvas → equal half-extents (circle)"
                );
                assert!((half_extents.x - CamLayout::BOTTOM_RIGHT.radius).abs() < 1e-6);
            }
            other => panic!("expected Ellipse, got {other:?}"),
        }
    }

    #[test]
    fn cam_bubble_aspect_compensated_on_landscape_canvas() {
        let app = boot();
        // 16:9 landscape: the x half-extent must shrink so the bubble
        // is a true circle in PIXELS, not a horizontally-stretched
        // ellipse (the recorded-output bug). Equal pixel radii means
        // `hx * width == hy * height`.
        let scene = RecordingScene::new(
            &app,
            StreamDimensions::new(1920, 1080),
            StreamDimensions::new(64, 64),
            CamLayout::BOTTOM_RIGHT,
        );
        let clip = scene
            .stage()
            .get(scene.cam_container_id())
            .expect("cam container alive")
            .container()
            .clip
            .expect("cam container has clip");
        let MaskShape::Ellipse {
            half_extents: he, ..
        } = clip
        else {
            panic!("expected Ellipse, got {clip:?}");
        };
        let r = CamLayout::BOTTOM_RIGHT.radius;
        // min_side = 1080 → hx = r * 1080/1920, hy = r.
        assert!((he.x - r * 1080.0 / 1920.0).abs() < 1e-6, "hx compensated");
        assert!((he.y - r).abs() < 1e-6, "hy unchanged on landscape");
        assert!(
            (he.x * 1920.0 - he.y * 1080.0).abs() < 1e-3,
            "equal pixel radii → true circle"
        );
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
