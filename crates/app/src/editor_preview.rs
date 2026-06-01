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
use edit::style::{BackgroundConfig, BackgroundSource, CropRect, CursorConfig};
use edit::zoom_anim::ZoomTransform;
use playback::EditorPlayer;
use wisp::recording::{CursorRipple, StreamDimensions};
use wisp::{Color, MaskShape, Rect, Stroke, Transform, Vec2};

use crate::recording::FrameSlot;
use crate::recording_compose::{ComposedFrame, RecordingCompose};

/// Base scale that fills the NDC `[-1, 1]` viewport: the screen sprite is
/// anchored centre with a `[0, 1]²` local rect, so a scale of `2` maps the
/// full source frame onto `[-1, 1]`.
const FILL_SCALE: f64 = 2.0;

/// Cursor pointer half-extent in NDC at 100 % size and 1× zoom (ED.19). The
/// pointer grows with `size_pct` and with the zoom (it's part of the
/// magnified content).
const CURSOR_BASE_HALF: f32 = 0.05;

/// Resolution the procedural wallpaper (ISS-15) is generated at — small, since
/// a soft gradient stretches cleanly to fill the frame.
const WALLPAPER_GEN_W: u32 = 320;
/// See [`WALLPAPER_GEN_W`].
const WALLPAPER_GEN_H: u32 = 180;

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

/// [`framed_transform`] with a uniform **padding** inset folded in (ED.18).
///
/// Background framing lifts the screen off its backdrop by `padding` pixels.
/// In NDC that's a centred shrink by `(k_x, k_y)` about the origin, and since
/// both the base fill and the zoom focal-pin are about the origin, the inset
/// is *exactly* a post-multiply: `scale *= k`, `position *= k`. With
/// `k = (1, 1)` it reduces to [`framed_transform`].
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "padding factors are in (0, 1] and the transform components are small NDC values, well within f32 precision"
)]
fn framed_transform_padded(zoom: ZoomTransform, crop: CropRect, k_x: f64, k_y: f64) -> Transform {
    let t = framed_transform(zoom, crop);
    let (kx, ky) = (k_x as f32, k_y as f32);
    Transform {
        scale: Vec2::new(t.scale.x * kx, t.scale.y * ky),
        position: Vec2::new(t.position.x * kx, t.position.y * ky),
        ..t
    }
}

/// Derive the rounded-corner frame window + corner radius (both NDC) and the
/// per-axis padding shrink factor from the canvas dims and the config's
/// pixel padding / corner radius (ED.18). Pure — unit-tested without a GPU.
///
/// `padding` px → NDC margin `2·padding/axis` (NDC spans `[-1, 1]` = 2 units
/// per axis), so the screen shrinks to `[-k, k]` with `k = 1 − 2·padding/axis`.
/// The window rect is that `[-k, k]²` region; `corner_radius` px → NDC
/// `2·corner_radius/width` (isotropic in NDC).
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "NDC window extents (|·| ≤ 2) and the corner radius fraction are small, well within f32 precision"
)]
fn background_geometry(
    width: u32,
    height: u32,
    padding: u32,
    corner_radius: u32,
) -> (Rect, f32, (f64, f64)) {
    let w = f64::from(width.max(1));
    let h = f64::from(height.max(1));
    let k_x = (1.0 - 2.0 * f64::from(padding) / w).clamp(0.05, 1.0);
    let k_y = (1.0 - 2.0 * f64::from(padding) / h).clamp(0.05, 1.0);
    let window = Rect::new(
        -(k_x as f32),
        -(k_y as f32),
        (2.0 * k_x) as f32,
        (2.0 * k_y) as f32,
    );
    let corner_ndc = (2.0 * f64::from(corner_radius) / w) as f32;
    (window, corner_ndc, (k_x, k_y))
}

/// Three RGB anchor colors for a named wallpaper palette (ISS-15). Unknown
/// names fall to the default "aurora".
fn wallpaper_palette(name: &str) -> [[u8; 3]; 3] {
    match name.to_ascii_lowercase().as_str() {
        "sunset" => [[255, 94, 98], [255, 195, 113], [113, 70, 132]],
        "ocean" | "aqua" => [[0, 79, 131], [0, 160, 176], [137, 218, 196]],
        "forest" | "mint" => [[20, 60, 50], [44, 110, 73], [149, 200, 120]],
        // "aurora" / default — warm→cool diagonal, matching the gradient default.
        _ => [[40, 53, 147], [123, 67, 151], [255, 138, 128]],
    }
}

/// Generate a procedural wallpaper backdrop as packed RGBA8 (`width*height*4`).
/// A soft diagonal three-stop gradient with a faint aurora band, keyed on
/// `name`'s palette. License-clean (no bundled asset), deterministic, and pure
/// — exhaustively testable. (ISS-15.)
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel coords + small dims are exact in f32; the channel result is clamped to [0,255] and rounded before the f32→u8 cast"
)]
fn wallpaper_rgba(name: &str, width: u32, height: u32) -> Vec<u8> {
    let [c0, c1, c2] = wallpaper_palette(name);
    let cols = width.max(1) as usize;
    let rows = height.max(1) as usize;
    let (wf, hf) = (cols as f32, rows as f32);
    let lerp = |lo: [u8; 3], hi: [u8; 3], t: f32| -> [u8; 3] {
        let t = t.clamp(0.0, 1.0);
        let chan = |lc: u8, hc: u8| (f32::from(lc) + (f32::from(hc) - f32::from(lc)) * t).round();
        [
            chan(lo[0], hi[0]) as u8,
            chan(lo[1], hi[1]) as u8,
            chan(lo[2], hi[2]) as u8,
        ]
    };
    let mut out = vec![0u8; cols * rows * 4];
    for row in 0..rows {
        for col in 0..cols {
            let (xf, yf) = (col as f32, row as f32);
            // Diagonal sweep across the two halves of the palette.
            let diag = (xf / wf).midpoint(yf / hf);
            // A faint aurora band on the cross-diagonal, lifting toward c2.
            let band = (((xf / wf - yf / hf) * std::f32::consts::TAU).sin() * 0.5 + 0.5) * 0.22;
            let base = if diag < 0.5 {
                lerp(c0, c1, diag * 2.0)
            } else {
                lerp(c1, c2, (diag - 0.5) * 2.0)
            };
            let px = lerp(base, c2, band);
            let idx = (row * cols + col) * 4;
            out[idx] = px[0];
            out[idx + 1] = px[1];
            out[idx + 2] = px[2];
            out[idx + 3] = 255;
        }
    }
    out
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
    /// Per-axis padding shrink factor applied to the screen transform (ED.18).
    /// `(1, 1)` until [`Self::set_background`] folds the config's padding in.
    pad: (f64, f64),
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
            pad: (1.0, 1.0),
        })
    }

    /// Apply the project's **background framing** (ED.18): the backdrop fill,
    /// the padding inset, and the rounded-corner frame window. Call once when
    /// the config changes — the export sets it once at generator construction;
    /// a live preview re-applies it on edit. After this, the per-frame
    /// [`Self::render_framed`] folds the stored padding factor into the screen
    /// transform, the screen sprite is clipped to the rounded window, and the
    /// backdrop renders behind it.
    ///
    /// Drop-shadow (`shadow`) lifts the screen off the backdrop; the inset
    /// border (`inset`) traces a colored edge just inside the frame — both
    /// rendered here (ED.18). A `Wallpaper` source renders a procedural
    /// backdrop ([`wallpaper_rgba`], ISS-15) — license-clean, no bundled asset.
    pub fn set_background(&mut self, bg: &BackgroundConfig) {
        let (window, corner_ndc, pad) =
            background_geometry(self.width, self.height, bg.padding, bg.corner_radius);
        self.pad = pad;
        self.compose
            .set_screen_clip(Some(MaskShape::rounded_rect(window, corner_ndc)));
        self.apply_shadow_and_border(bg, window, corner_ndc);

        // The wallpaper is a Sprite; the gradient/color is a Graphics. The
        // renderer batches by pipeline, so the two backdrop layers can't
        // coexist (the Graphics would paint over the Sprite) — they're
        // mutually exclusive. Each arm shows its layer and hides the other.
        match &bg.source {
            BackgroundSource::Gradient {
                from,
                to,
                angle_deg,
            } => {
                self.compose.set_background_gradient(
                    Color::rgb_u8(from[0], from[1], from[2]),
                    Color::rgb_u8(to[0], to[1], to[2]),
                    *angle_deg,
                );
                self.compose.set_background_wallpaper_visible(false);
            }
            BackgroundSource::Color { rgb } => {
                self.compose
                    .set_background_color(Color::rgb_u8(rgb[0], rgb[1], rgb[2]));
                self.compose.set_background_wallpaper_visible(false);
            }
            BackgroundSource::Wallpaper { name } => {
                // Procedural wallpaper (ISS-15) — license-clean, no bundled
                // asset, deterministic per `name`. Generated small + stretched
                // by the Sprite (a soft gradient doesn't need resolution).
                let rgba = wallpaper_rgba(name, WALLPAPER_GEN_W, WALLPAPER_GEN_H);
                self.compose
                    .set_background_wallpaper(WALLPAPER_GEN_W, WALLPAPER_GEN_H, &rgba);
                self.compose.set_background_visible(false);
            }
        }
    }

    /// Apply the drop shadow + inset border for the frame window (ED.18).
    /// `shadow` (0..=100) scales a dark, down-right-offset rounded-rect behind
    /// the screen; `inset` (px) strokes a colored border tracing the window.
    /// Each hides its node when its value is 0.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "shadow/border NDC magnitudes are small fractions, well within f32 precision"
    )]
    fn apply_shadow_and_border(&mut self, bg: &BackgroundConfig, window: Rect, corner_ndc: f32) {
        if bg.shadow > 0 {
            let s = f32::from(u16::try_from(bg.shadow.min(100)).unwrap_or(60)) / 100.0;
            let mag = s * 0.035;
            self.compose.set_frame_shadow(
                window,
                corner_ndc,
                // Down (+y is up, so down = −y) and slightly right.
                Vec2::new(mag * 0.5, -mag),
                Color::rgba(0.0, 0.0, 0.0, (s * 0.55).min(0.55)),
            );
        } else {
            self.compose.set_frame_shadow_visible(false);
        }

        if bg.inset > 0 {
            let stroke_ndc = (2.0 * f64::from(bg.inset) / f64::from(self.width.max(1))) as f32;
            self.compose.set_frame_border(
                window,
                corner_ndc,
                Stroke::new(stroke_ndc, Color::rgba(1.0, 1.0, 1.0, 0.7)),
            );
        } else {
            self.compose.set_frame_border_visible(false);
        }
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
    /// `crop`/aspect reframe (ED.15), the `zoom` punch-in (ED.16), and the
    /// background **padding** inset (ED.18) are written into the screen
    /// sprite's transform, then the frame composes through the same path as
    /// [`Self::render_frame`]. The backdrop + rounded-corner clip are set
    /// once via [`Self::set_background`]; this only updates the per-frame
    /// transform. The padding factor is `(1, 1)` until `set_background` runs,
    /// so an un-styled call is the plain ED.16 transform. This is the export
    /// generator's entry point (ED.20), so preview and export agree
    /// frame-for-frame. `bgra` is top-down packed BGRA8 of `width*height*4`.
    #[must_use]
    pub fn render_framed(
        &mut self,
        bgra: Vec<u8>,
        zoom: ZoomTransform,
        crop: CropRect,
    ) -> Option<ComposedFrame> {
        let (k_x, k_y) = self.pad;
        self.compose
            .set_screen_transform(framed_transform_padded(zoom, crop, k_x, k_y));
        self.render_frame(bgra)
    }

    /// Like [`Self::render_framed`], plus the cursor overlay (ED.19). `cursor`
    /// is the smoothed normalized position at this frame
    /// ([`edit::telemetry::cursor_at`]); `ripples` are the active click
    /// ripples (`(x, y, age)` from [`edit::telemetry::ripples_at`]). Both the
    /// pointer and each ripple are mapped through the **same** screen
    /// transform as the frame, so the cursor stays glued to its pixel through
    /// zoom / crop / padding. `cursor = None` hides the overlay (e.g. before
    /// any sample, or a `hide_static` gap the caller decides).
    #[must_use]
    pub fn render_framed_with_cursor(
        &mut self,
        bgra: Vec<u8>,
        zoom: ZoomTransform,
        crop: CropRect,
        cursor: Option<(f32, f32)>,
        ripples: &[(f32, f32, f32)],
        cfg: &CursorConfig,
    ) -> Option<ComposedFrame> {
        let (k_x, k_y) = self.pad;
        let t = framed_transform_padded(zoom, crop, k_x, k_y);
        self.compose.set_screen_transform(t);

        // Map a normalized source point (top-left origin) through the same
        // transform the screen sprite carries — `pos + scale · (local)` with
        // the `+y`-up flip on the local `y` (matching `framed_transform`).
        let map = |x: f32, y: f32| {
            Vec2::new(
                t.position.x + t.scale.x * (x - 0.5),
                t.position.y + t.scale.y * -(y - 0.5),
            )
        };

        if let Some((cx, cy)) = cursor {
            // Pointer half-size: base × size_pct, grown by the transform's
            // scale (base scale is 2, so `scale.y / 2` is the z·k_y zoom
            // factor) — the cursor magnifies with the punch-in.
            let size_factor = f32::from(u16::try_from(cfg.size_pct).unwrap_or(180)) / 100.0;
            // Base sprite scale is 2 (`FILL_SCALE`), so `scale.y / 2` is the
            // z·k_y zoom factor — the pointer magnifies with the punch-in.
            let zoom_scale = (t.scale.y / 2.0).abs();
            let half = CURSOR_BASE_HALF * size_factor * zoom_scale;
            let rings: Vec<CursorRipple> = if cfg.click_ripples {
                ripples
                    .iter()
                    .map(|&(rx, ry, age)| CursorRipple {
                        center: map(rx, ry),
                        radius: half * (1.0 + 3.0 * age),
                        alpha: (1.0 - age) * 0.35,
                    })
                    .collect()
            } else {
                Vec::new()
            };
            self.compose.set_cursor(map(cx, cy), half, &rings);
            self.compose.set_cursor_visible(true);
        } else {
            self.compose.set_cursor_visible(false);
        }
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
    fn framed_transform_padded_with_unit_factor_equals_unpadded() {
        // k = (1, 1) ⇒ no inset ⇒ identical to framed_transform.
        let z = ZoomTransform {
            scale: 1.6,
            center_x: 0.3,
            center_y: 0.7,
        };
        let crop = CropRect {
            x: 0.1,
            y: 0.05,
            width: 0.7,
            height: 0.8,
        };
        let plain = framed_transform(z, crop);
        let padded = framed_transform_padded(z, crop, 1.0, 1.0);
        assert!((plain.scale.x - padded.scale.x).abs() < 1e-6);
        assert!((plain.scale.y - padded.scale.y).abs() < 1e-6);
        assert!((plain.position.x - padded.position.x).abs() < 1e-6);
        assert!((plain.position.y - padded.position.y).abs() < 1e-6);
    }

    #[test]
    fn framed_transform_padded_shrinks_about_the_centre() {
        // No zoom / crop: base fill scale 2 → with k = 0.5 → scale 1, pos 0.
        let t = framed_transform_padded(ZoomTransform::identity(), CropRect::full(), 0.5, 0.5);
        assert!((t.scale.x - 1.0).abs() < 1e-6 && (t.scale.y - 1.0).abs() < 1e-6);
        assert!(t.position.x.abs() < 1e-6 && t.position.y.abs() < 1e-6);
    }

    #[test]
    fn framed_transform_padded_pins_the_focal_corner_inside_the_window() {
        // 2× at top-right pins focal NDC (1, 1) → unpadded pos (−1, −1),
        // scale 4. With k = 0.5 the pinned corner lands at the padded
        // window corner: scale 2, pos (−0.5, −0.5).
        let z = ZoomTransform {
            scale: 2.0,
            center_x: 1.0,
            center_y: 0.0,
        };
        let t = framed_transform_padded(z, CropRect::full(), 0.5, 0.5);
        assert!((t.scale.x - 2.0).abs() < 1e-6);
        assert!(
            (t.position.x + 0.5).abs() < 1e-6,
            "right edge pinned to window"
        );
        assert!(
            (t.position.y + 0.5).abs() < 1e-6,
            "top edge pinned to window"
        );
    }

    #[test]
    fn background_geometry_matches_reference_defaults() {
        // Reference design: padding 64, corner 14, on a 1920×1080 canvas.
        let (window, corner, (k_x, k_y)) = background_geometry(1920, 1080, 64, 14);
        // k_x = 1 − 128/1920 = 0.93333; k_y = 1 − 128/1080 = 0.88148.
        assert!((k_x - 0.933_333).abs() < 1e-4);
        assert!((k_y - 0.881_481).abs() < 1e-4);
        assert!((window.min.x + 0.933_333).abs() < 1e-4);
        assert!((window.min.y + 0.881_481).abs() < 1e-4);
        assert!((window.size.x - 1.866_667).abs() < 1e-4);
        assert!((window.size.y - 1.762_963).abs() < 1e-4);
        // corner_ndc = 2·14/1920 = 0.014583.
        assert!((corner - 0.014_583).abs() < 1e-5);
    }

    #[test]
    fn background_geometry_clamps_pathological_padding() {
        // Padding ≥ half the canvas would invert the screen — clamp to a
        // small positive factor instead.
        let (_w, _c, (k_x, k_y)) = background_geometry(128, 128, 200, 0);
        assert!(
            k_x >= 0.05 && k_y >= 0.05,
            "padding clamped, screen survives"
        );
    }

    /// GPU: a gradient backdrop + padding + rounded corners on a solid-white
    /// "screen" frame. The centre stays white (screen visible); the corner
    /// margin is the backdrop (rounded corners cut the white + padding reveals
    /// the backdrop); and the backdrop is a real gradient (the margin colour
    /// varies across the canvas, not a flat fill). Single-bind-group clip +
    /// graphics — no blur — so it runs on every CI OS (no lavapipe guard).
    #[test]
    fn render_framed_with_gradient_background_frames_the_screen() {
        const SIDE: usize = 128;
        let dim = u32::try_from(SIDE).expect("SIDE fits u32");
        let mut preview = EditorPreview::new(dim, dim).expect("init wgpu");
        let bg = BackgroundConfig {
            source: BackgroundSource::Gradient {
                from: [255, 138, 128],
                to: [40, 53, 147],
                angle_deg: 135.0,
            },
            padding: 16,
            corner_radius: 24,
            shadow: 0,
            inset: 0,
        };
        preview.set_background(&bg);
        let white = vec![255u8; SIDE * SIDE * 4];
        let frame = preview
            .render_framed(white, ZoomTransform::identity(), CropRect::full())
            .expect("compose");
        let pixel = |col: usize, row: usize| {
            let base = (row * SIDE + col) * 4;
            [
                frame.bytes[base],
                frame.bytes[base + 1],
                frame.bytes[base + 2],
            ] // B, G, R
        };
        let is_white = |bgr: [u8; 3]| bgr[0] > 230 && bgr[1] > 230 && bgr[2] > 230;
        // Centre: the white screen shows through the rounded window.
        assert!(
            is_white(pixel(SIDE / 2, SIDE / 2)),
            "centre is the white screen"
        );
        // Corners: padding margin + rounded-corner cut → backdrop, not white,
        // and not black (the gradient drew).
        let corners = [
            pixel(3, 3),
            pixel(3, SIDE - 4),
            pixel(SIDE - 4, 3),
            pixel(SIDE - 4, SIDE - 4),
        ];
        for bgr in corners {
            assert!(
                !is_white(bgr),
                "corner is backdrop, not the white screen ({bgr:?})"
            );
            let lum = u16::from(bgr[0]) + u16::from(bgr[1]) + u16::from(bgr[2]);
            assert!(lum > 30, "backdrop drew (not black) at a corner ({bgr:?})");
        }
        // It's a gradient, not a flat fill: the corner colours span a range.
        let reds: Vec<i32> = corners.iter().map(|bgr| i32::from(bgr[2])).collect();
        let spread = reds.iter().max().unwrap() - reds.iter().min().unwrap();
        assert!(
            spread > 15,
            "backdrop varies across the canvas (gradient), spread={spread}"
        );
    }

    /// GPU: a flat-colour backdrop. The margin equals the requested colour's
    /// channel ordering (R-dominant here) and is neither white nor black.
    #[test]
    fn render_framed_with_color_background_fills_the_margin() {
        const SIDE: usize = 128;
        let dim = u32::try_from(SIDE).expect("SIDE fits u32");
        let mut preview = EditorPreview::new(dim, dim).expect("init wgpu");
        preview.set_background(&BackgroundConfig {
            source: BackgroundSource::Color { rgb: [210, 40, 70] },
            padding: 16,
            corner_radius: 8,
            shadow: 0,
            inset: 0,
        });
        let white = vec![255u8; SIDE * SIDE * 4];
        let frame = preview
            .render_framed(white, ZoomTransform::identity(), CropRect::full())
            .expect("compose");
        // A mid-edge margin pixel (outside the padded window, away from the
        // rounded corners): the flat backdrop colour.
        let base = ((SIDE / 2) * SIDE + 3) * 4; // row centre, col 3 → left margin
        let (blue, green, red) = (
            frame.bytes[base],
            frame.bytes[base + 1],
            frame.bytes[base + 2],
        );
        assert!(
            red > green && red > blue,
            "R-dominant fill (got B{blue} G{green} R{red})"
        );
        assert!(red > 60, "backdrop colour drew (not black)");
        assert!(
            !(red > 230 && green > 230 && blue > 230),
            "not the white screen"
        );
    }

    /// GPU: the drop shadow + inset border (ED.18) visibly change the composed
    /// frame versus the same backdrop with `shadow = 0, inset = 0`. Pure
    /// Graphics (no blur) → runs on every CI OS, no lavapipe guard.
    #[test]
    fn render_with_shadow_and_border_differs_from_without() {
        const SIDE: usize = 128;
        let dim = u32::try_from(SIDE).expect("SIDE fits u32");
        let mut pv = EditorPreview::new(dim, dim).expect("init wgpu");
        let cfg = |shadow: u32, inset: u32| BackgroundConfig {
            source: BackgroundSource::Color {
                rgb: [235, 235, 240],
            },
            padding: 18,
            corner_radius: 10,
            shadow,
            inset,
        };
        let gray = || vec![128u8; SIDE * SIDE * 4];

        pv.set_background(&cfg(0, 0));
        let plain = pv
            .render_framed(gray(), ZoomTransform::identity(), CropRect::full())
            .expect("compose");
        pv.set_background(&cfg(95, 8));
        let decorated = pv
            .render_framed(gray(), ZoomTransform::identity(), CropRect::full())
            .expect("compose");

        let diff = plain
            .bytes
            .iter()
            .zip(decorated.bytes.iter())
            .filter(|(a, b)| a.abs_diff(**b) > 16)
            .count();
        assert!(
            diff > 200,
            "shadow + inset border visibly change the frame ({diff} bytes differ)"
        );
    }

    #[test]
    fn wallpaper_rgba_is_well_formed_and_name_keyed() {
        let aurora = wallpaper_rgba("aurora", 32, 18);
        assert_eq!(aurora.len(), 32 * 18 * 4, "packed RGBA8 of the right size");
        assert!(aurora.chunks_exact(4).all(|p| p[3] == 255), "opaque");
        // Unknown name falls to the default (aurora).
        assert_eq!(wallpaper_rgba("nonsense", 32, 18), aurora);
        // Different palettes produce different pixels.
        let ocean = wallpaper_rgba("ocean", 32, 18);
        assert_ne!(ocean, aurora, "named palettes differ");
        // Ocean is blue-dominant (its anchors are all blue/teal).
        assert!(
            ocean
                .chunks_exact(4)
                .all(|p| u16::from(p[2]) >= u16::from(p[0])),
            "ocean wallpaper: blue ≥ red everywhere"
        );
    }

    /// GPU: a `Wallpaper` backdrop fills the margin with the (procedural)
    /// wallpaper — here the blue-dominant "ocean" palette — behind the white
    /// screen. Sprite backdrop, single-bind-group → no lavapipe guard.
    #[test]
    fn render_with_wallpaper_backdrop_fills_the_margin() {
        const SIDE: usize = 128;
        let dim = u32::try_from(SIDE).expect("SIDE fits u32");
        let mut pv = EditorPreview::new(dim, dim).expect("init wgpu");
        pv.set_background(&BackgroundConfig {
            source: BackgroundSource::Wallpaper {
                name: "ocean".to_string(),
            },
            padding: 18,
            corner_radius: 8,
            shadow: 0,
            inset: 0,
        });
        let white = vec![255u8; SIDE * SIDE * 4];
        let f = pv
            .render_framed(white, ZoomTransform::identity(), CropRect::full())
            .expect("compose");
        // Centre: the white screen.
        let ci = ((SIDE / 2) * SIDE + SIDE / 2) * 4;
        assert!(
            f.bytes[ci] > 230 && f.bytes[ci + 1] > 230 && f.bytes[ci + 2] > 230,
            "centre is the white screen"
        );
        // A margin pixel (outside the padded window): the ocean wallpaper —
        // blue-dominant (B > R), not white.
        let mi = (4 * SIDE + 4) * 4;
        let (b, g, r) = (f.bytes[mi], f.bytes[mi + 1], f.bytes[mi + 2]);
        assert!(
            b > r,
            "ocean wallpaper is blue-dominant in the margin (B{b} R{r})"
        );
        assert!(
            !(b > 230 && g > 230 && r > 230),
            "margin is the wallpaper, not the white screen"
        );
    }

    /// GPU: the cursor overlay (ED.19) draws a pointer and — the load-bearing
    /// decision — *rides the screen transform*: the same source point lands
    /// further from centre under a zoom (glued to its pixel), not pinned in
    /// output space. Detected via the pointer's white inner triangle on a
    /// mid-grey screen. Single-bind-group graphics — no blur, no lavapipe
    /// guard.
    #[test]
    #[allow(
        clippy::cast_precision_loss,
        reason = "SIDE = 128, so every pixel count / coordinate is a small integer exact in f64"
    )]
    fn render_framed_with_cursor_rides_the_zoom() {
        const SIDE: usize = 128;
        let dim = u32::try_from(SIDE).expect("SIDE fits u32");
        let mut pv = EditorPreview::new(dim, dim).expect("init wgpu");
        let cfg = CursorConfig::default();
        let gray = || vec![128u8; SIDE * SIDE * 4];

        // The pointer's white inner triangle — count + centroid of white px.
        let white = |f: &ComposedFrame| -> (usize, f64, f64) {
            let (mut n, mut sx, mut sy) = (0usize, 0.0, 0.0);
            for row in 0..SIDE {
                for col in 0..SIDE {
                    let i = (row * SIDE + col) * 4;
                    if f.bytes[i] > 220 && f.bytes[i + 1] > 220 && f.bytes[i + 2] > 220 {
                        n += 1;
                        sx += col as f64;
                        sy += row as f64;
                    }
                }
            }
            if n == 0 {
                (0, 0.0, 0.0)
            } else {
                (n, sx / n as f64, sy / n as f64)
            }
        };

        // No cursor → a flat grey frame has no white pointer pixels.
        let none = pv
            .render_framed_with_cursor(
                gray(),
                ZoomTransform::identity(),
                CropRect::full(),
                None,
                &[],
                &cfg,
            )
            .expect("compose");
        assert_eq!(white(&none).0, 0, "no pointer drawn when position is None");

        // Cursor at the top-left quadrant (0.25, 0.25) at 1×.
        let one = pv
            .render_framed_with_cursor(
                gray(),
                ZoomTransform::identity(),
                CropRect::full(),
                Some((0.25, 0.25)),
                &[],
                &cfg,
            )
            .expect("compose");
        let (n1, cx1, cy1) = white(&one);
        assert!(n1 > 4, "pointer visible at 1× ({n1} white px)");

        // Same source point under a 2× centre zoom → it rides toward the
        // corner (further from centre), proving the cursor is glued to its
        // pixel rather than pinned in output space.
        let two = pv
            .render_framed_with_cursor(
                gray(),
                ZoomTransform {
                    scale: 2.0,
                    center_x: 0.5,
                    center_y: 0.5,
                },
                CropRect::full(),
                Some((0.25, 0.25)),
                &[],
                &cfg,
            )
            .expect("compose");
        let (n2, cx2, cy2) = white(&two);
        assert!(n2 > 4, "pointer visible at 2× ({n2} white px)");

        let center = (SIDE as f64) / 2.0;
        let d1 = (cx1 - center).hypot(cy1 - center);
        let d2 = (cx2 - center).hypot(cy2 - center);
        assert!(
            d2 > d1,
            "cursor rides the zoom toward the corner (1×→{d1:.1}, 2×→{d2:.1} from centre)"
        );
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
