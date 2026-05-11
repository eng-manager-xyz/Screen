//! Stroked / outlined text rendering (M-TEXT.7 / AUT-81).
//!
//! Text that has to stay readable over a busy screen recording needs
//! a high-contrast outline. The standard CSS-style technique — stamp
//! the rendered text texture multiple times in the stroke color at
//! small offsets around a center, then stamp the fill color on top —
//! works without a shader change.
//!
//! Output of [`stroked_text_sprites`] is a `Vec<Sprite>` ready to be
//! attached to a [`crate::Container`]. The container's transform
//! handles position / scale; offsets are emitted in **local** NDC so
//! the container scale doesn't break stroke geometry.

use std::sync::Arc;

use glam::Vec2;

use crate::color::Color;
use crate::scene::Sprite;
use crate::texture::render_texture::RenderTexture;

/// Builder for a stroked-text composition.
#[derive(Debug, Clone)]
pub struct StrokedTextLayer {
    /// Color the glyph interior fills with.
    pub fill: Color,
    /// Color the outline ring is drawn in.
    pub stroke: Color,
    /// Outline radius in **local NDC** units (the container's
    /// untransformed NDC). `0.0` skips the stroke and emits just the
    /// fill sprite.
    pub stroke_width_ndc: f32,
}

impl Default for StrokedTextLayer {
    fn default() -> Self {
        Self {
            fill: Color::WHITE,
            stroke: Color::BLACK,
            stroke_width_ndc: 0.0,
        }
    }
}

/// Eight unit-vectors at 45° spacing — the offsets used to stamp the
/// stroke ring. More directions = smoother edges but more sprite
/// nodes per stroke.
#[expect(
    clippy::approx_constant,
    reason = "0.7071 is √2/2 in 4-digit literals — explicit for readability"
)]
const STROKE_OFFSETS: [(f32, f32); 8] = [
    (1.0, 0.0),
    (0.7071, 0.7071),
    (0.0, 1.0),
    (-0.7071, 0.7071),
    (-1.0, 0.0),
    (-0.7071, -0.7071),
    (0.0, -1.0),
    (0.7071, -0.7071),
];

/// Produce a list of sprites that, when attached to a container,
/// render the given text-texture with `fill` interior and `stroke`
/// outline at the configured `stroke_width_ndc`.
///
/// Sprites are emitted with `tint` set; the caller positions them via
/// the container's transform. The `+y` flip used by every text-texture
/// consumer (`scale.y = -1`) is **not** applied here — apply it on
/// the parent container or override per-sprite if your render-texture
/// already arrives oriented `+y`-up.
///
/// Order: stroke sprites first (background), fill sprite last. The
/// renderer draws in scene-tree order, so the fill sits on top.
///
/// Skipping the stroke entirely (`stroke_width_ndc == 0.0`) returns a
/// single-sprite vector.
#[must_use]
pub fn stroked_text_sprites(text_rt: &Arc<RenderTexture>, layer: &StrokedTextLayer) -> Vec<Sprite> {
    let tex = text_rt.as_texture();
    let mut sprites = Vec::with_capacity(if layer.stroke_width_ndc > 0.0 { 9 } else { 1 });

    if layer.stroke_width_ndc > 0.0 {
        for (dx, dy) in STROKE_OFFSETS {
            let mut s = Sprite::from_texture(tex.clone()).with_tint(layer.stroke);
            s.container.transform.position =
                Vec2::new(dx * layer.stroke_width_ndc, dy * layer.stroke_width_ndc);
            sprites.push(s);
        }
    }

    let fill_sprite = Sprite::from_texture(tex).with_tint(layer.fill);
    sprites.push(fill_sprite);

    sprites
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_rt(width: u32, height: u32) -> Arc<RenderTexture> {
        // Test only needs an `Arc<RenderTexture>` shape — construct
        // one via the real GPU path under pollster.
        use crate::application::{AppConfig, Application};
        use pollster::block_on;
        let app = block_on(Application::new(AppConfig::default())).expect("Application::new");
        Arc::new(RenderTexture::new(&app, width, height))
    }

    #[test]
    fn zero_stroke_emits_one_sprite() {
        let rt = dummy_rt(64, 32);
        let sprites = stroked_text_sprites(
            &rt,
            &StrokedTextLayer {
                stroke_width_ndc: 0.0,
                ..StrokedTextLayer::default()
            },
        );
        assert_eq!(sprites.len(), 1);
        assert_eq!(sprites[0].tint, Color::WHITE);
    }

    #[test]
    fn positive_stroke_emits_eight_plus_one() {
        let rt = dummy_rt(64, 32);
        let sprites = stroked_text_sprites(
            &rt,
            &StrokedTextLayer {
                fill: Color::WHITE,
                stroke: Color::BLACK,
                stroke_width_ndc: 0.01,
            },
        );
        assert_eq!(sprites.len(), 9);
        // First 8 are stroke sprites; last is fill.
        for stroke_sprite in &sprites[..8] {
            assert_eq!(stroke_sprite.tint, Color::BLACK);
        }
        assert_eq!(sprites[8].tint, Color::WHITE);
    }

    #[test]
    fn stroke_sprites_offset_at_configured_radius() {
        let rt = dummy_rt(64, 32);
        let r = 0.04_f32;
        let sprites = stroked_text_sprites(
            &rt,
            &StrokedTextLayer {
                fill: Color::WHITE,
                stroke: Color::BLACK,
                stroke_width_ndc: r,
            },
        );
        // Every stroke sprite's position lies on the radius-r ring (within fp tolerance).
        for s in &sprites[..8] {
            let p = s.container.transform.position;
            let dist = (p.x * p.x + p.y * p.y).sqrt();
            assert!(
                (dist - r).abs() < 1e-4,
                "stroke sprite at {p:?} is at radius {dist}, expected {r}",
            );
        }
        // Fill sprite is centered.
        let fill_pos = sprites[8].container.transform.position;
        assert!(fill_pos.length() < 1e-6);
    }

    #[test]
    fn stroke_width_scales_linearly() {
        let rt = dummy_rt(64, 32);
        for &r in &[0.001_f32, 0.01, 0.05, 0.1] {
            let sprites = stroked_text_sprites(
                &rt,
                &StrokedTextLayer {
                    fill: Color::WHITE,
                    stroke: Color::BLACK,
                    stroke_width_ndc: r,
                },
            );
            let p = sprites[0].container.transform.position;
            let dist = (p.x * p.x + p.y * p.y).sqrt();
            assert!((dist - r).abs() < 1e-4);
        }
    }
}
