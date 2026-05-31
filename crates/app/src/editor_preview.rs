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
use edit::style::CropRect;
use edit::zoom_anim::ZoomTransform;
use playback::EditorPlayer;
use wisp::recording::StreamDimensions;
use wisp::{Transform, Vec2};

use crate::recording::FrameSlot;
use crate::recording_compose::{ComposedFrame, RecordingCompose};

/// Base scale that fills the NDC `[-1, 1]` viewport: the screen sprite is
/// anchored centre with a `[0, 1]²` local rect, so a scale of `2` maps the
/// full source frame onto `[-1, 1]`.
const FILL_SCALE: f64 = 2.0;

/// Build the screen-sprite transform that applies `crop` then `zoom`.
///
/// The base screen sprite maps a normalized source point `(u, v)` to NDC
/// `(2u − 1, −(2v − 1))` (centre-anchored, scale 2; the `−` on `v` is wisp's
/// `+y`-up convention — the decoded top-down frame is flipped bottom-up at
/// upload). This composes two affines into the single transform the sprite
/// carries:
///
/// 1. **Crop** `[x, y, w, h]` → fill the sub-rect: scale `2/w, 2/h` and
///    recentre so the crop centre lands at NDC `(0, 0)`.
/// 2. **Zoom** by `z` about the focal NDC point `(2·fx − 1, −(2·fy − 1))`,
///    keeping that point fixed: `pos += focal · (1 − z)`.
///
/// Composed `zoom ∘ crop`: `scale = z · crop_scale`,
/// `pos = z · crop_pos + zoom_pos`. Pure — no GPU, exhaustively testable.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "screen-space NDC transform components are small; the f64→f32 narrowing for the wisp Vec2 is intentional and well within f32 precision"
)]
fn framed_transform(zoom: ZoomTransform, crop: CropRect) -> Transform {
    // Crop arm: fill sub-rect [x, x+w] × [y, y+h]. `CropRect` is sanitized
    // to a non-zero in-frame rect, but clamp defensively against /0.
    let w = f64::from(crop.width).max(1e-3);
    let h = f64::from(crop.height).max(1e-3);
    let crop_scale = (FILL_SCALE / w, FILL_SCALE / h);
    let crop_pos = (
        (1.0 - 2.0 * f64::from(crop.x)) / w - 1.0,
        (2.0 * f64::from(crop.y) - 1.0) / h + 1.0,
    );

    // Zoom arm: magnify by `z` about the focal NDC point, keeping it fixed.
    let z = zoom.scale.max(1.0);
    let focal = (2.0 * zoom.center_x - 1.0, -(2.0 * zoom.center_y - 1.0));
    let zoom_pos = (focal.0 * (1.0 - z), focal.1 * (1.0 - z));

    Transform {
        scale: Vec2::new((z * crop_scale.0) as f32, (z * crop_scale.1) as f32),
        position: Vec2::new(
            (z * crop_pos.0 + zoom_pos.0) as f32,
            (z * crop_pos.1 + zoom_pos.1) as f32,
        ),
        ..Transform::IDENTITY
    }
}

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

    /// Compose a source frame with the editor's **framing** applied — the
    /// `crop`/aspect reframe (ED.15) and `zoom` punch-in (ED.16) are written
    /// into the screen sprite's transform, then the frame composes through
    /// the same path as [`Self::render_frame`]. This is the export
    /// generator's entry point (ED.20), so preview and export agree
    /// frame-for-frame. `bgra` is top-down packed BGRA8 of `width*height*4`.
    #[must_use]
    pub fn render_framed(
        &mut self,
        bgra: Vec<u8>,
        zoom: ZoomTransform,
        crop: CropRect,
    ) -> Option<ComposedFrame> {
        self.compose
            .set_screen_transform(framed_transform(zoom, crop));
        self.render_frame(bgra)
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

    #[test]
    fn framed_transform_is_base_fill_with_no_zoom_no_crop() {
        let t = framed_transform(ZoomTransform::identity(), CropRect::full());
        assert!((t.scale.x - 2.0).abs() < 1e-5 && (t.scale.y - 2.0).abs() < 1e-5);
        assert!(t.position.x.abs() < 1e-5 && t.position.y.abs() < 1e-5);
    }

    #[test]
    fn framed_transform_zoom_at_centre_magnifies_in_place() {
        let z = ZoomTransform {
            scale: 2.0,
            center_x: 0.5,
            center_y: 0.5,
        };
        let t = framed_transform(z, CropRect::full());
        // 2× the base fill (→ 4) about the centred focal → no translation.
        assert!((t.scale.x - 4.0).abs() < 1e-5 && (t.scale.y - 4.0).abs() < 1e-5);
        assert!(t.position.x.abs() < 1e-5 && t.position.y.abs() < 1e-5);
    }

    #[test]
    fn framed_transform_zoom_pins_the_focal_corner() {
        // 2× at top-right (1, 0): focal NDC (1, 1) stays fixed →
        // pos = (1, 1) · (1 − 2) = (−1, −1).
        let z = ZoomTransform {
            scale: 2.0,
            center_x: 1.0,
            center_y: 0.0,
        };
        let t = framed_transform(z, CropRect::full());
        assert!((t.scale.x - 4.0).abs() < 1e-5);
        assert!((t.position.x + 1.0).abs() < 1e-5, "right edge pinned");
        assert!((t.position.y + 1.0).abs() < 1e-5, "top edge pinned");
    }

    #[test]
    fn framed_transform_crop_fills_the_subrect() {
        // Top-left quadrant [0, 0, 0.5, 0.5] fills the frame: scale
        // 2 / 0.5 = 4 each; the quadrant maps [0, 0.5] → [−1, 1] (pos (1, −1)).
        let crop = CropRect {
            x: 0.0,
            y: 0.0,
            width: 0.5,
            height: 0.5,
        };
        let t = framed_transform(ZoomTransform::identity(), crop);
        assert!((t.scale.x - 4.0).abs() < 1e-5 && (t.scale.y - 4.0).abs() < 1e-5);
        assert!((t.position.x - 1.0).abs() < 1e-5);
        assert!((t.position.y + 1.0).abs() < 1e-5);
    }

    #[test]
    fn framed_transform_clamps_sub_one_zoom_to_no_zoom() {
        // A sub-1.0 amount can't shrink (clamped in zoom_at; defended here).
        let z = ZoomTransform {
            scale: 0.5,
            center_x: 0.5,
            center_y: 0.5,
        };
        let t = framed_transform(z, CropRect::full());
        assert!((t.scale.x - 2.0).abs() < 1e-5, "clamped to base fill");
    }

    /// Golden render: a centred white marker on a dark field, composed with
    /// and without a 2× centre zoom. The zoom must *magnify* (not shrink or
    /// vanish), so the marker covers ~4× the pixels (2× linear → 4× area).
    /// A coarse pixel count is robust to driver AA / sub-pixel variation.
    #[test]
    fn render_framed_zoom_magnifies_the_focal_region() {
        const S: usize = 128;
        let pattern = {
            let mut b = vec![0u8; S * S * 4];
            for y in 0..S {
                for x in 0..S {
                    let i = (y * S + x) * 4;
                    if x.abs_diff(S / 2) < 8 && y.abs_diff(S / 2) < 8 {
                        b[i] = 255;
                        b[i + 1] = 255;
                        b[i + 2] = 255;
                    }
                    b[i + 3] = 255;
                }
            }
            b
        };
        let dim = u32::try_from(S).expect("S fits u32");
        let mut pv = EditorPreview::new(dim, dim).expect("init wgpu");
        let white = |f: &ComposedFrame| {
            f.bytes
                .chunks_exact(4)
                .filter(|p| p[0] > 240 && p[1] > 240 && p[2] > 240)
                .count()
        };
        let none = pv
            .render_framed(pattern.clone(), ZoomTransform::identity(), CropRect::full())
            .expect("compose");
        let zoomed = pv
            .render_framed(
                pattern,
                ZoomTransform {
                    scale: 2.0,
                    center_x: 0.5,
                    center_y: 0.5,
                },
                CropRect::full(),
            )
            .expect("compose");
        let (a, z) = (white(&none), white(&zoomed));
        assert!(a > 0 && z > 0, "marker visible in both ({a}, {z})");
        // 2× linear ⇒ ~4× area; allow [3×, 5×] for AA + clamping.
        assert!(
            z >= 3 * a && z <= 5 * a,
            "2× zoom should ~4× the marker area (got {a} → {z})"
        );
    }
}
