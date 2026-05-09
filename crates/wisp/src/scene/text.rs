//! `Text` — bitmap-font glyph rendering.
//!
//! M0.15 ships the bitmap atlas backed by [`font8x8`]. Each glyph is an 8×8
//! bitmap; the atlas is a 16×16 grid of cells (128×128 pixels total). Vector
//! fonts via `fontdue` come in a later chunk if/when we need anti-aliased
//! type at multiple sizes.

use std::sync::Arc;

use font8x8::legacy::BASIC_LEGACY;

use crate::application::Application;
use crate::color::Color;
use crate::scene::container::Container;
use crate::texture::Texture;

/// Per-glyph metrics — UV rect in the atlas.
#[derive(Clone, Copy, Debug)]
pub struct GlyphMetrics {
    /// Left U coordinate in the atlas.
    pub u_min: f32,
    /// Top V coordinate in the atlas.
    pub v_min: f32,
    /// Right U coordinate in the atlas.
    pub u_max: f32,
    /// Bottom V coordinate in the atlas.
    pub v_max: f32,
}

/// Bitmap font — owns a glyph atlas and per-codepoint metrics. Cheaply
/// cloneable; the underlying atlas is `Arc`-wrapped.
#[derive(Clone)]
pub struct Font {
    inner: Arc<FontInner>,
}

struct FontInner {
    atlas: Texture,
    glyphs: [Option<GlyphMetrics>; 128],
    /// Pixels per glyph cell (8 for font8x8).
    cell_pixels: u32,
}

impl Font {
    /// Build the embedded 8×8 ASCII bitmap font.
    ///
    /// The returned `Font` carries its own atlas texture and glyph metrics —
    /// no external font files required.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "all values bounded by 128 (atlas dim) — fit losslessly in f32"
    )]
    pub fn bitmap_8x8(app: &Application) -> Self {
        const CELL: u32 = 8;
        const COLS: u32 = 16;
        const ROWS: u32 = 16;
        let atlas_w = COLS * CELL;
        let atlas_h = ROWS * CELL;

        let mut bytes = vec![0u8; (atlas_w * atlas_h * 4) as usize];
        let mut glyphs: [Option<GlyphMetrics>; 128] = [None; 128];

        for c in 0u32..128 {
            let glyph_rows = BASIC_LEGACY[c as usize];
            let cell_x = (c % COLS) * CELL;
            let cell_y = (c / COLS) * CELL;

            for row in 0u32..CELL {
                let bits = glyph_rows[row as usize];
                for col in 0u32..CELL {
                    let on = ((bits >> col) & 1) != 0;
                    let px = (cell_y + row) * atlas_w + (cell_x + col);
                    let idx = (px * 4) as usize;
                    if on {
                        bytes[idx] = 255;
                        bytes[idx + 1] = 255;
                        bytes[idx + 2] = 255;
                        bytes[idx + 3] = 255;
                    }
                }
            }

            // UVs: cell (cell_x, cell_y) of size (CELL, CELL).
            let atlas_width_f = atlas_w as f32;
            let atlas_height_f = atlas_h as f32;
            glyphs[c as usize] = Some(GlyphMetrics {
                u_min: cell_x as f32 / atlas_width_f,
                v_min: cell_y as f32 / atlas_height_f,
                u_max: (cell_x + CELL) as f32 / atlas_width_f,
                v_max: (cell_y + CELL) as f32 / atlas_height_f,
            });
        }

        let atlas = Texture::from_rgba(app, atlas_w, atlas_h, &bytes);

        Self {
            inner: Arc::new(FontInner {
                atlas,
                glyphs,
                cell_pixels: CELL,
            }),
        }
    }

    /// Pixels per cell side (8 for font8x8).
    #[must_use]
    pub fn cell_pixels(&self) -> u32 {
        self.inner.cell_pixels
    }

    pub(crate) fn atlas(&self) -> &Texture {
        &self.inner.atlas
    }

    pub(crate) fn glyph(&self, c: char) -> Option<GlyphMetrics> {
        let cp = u32::from(c);
        if cp >= 128 {
            return None;
        }
        self.inner.glyphs[cp as usize]
    }
}

impl std::fmt::Debug for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Font")
            .field("cell_pixels", &self.inner.cell_pixels)
            .finish_non_exhaustive()
    }
}

/// Text node — bitmap glyph string anchored at its container's origin.
///
/// Layout flows left-to-right, top-to-bottom. Newlines (`\n`) advance
/// `cursor.y` by `line_height` (= `cell_size` × `cell_pixels`).
#[derive(Debug, Clone)]
pub struct Text {
    /// Transform / visibility container.
    pub container: Container,
    /// String to render. Newlines start a new line.
    pub content: String,
    /// Bitmap font supplying the glyph atlas.
    pub font: Font,
    /// Text color tint.
    pub color: Color,
    /// NDC units per atlas pixel. Default `0.02` ≈ glyphs ~16% of NDC tall.
    pub cell_size: f32,
}

impl Text {
    /// Construct a text node with default white color and `cell_size = 0.02`.
    #[must_use]
    pub fn new(font: Font, content: impl Into<String>) -> Self {
        Self {
            container: Container::default(),
            content: content.into(),
            font,
            color: Color::WHITE,
            cell_size: 0.02,
        }
    }

    /// Builder: set color.
    #[must_use]
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Builder: set NDC-per-atlas-pixel scale.
    #[must_use]
    pub fn with_cell_size(mut self, size: f32) -> Self {
        self.cell_size = size;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{AppConfig, Application};

    fn boot() -> Application {
        pollster::block_on(Application::new(AppConfig::default())).expect("init")
    }

    #[test]
    fn font_has_ascii_glyphs() {
        let app = boot();
        let font = Font::bitmap_8x8(&app);
        assert!(font.glyph('A').is_some());
        assert!(font.glyph('z').is_some());
        assert!(font.glyph(' ').is_some());
        // Non-ASCII falls outside the 128-codepoint atlas.
        assert!(font.glyph('é').is_none());
    }

    #[test]
    fn text_defaults() {
        let app = boot();
        let font = Font::bitmap_8x8(&app);
        let text = Text::new(font, "Hello");
        assert_eq!(text.content, "Hello");
        assert_eq!(text.color, Color::WHITE);
        assert!((text.cell_size - 0.02).abs() < f32::EPSILON);
    }

    #[test]
    fn builders_set_fields() {
        let app = boot();
        let font = Font::bitmap_8x8(&app);
        let text = Text::new(font, "X")
            .with_color(Color::RED)
            .with_cell_size(0.05);
        assert_eq!(text.color, Color::RED);
        assert!((text.cell_size - 0.05).abs() < f32::EPSILON);
    }
}
