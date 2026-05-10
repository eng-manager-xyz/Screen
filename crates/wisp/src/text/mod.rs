//! Wisp text abstraction + backend boundary (M-TEXT.1 / AUT-75).
//!
//! Backend modules live under this one — see [`atlas`] for the
//! bitmap-font path (M-TEXT.4 / AUT-78). M-TEXT.2/.3 will add a
//! `flexible` module wrapping `cosmic_text` + `glyphon`.
//!
//! Wisp owns the text data model. App, editor, and project state never
//! see `cosmic_text::*` or `glyphon::*` types — they see [`WispText`],
//! [`WispTextStyle`], [`WispTextLayout`], and a few related value types.
//! Backends ([`WispTextEngine`] + [`WispTextRenderer`]) plug in behind
//! this surface; today the only backend is the M0.15 bitmap path
//! (preserved as `AtlasText` in M-TEXT.4 / AUT-78). M-TEXT.2/.3 add a
//! Cosmic Text + Glyphon `FlexibleText` backend.
//!
//! The boundary is what makes "improve text rendering" possible without
//! a project-format breaking change.
//!
//! ## Type relationships
//!
//! ```text
//!   WispText { content, style, position }
//!         │
//!         │ engine.layout(text)
//!         ▼
//!   Box<dyn WispTextLayout>           ── line-broken, per-glyph data
//!         │ metrics()                       (engine-specific concrete type)
//!         │
//!         ▼
//!   renderer.draw(layout, transform)   ── GPU side
//! ```
//!
//! For most users [`WispText`] is the only type they construct directly.
//! Backends are selected through whichever method on
//! [`crate::render::Renderer`] consumes the text (e.g. `apply_atlas_text`
//! today; `apply_flexible_text` after M-TEXT.3).

use glam::Vec2;

use crate::color::Color;

pub mod atlas;
pub mod flexible;
pub mod flexible_renderer;

pub use atlas::{AtlasGlyphInstance, AtlasTextEngine, AtlasTextLayout};
pub use flexible::{FlexibleTextEngine, FlexibleTextLayout};
pub use flexible_renderer::FlexibleTextRenderer;

/// Font weight on a 100..=900 scale matching CSS / OpenType.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum WispFontWeight {
    /// 100.
    Thin,
    /// 300.
    Light,
    /// 400. Default.
    #[default]
    Regular,
    /// 500.
    Medium,
    /// 700.
    Bold,
    /// 900.
    Black,
    /// Custom numeric weight, clamped to `[100, 900]`.
    Custom(u16),
}

impl WispFontWeight {
    /// Numeric weight value (CSS-style integer, 100..=900).
    #[must_use]
    pub fn value(self) -> u16 {
        match self {
            Self::Thin => 100,
            Self::Light => 300,
            Self::Regular => 400,
            Self::Medium => 500,
            Self::Bold => 700,
            Self::Black => 900,
            Self::Custom(v) => v.clamp(100, 900),
        }
    }
}

/// Italic / oblique state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WispFontStyle {
    /// Upright. Default.
    #[default]
    Normal,
    /// Italic.
    Italic,
}

/// Horizontal alignment of laid-out lines within their box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WispTextAlign {
    /// Default.
    #[default]
    Left,
    /// Center.
    Center,
    /// Right.
    Right,
}

/// Font reference. Opaque identifier — the backend knows how to resolve
/// it. Atlas backend treats it as a slot id; Cosmic Text backend
/// treats it as a `Family + Weight + Style` query.
///
/// `Default` returns the renderer's "default font" (whatever the active
/// backend exposes). Most callers should use [`WispFontHandle::default`]
/// unless they're explicitly fan-outing across multiple fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct WispFontHandle(u32);

impl WispFontHandle {
    /// Construct from a backend-supplied numeric id.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Underlying backend id.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.0
    }
}

/// Style applied to a `WispText`. Pure data — no third-party types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WispTextStyle {
    /// Font selection.
    pub font: WispFontHandle,
    /// Size in NDC units (e.g. 0.06 ≈ 6% of canvas height).
    pub size_ndc: f32,
    /// Solid fill color.
    pub color: Color,
    /// Line height as a multiple of `size_ndc`. `1.2` is comfortable
    /// for body copy; `1.0` is tight.
    pub line_height: f32,
    /// Letter spacing in NDC units (positive = wider).
    pub letter_spacing_ndc: f32,
    /// Font weight.
    pub weight: WispFontWeight,
    /// Italic / normal.
    pub style: WispFontStyle,
    /// Horizontal alignment within the layout box.
    pub align: WispTextAlign,
}

impl Default for WispTextStyle {
    fn default() -> Self {
        Self {
            font: WispFontHandle::default(),
            size_ndc: 0.06,
            color: Color::WHITE,
            line_height: 1.2,
            letter_spacing_ndc: 0.0,
            weight: WispFontWeight::Regular,
            style: WispFontStyle::Normal,
            align: WispTextAlign::Left,
        }
    }
}

impl WispTextStyle {
    /// Builder — set the font.
    #[must_use]
    pub fn with_font(mut self, font: WispFontHandle) -> Self {
        self.font = font;
        self
    }

    /// Builder — set the size in NDC units.
    #[must_use]
    pub fn with_size(mut self, size_ndc: f32) -> Self {
        self.size_ndc = size_ndc;
        self
    }

    /// Builder — set the fill color.
    #[must_use]
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Builder — set the weight.
    #[must_use]
    pub fn with_weight(mut self, weight: WispFontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Builder — set italic.
    #[must_use]
    pub fn italic(mut self) -> Self {
        self.style = WispFontStyle::Italic;
        self
    }

    /// Builder — set alignment.
    #[must_use]
    pub fn with_align(mut self, align: WispTextAlign) -> Self {
        self.align = align;
        self
    }
}

/// Layout-time metrics for a piece of text — output of
/// [`WispTextEngine::layout`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WispTextMetrics {
    /// Number of layout lines (1 for non-wrapped text).
    pub line_count: u32,
    /// Maximum line width in NDC units.
    pub max_width_ndc: f32,
    /// Total height (`line_count` × `line_height_ndc`) in NDC units.
    pub total_height_ndc: f32,
    /// Baseline of the FIRST line measured from `position.y`, NDC
    /// units (positive moves down toward the bottom of the canvas).
    pub baseline_ndc: f32,
}

/// User-facing text primitive. Owns content + style + position; the
/// engine + renderer turn it into pixels.
///
/// `position` is the top-left of the layout box in NDC. `max_width_ndc`
/// (when `Some`) wraps the content; `None` lays out a single line.
#[derive(Debug, Clone, PartialEq)]
pub struct WispText {
    /// String to render.
    pub content: String,
    /// Style.
    pub style: WispTextStyle,
    /// Top-left of the layout box in NDC.
    pub position: Vec2,
    /// Optional wrap width (NDC). `None` = single line.
    pub max_width_ndc: Option<f32>,
    /// Optional family-name override. `None` falls back to the
    /// backend's default sans-serif family. Backends that match by
    /// CSS-style family names (`FlexibleText` / cosmic-text) honor
    /// this; `AtlasText` ignores it (single bitmap face).
    pub font_family: Option<String>,
}

impl WispText {
    /// Convenience constructor with default style.
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: WispTextStyle::default(),
            position: Vec2::ZERO,
            max_width_ndc: None,
            font_family: None,
        }
    }

    /// Builder — set the style.
    #[must_use]
    pub fn with_style(mut self, style: WispTextStyle) -> Self {
        self.style = style;
        self
    }

    /// Builder — set the position.
    #[must_use]
    pub fn with_position(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    /// Builder — enable word-wrapping at a given NDC max width.
    #[must_use]
    pub fn with_wrap(mut self, max_width_ndc: f32) -> Self {
        self.max_width_ndc = Some(max_width_ndc);
        self
    }

    /// Builder — override the font family by CSS-style family name.
    ///
    /// Honored by `FlexibleText` (cosmic-text); ignored by
    /// `AtlasText` (single bitmap face). `None` (the default) falls
    /// back to the backend's default sans-serif.
    #[must_use]
    pub fn with_font_family(mut self, family: impl Into<String>) -> Self {
        self.font_family = Some(family.into());
        self
    }
}

/// A laid-out piece of text — ready to be rendered.
///
/// Backends return their own concrete layout type (e.g. an
/// `AtlasLayout` of glyph instances, or a Cosmic Text `Buffer`),
/// hidden behind this trait. The renderer reads `metrics()` for
/// composition decisions (sizing the RT in M-TEXT.5, fitting captions
/// in M-TEXT.9) and downcasts to the backend type via the
/// engine/renderer pair.
pub trait WispTextLayout: std::fmt::Debug + Send + Sync {
    /// Per-layout metrics.
    fn metrics(&self) -> WispTextMetrics;
}

/// Text-layout engine — turns a [`WispText`] into a backend-specific
/// [`WispTextLayout`] implementor.
pub trait WispTextEngine {
    /// Lay out `text`. Backend returns a `Box<dyn WispTextLayout>`
    /// whose concrete type is recognized by the matching renderer.
    fn layout(&self, text: &WispText) -> Box<dyn WispTextLayout>;
}

/// Text renderer — consumes a layout and emits GPU draw calls.
///
/// `target_width_ndc` and `target_height_ndc` describe the destination
/// surface so the renderer can convert NDC sizes back into pixels for
/// glyph rasterization (matters for `FlexibleText` — atlas text is
/// already-rasterized).
pub trait WispTextRenderer {
    /// Draw `layout` at `text.position` (NDC). Implementations are
    /// expected to short-circuit no-ops gracefully (empty content,
    /// out-of-view position, etc.).
    fn draw(&self, layout: &dyn WispTextLayout, text: &WispText);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weight_value_is_clamped_for_custom() {
        assert_eq!(WispFontWeight::Custom(0).value(), 100);
        assert_eq!(WispFontWeight::Custom(2000).value(), 900);
        assert_eq!(WispFontWeight::Custom(550).value(), 550);
    }

    #[test]
    fn weight_named_values_match_css_scale() {
        assert_eq!(WispFontWeight::Thin.value(), 100);
        assert_eq!(WispFontWeight::Regular.value(), 400);
        assert_eq!(WispFontWeight::Bold.value(), 700);
        assert_eq!(WispFontWeight::Black.value(), 900);
    }

    #[test]
    fn style_default_is_regular_left_white_normal() {
        let s = WispTextStyle::default();
        assert_eq!(s.weight, WispFontWeight::Regular);
        assert_eq!(s.style, WispFontStyle::Normal);
        assert_eq!(s.align, WispTextAlign::Left);
        assert!((s.size_ndc - 0.06).abs() < f32::EPSILON);
        assert!((s.line_height - 1.2).abs() < f32::EPSILON);
    }

    #[test]
    fn style_builder_chains() {
        let s = WispTextStyle::default()
            .with_size(0.1)
            .with_color(Color::rgba(1.0, 0.0, 0.0, 1.0))
            .with_weight(WispFontWeight::Bold)
            .italic()
            .with_align(WispTextAlign::Center);
        assert!((s.size_ndc - 0.1).abs() < f32::EPSILON);
        assert_eq!(s.weight, WispFontWeight::Bold);
        assert_eq!(s.style, WispFontStyle::Italic);
        assert_eq!(s.align, WispTextAlign::Center);
    }

    #[test]
    fn wisp_text_builder_chains() {
        let t = WispText::new("Hello world")
            .with_position(Vec2::new(-0.5, 0.2))
            .with_wrap(0.8);
        assert_eq!(t.content, "Hello world");
        assert_eq!(t.position, Vec2::new(-0.5, 0.2));
        assert_eq!(t.max_width_ndc, Some(0.8));
        assert!(t.font_family.is_none());
    }

    #[test]
    fn with_font_family_sets_field() {
        let t = WispText::new("hi").with_font_family("Inter");
        assert_eq!(t.font_family.as_deref(), Some("Inter"));
        let t2 = WispText::new("hi").with_font_family("JetBrains Mono");
        assert_eq!(t2.font_family.as_deref(), Some("JetBrains Mono"));
    }
}
