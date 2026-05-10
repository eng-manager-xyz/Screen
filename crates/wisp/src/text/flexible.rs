//! Flexible text backend — Cosmic Text layout (M-TEXT.2 / AUT-76).
//!
//! `FlexibleText` is the styled / wrapped / shaped text path. It uses
//! `cosmic_text` for layout (line breaking, `BiDi`, font fallback,
//! shaping) and — once M-TEXT.3 lands — `glyphon` for rasterization.
//!
//! This chunk is the **layout half**. The renderer half (`glyphon`
//! pipeline + `WispTextRenderer` impl) lands in M-TEXT.3 / AUT-77.
//! Until then, the engine produces `FlexibleTextLayout` instances that
//! carry an internal `cosmic_text::Buffer` ready to be fed to glyphon.
//!
//! # Boundary
//!
//! The trait surface ([`WispTextEngine`], [`WispTextLayout`]) does
//! **not** expose `cosmic_text::*` types — they live behind private
//! fields on [`FlexibleTextLayout`]. App / editor / project code only
//! sees `WispText` + `Box<dyn WispTextLayout>`. The renderer
//! (`glyphon`) downcasts to read the buffer.
//!
//! # Layout reference basis
//!
//! Cosmic Text works in pixels; wisp works in NDC. The engine adopts
//! a **reference height** of `REFERENCE_PX` pixels — `style.size_ndc`
//! is multiplied by this constant to derive the cosmic-text font
//! size. Per-glyph positions are converted back to NDC by dividing by
//! the same constant. The renderer (M-TEXT.3) re-scales to the actual
//! target dimensions at draw time.
//!
//! Picking 1000 px as the basis keeps numbers well within f32
//! precision, gives sub-pixel positioning headroom for `size_ndc =
//! 0.06` (= 60 px ≈ caption-y), and matches what glyphon's atlas
//! cache expects for typical desktop UIs.

use std::path::Path;
use std::sync::{Arc, Mutex};

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Style, Weight, Wrap};

use super::{WispText, WispTextAlign, WispTextEngine, WispTextLayout, WispTextMetrics};

/// Pixel basis for NDC ↔ cosmic-text px conversion. See module docs.
pub const REFERENCE_PX: f32 = 1000.0;

/// Result of [`FlexibleTextEngine::layout`] — a shaped cosmic-text
/// `Buffer` plus metrics.
///
/// The buffer is held privately so callers can't reach into the
/// cosmic-text API through this struct. The renderer (M-TEXT.3) uses
/// the crate-private `buffer()` accessor.
pub struct FlexibleTextLayout {
    /// Underlying cosmic-text buffer. Crate-public so the
    /// `FlexibleTextRenderer` (glyphon) can read it without a
    /// downcast; not exposed outside the crate.
    pub(crate) buffer: Buffer,
    metrics: WispTextMetrics,
}

impl std::fmt::Debug for FlexibleTextLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlexibleTextLayout")
            .field("metrics", &self.metrics)
            .finish_non_exhaustive()
    }
}

impl WispTextLayout for FlexibleTextLayout {
    fn metrics(&self) -> WispTextMetrics {
        self.metrics
    }
}

/// Cosmic-Text-backed layout engine.
///
/// Wraps a `FontSystem` (which loads system fonts on construction).
/// Cosmic Text's `FontSystem` is `!Sync`, so we wrap it in a `Mutex`
/// so the engine can be shared across threads — necessary for caches
/// (M-DYN.2-style) and the renderer's `&self` access pattern.
pub struct FlexibleTextEngine {
    font_system: Arc<Mutex<FontSystem>>,
}

impl std::fmt::Debug for FlexibleTextEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlexibleTextEngine").finish_non_exhaustive()
    }
}

impl Default for FlexibleTextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FlexibleTextEngine {
    /// Build the engine with a system-fonts `FontSystem`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            font_system: Arc::new(Mutex::new(FontSystem::new())),
        }
    }

    /// Build the engine with a pre-existing `FontSystem`.
    ///
    /// Useful for tests that want a deterministic font set, or for
    /// hosts that have already loaded a `FontSystem` for another
    /// purpose.
    #[must_use]
    pub fn with_font_system(font_system: FontSystem) -> Self {
        Self {
            font_system: Arc::new(Mutex::new(font_system)),
        }
    }

    /// Build the engine with a `FontSystem` seeded from a list of
    /// font file paths. No system fonts are loaded — only the
    /// supplied files are available, and family-name lookups
    /// ([`WispText::with_font_family`](super::WispText::with_font_family))
    /// resolve against this set.
    ///
    /// Used by storybook exporters and tests that want byte-identical
    /// output across hosts (CI runners have different system fonts).
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if any path can't be opened or isn't a
    /// recognized font file.
    pub fn from_font_paths<P: AsRef<Path>>(
        paths: impl IntoIterator<Item = P>,
    ) -> std::io::Result<Self> {
        let mut db = cosmic_text::fontdb::Database::new();
        for p in paths {
            db.load_font_file(p.as_ref())?;
        }
        Ok(Self::with_font_system(FontSystem::new_with_locale_and_db(
            "en-US".to_owned(),
            db,
        )))
    }

    /// Borrow the shared `FontSystem` handle. The
    /// [`FlexibleTextRenderer`](super::FlexibleTextRenderer) constructor
    /// uses this to wire layout + rasterization to the same font
    /// database (so glyph metrics agree).
    #[must_use]
    pub fn font_system_handle(&self) -> Arc<Mutex<FontSystem>> {
        Arc::clone(&self.font_system)
    }

    /// Concrete-typed layout entrypoint — preserves the
    /// `FlexibleTextLayout` type so the renderer half can read the
    /// buffer without going through `dyn` downcasting.
    #[must_use]
    pub fn layout_concrete(&self, text: &WispText) -> FlexibleTextLayout {
        let mut fs = self
            .font_system
            .lock()
            .expect("FlexibleTextEngine font_system poisoned");
        layout_flexible(&mut fs, text)
    }
}

impl WispTextEngine for FlexibleTextEngine {
    fn layout(&self, text: &WispText) -> Box<dyn WispTextLayout> {
        Box::new(self.layout_concrete(text))
    }
}

fn layout_flexible(font_system: &mut FontSystem, text: &WispText) -> FlexibleTextLayout {
    let style = text.style;
    let font_size_px = style.size_ndc * REFERENCE_PX;
    let line_height_px = font_size_px * style.line_height;
    let metrics = Metrics::new(font_size_px, line_height_px);

    let mut buffer = Buffer::new(font_system, metrics);

    let wrap_width_px = text.max_width_ndc.map(|w| w * REFERENCE_PX);
    buffer.set_wrap(
        font_system,
        if wrap_width_px.is_some() {
            Wrap::Word
        } else {
            Wrap::None
        },
    );
    let wrap_height_px = wrap_width_px.map_or(f32::INFINITY, |_| f32::INFINITY);
    buffer.set_size(font_system, wrap_width_px, Some(wrap_height_px));

    let family = text
        .font_family
        .as_deref()
        .map_or(Family::SansSerif, Family::Name);
    let attrs = Attrs::new()
        .family(family)
        .weight(Weight(style.weight.value()))
        .style(match style.style {
            super::WispFontStyle::Normal => Style::Normal,
            super::WispFontStyle::Italic => Style::Italic,
        });
    buffer.set_text(font_system, &text.content, attrs, Shaping::Advanced);

    buffer.shape_until_scroll(font_system, false);

    let mut max_width_px: f32 = 0.0;
    let mut line_count: u32 = 0;
    let mut last_baseline_px: f32 = 0.0;
    for run in buffer.layout_runs() {
        line_count += 1;
        max_width_px = max_width_px.max(run.line_w);
        last_baseline_px = run.line_top + line_height_px;
    }
    let total_height_px = if line_count == 0 {
        0.0
    } else {
        last_baseline_px
    };

    let metrics_out = WispTextMetrics {
        line_count,
        max_width_ndc: max_width_px / REFERENCE_PX,
        total_height_ndc: total_height_px / REFERENCE_PX,
        baseline_ndc: line_height_px / REFERENCE_PX,
    };

    // Alignment is layered on at render time once the line widths
    // are known relative to the wrap box; cosmic-text's per-line
    // alignment hooks land in M-TEXT.3 with the rasterization pass.
    let _ = WispTextAlign::Left;

    FlexibleTextLayout {
        buffer,
        metrics: metrics_out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{WispFontWeight, WispText, WispTextStyle};

    fn engine() -> FlexibleTextEngine {
        FlexibleTextEngine::new()
    }

    #[test]
    fn empty_string_yields_metrics_with_zero_width() {
        let eng = engine();
        let layout = eng.layout_concrete(&WispText::new(""));
        let m = layout.metrics();
        assert!(
            m.max_width_ndc.abs() < 1e-3,
            "expected ~0 width for empty content, got {}",
            m.max_width_ndc
        );
    }

    #[test]
    fn single_line_has_one_run_and_positive_width() {
        let eng = engine();
        let layout = eng.layout_concrete(&WispText::new("Hello, world!"));
        let m = layout.metrics();
        assert_eq!(m.line_count, 1);
        assert!(m.max_width_ndc > 0.0, "expected positive width");
        // Baseline ≈ size_ndc * line_height = 0.06 * 1.2 = 0.072.
        let expected = 0.06_f32 * 1.2;
        assert!(
            (m.baseline_ndc - expected).abs() < 1e-3,
            "baseline_ndc={} expected~{expected}",
            m.baseline_ndc
        );
    }

    #[test]
    fn explicit_newlines_produce_multiple_runs() {
        let eng = engine();
        let layout = eng.layout_concrete(&WispText::new("first\nsecond\nthird"));
        let m = layout.metrics();
        assert_eq!(m.line_count, 3, "expected 3 runs, got {}", m.line_count);
        // Total height ≈ line_height_ndc * line_count = 0.072 * 3 = 0.216.
        let expected = 0.06_f32 * 1.2 * 3.0;
        assert!(
            (m.total_height_ndc - expected).abs() < 1e-3,
            "total_height={} expected~{expected}",
            m.total_height_ndc
        );
    }

    #[test]
    fn word_wrap_increases_line_count_when_wrap_width_is_tight() {
        let eng = engine();
        let unwrapped = eng.layout_concrete(&WispText::new(
            "the quick brown fox jumps over the lazy dog",
        ));
        let wrapped = eng.layout_concrete(
            &WispText::new("the quick brown fox jumps over the lazy dog").with_wrap(0.20),
        );
        assert_eq!(unwrapped.metrics().line_count, 1);
        assert!(
            wrapped.metrics().line_count >= 2,
            "wrap=0.20 should have produced ≥2 lines, got {}",
            wrapped.metrics().line_count
        );
    }

    #[test]
    fn weight_and_italic_style_are_passed_through_attrs() {
        // We can't assert specific glyph metrics (depends on system fonts),
        // but we can assert layout still produces non-zero metrics for the
        // styled variant — i.e. the attrs path doesn't blow up.
        let eng = engine();
        let style = WispTextStyle::default()
            .with_weight(WispFontWeight::Bold)
            .italic();
        let layout = eng.layout_concrete(&WispText::new("Bold italic").with_style(style));
        assert_eq!(layout.metrics().line_count, 1);
        assert!(layout.metrics().max_width_ndc > 0.0);
    }

    #[test]
    fn custom_font_family_lays_out_without_panic() {
        // Family name doesn't need to resolve to a loaded face for
        // layout to succeed — cosmic-text falls back to sans-serif when
        // the name is unknown. The contract under test is "Family::Name
        // path doesn't crash and still produces metrics."
        let eng = engine();
        let layout = eng.layout_concrete(&WispText::new("hello").with_font_family("Inter"));
        assert_eq!(layout.metrics().line_count, 1);
        assert!(layout.metrics().max_width_ndc > 0.0);
    }

    #[test]
    fn engine_is_send_and_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<FlexibleTextEngine>();
        assert_sync::<FlexibleTextEngine>();
    }
}
