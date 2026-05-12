//! Text render-to-texture path (M-TEXT.5 / AUT-79).
//!
//! Pairs [`FlexibleTextEngine`] (layout) and [`FlexibleTextRenderer`]
//! (rasterization) with a FIFO-bounded cache keyed on
//! `(content, style, wrap_width, dimensions)`. Static text reuses the
//! cached texture; any change to the inputs invalidates the entry.
//!
//! The texture is a [`RenderTexture`], so downstream callers can feed
//! it into the same render path as masks (M-DYN.1) and dynamic
//! textures — filter / mask / blend / export composition all become
//! sprite-pipeline routing decisions (the M-TEXT.6 work).
//!
//! Modeled on the `mask_cache` pattern in `crate::render`: same FIFO
//! eviction at `MAX_ENTRIES`, same `Arc`-shared texture ownership,
//! same `(hits, misses)` instrumentation hook.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::Arc;

use crate::application::Application;
use crate::text::flexible::FlexibleTextEngine;
use crate::text::flexible_renderer::FlexibleTextRenderer;
use crate::text::{WispFontStyle, WispText, WispTextAlign, WispTextLayout};
use crate::texture::render_texture::RenderTexture;
use glam::Vec2;

/// Maximum number of cached text textures before FIFO eviction.
///
/// 64 × 512 × 256 × 4 bytes ≈ 32 MB upper bound — small even on
/// integrated GPUs and well within the recorder's working set.
pub const MAX_ENTRIES: usize = 64;

/// Cache key for a rendered text texture.
///
/// Hashes the content + style + wrap width + output dimensions; `f32`
/// fields go through `to_bits()` so equality is exact-bit (avoids the
/// NaN-aware quirks of `std::cmp::Eq` on floats). Same canonical NaN
/// bits hash identically — fine because callers re-pass the same
/// style value across frames.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct TextTextureKey {
    content: String,
    family: Option<String>,
    size_bits: u32,
    color_bits: [u32; 4],
    line_height_bits: u32,
    letter_spacing_bits: u32,
    weight: u16,
    italic: bool,
    align: u8,
    wrap_width_bits: Option<u32>,
    width_px: u32,
    height_px: u32,
}

impl TextTextureKey {
    /// Build a key from a [`WispText`] and the requested texture
    /// dimensions.
    #[must_use]
    pub fn new(text: &WispText, width_px: u32, height_px: u32) -> Self {
        let s = text.style;
        Self {
            content: text.content.clone(),
            family: text.font_family.clone(),
            size_bits: s.size_ndc.to_bits(),
            color_bits: [
                s.color.r.to_bits(),
                s.color.g.to_bits(),
                s.color.b.to_bits(),
                s.color.a.to_bits(),
            ],
            line_height_bits: s.line_height.to_bits(),
            letter_spacing_bits: s.letter_spacing_ndc.to_bits(),
            weight: s.weight.value(),
            italic: matches!(s.style, WispFontStyle::Italic),
            align: match s.align {
                WispTextAlign::Left => 0,
                WispTextAlign::Center => 1,
                WispTextAlign::Right => 2,
            },
            wrap_width_bits: text.max_width_ndc.map(f32::to_bits),
            width_px,
            height_px,
        }
    }
}

/// FIFO-bounded cache of rendered text textures.
pub struct TextTextureCache {
    map: HashMap<TextTextureKey, Arc<RenderTexture>>,
    order: VecDeque<TextTextureKey>,
    hits: u64,
    misses: u64,
}

impl std::fmt::Debug for TextTextureCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextTextureCache")
            .field("entries", &self.map.len())
            .field("hits", &self.hits)
            .field("misses", &self.misses)
            .finish_non_exhaustive()
    }
}

impl Default for TextTextureCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TextTextureCache {
    /// Build an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Get the cached texture for `key`, generating it via `generate`
    /// (which is only called on a miss).
    pub fn get_or_insert<F>(&mut self, key: TextTextureKey, generate: F) -> Arc<RenderTexture>
    where
        F: FnOnce() -> RenderTexture,
    {
        if let Some(existing) = self.map.get(&key) {
            self.hits += 1;
            return Arc::clone(existing);
        }
        self.misses += 1;

        let rt = Arc::new(generate());
        if self.map.len() >= MAX_ENTRIES
            && let Some(oldest) = self.order.pop_front()
        {
            self.map.remove(&oldest);
        }
        self.map.insert(key.clone(), Arc::clone(&rt));
        self.order.push_back(key);
        rt
    }

    /// `(hits, misses)` since construction.
    #[must_use]
    pub fn stats(&self) -> (u64, u64) {
        (self.hits, self.misses)
    }

    /// Number of currently-resident entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True when no entries are resident.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Drop all cached entries.
    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        // Stats are sticky on purpose — they reflect lifetime activity.
    }
}

/// High-level pipeline: text → `RenderTexture`, with caching.
///
/// Owns its own [`FlexibleTextEngine`] + [`FlexibleTextRenderer`] +
/// [`TextTextureCache`]. The engine + renderer share an
/// `Arc<Mutex<FontSystem>>` (M-TEXT.3 contract).
///
/// The pipeline is **opt-in** — callers construct it explicitly. It is
/// not held by [`crate::render::Renderer`], so apps that never render
/// flexible text don't pay glyphon + cache costs.
pub struct TextTexturePipeline {
    engine: FlexibleTextEngine,
    renderer: RefCell<FlexibleTextRenderer>,
    cache: RefCell<TextTextureCache>,
    format: wgpu::TextureFormat,
}

impl std::fmt::Debug for TextTexturePipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextTexturePipeline")
            .field("format", &self.format)
            .field("cache", &self.cache.borrow())
            .finish_non_exhaustive()
    }
}

impl TextTexturePipeline {
    /// Build a pipeline with a system-fonts [`FlexibleTextEngine`].
    #[must_use]
    pub fn new(app: &Application, format: wgpu::TextureFormat) -> Self {
        let engine = FlexibleTextEngine::new();
        let renderer = FlexibleTextRenderer::new(app, format, engine.font_system_handle());
        Self {
            engine,
            renderer: RefCell::new(renderer),
            cache: RefCell::new(TextTextureCache::new()),
            format,
        }
    }

    /// Build a pipeline with an engine seeded from explicit font
    /// files. See [`FlexibleTextEngine::from_font_paths`].
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if any font path can't be opened or parsed.
    pub fn from_font_paths<P: AsRef<Path>>(
        app: &Application,
        format: wgpu::TextureFormat,
        paths: impl IntoIterator<Item = P>,
    ) -> std::io::Result<Self> {
        let engine = FlexibleTextEngine::from_font_paths(paths)?;
        let renderer = FlexibleTextRenderer::new(app, format, engine.font_system_handle());
        Ok(Self {
            engine,
            renderer: RefCell::new(renderer),
            cache: RefCell::new(TextTextureCache::new()),
            format,
        })
    }

    /// Color format the pipeline writes into (matches what was passed
    /// to [`Self::new`] / [`Self::from_font_paths`]).
    #[must_use]
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// `(hits, misses)` since construction. Useful in tests that
    /// verify the cache short-circuits on repeated lookups.
    #[must_use]
    pub fn stats(&self) -> (u64, u64) {
        self.cache.borrow().stats()
    }

    /// Number of currently-resident cache entries.
    #[must_use]
    pub fn cache_len(&self) -> usize {
        self.cache.borrow().len()
    }

    /// Drop every cached entry.
    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }

    /// Borrow the engine for direct layout queries (metrics, etc.).
    #[must_use]
    pub fn engine(&self) -> &FlexibleTextEngine {
        &self.engine
    }

    /// Render `text` into a `width_px × height_px` `RenderTexture`,
    /// or return the cached version if `text` + dimensions match an
    /// earlier call.
    ///
    /// The text is positioned with its top-left at the texture's top-left
    /// (NDC `(-1, +1)`) and rendered with [`crate::color::Color`]
    /// `WHITE` as the default per-glyph color (the actual glyph color
    /// comes from [`crate::text::WispTextStyle::color`]).
    pub fn render(
        &self,
        app: &Application,
        text: &WispText,
        width_px: u32,
        height_px: u32,
    ) -> Arc<RenderTexture> {
        let key = TextTextureKey::new(text, width_px, height_px);
        self.cache.borrow_mut().get_or_insert(key, || {
            let rt = RenderTexture::with_format(app, width_px, height_px, self.format);
            let layout = self.engine.layout_concrete(text);
            // Skip the GPU pass when the layout is empty — glyphon
            // accepts an empty TextArea but spending the encoder /
            // submit cost is wasteful for blank text.
            if layout.metrics().line_count == 0 {
                return rt;
            }
            let mut r = self.renderer.borrow_mut();
            r.set_resolution(width_px, height_px);
            r.draw(
                rt.view(),
                &[(&layout, Vec2::new(-1.0, 1.0), text.style.color)],
                /* clear = */ true,
            );
            rt
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{AppConfig, Application};
    use crate::color::Color;

    fn boot() -> Application {
        pollster::block_on(Application::new(AppConfig::default())).expect("init")
    }

    fn pipeline(app: &Application) -> TextTexturePipeline {
        TextTexturePipeline::new(app, wgpu::TextureFormat::Rgba8Unorm)
    }

    #[test]
    fn first_render_records_a_miss_and_a_cache_entry() {
        let app = boot();
        let p = pipeline(&app);
        let _ = p.render(&app, &WispText::new("hello"), 64, 32);
        assert_eq!(p.stats(), (0, 1));
        assert_eq!(p.cache_len(), 1);
    }

    #[test]
    fn second_render_with_same_inputs_is_a_cache_hit() {
        let app = boot();
        let p = pipeline(&app);
        let a = p.render(&app, &WispText::new("hello"), 64, 32);
        let b = p.render(&app, &WispText::new("hello"), 64, 32);
        assert_eq!(p.stats(), (1, 1));
        // Same Arc → same underlying GPU resource.
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn changing_content_invalidates_cache() {
        let app = boot();
        let p = pipeline(&app);
        let a = p.render(&app, &WispText::new("hello"), 64, 32);
        let b = p.render(&app, &WispText::new("world"), 64, 32);
        assert_eq!(p.stats(), (0, 2));
        assert!(!Arc::ptr_eq(&a, &b));
        assert_eq!(p.cache_len(), 2);
    }

    #[test]
    fn changing_style_invalidates_cache() {
        let app = boot();
        let p = pipeline(&app);
        let plain = WispText::new("hello");
        let bold = WispText::new("hello")
            .with_style(plain.style.with_weight(crate::text::WispFontWeight::Bold));
        let a = p.render(&app, &plain, 64, 32);
        let b = p.render(&app, &bold, 64, 32);
        assert_eq!(p.stats(), (0, 2));
        assert!(!Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn changing_color_invalidates_cache() {
        let app = boot();
        let p = pipeline(&app);
        let red = WispText::new("hello").with_style(
            crate::text::WispTextStyle::default().with_color(Color::rgba(1.0, 0.0, 0.0, 1.0)),
        );
        let green = WispText::new("hello").with_style(
            crate::text::WispTextStyle::default().with_color(Color::rgba(0.0, 1.0, 0.0, 1.0)),
        );
        let a = p.render(&app, &red, 64, 32);
        let b = p.render(&app, &green, 64, 32);
        assert!(!Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn changing_wrap_width_invalidates_cache() {
        let app = boot();
        let p = pipeline(&app);
        let unwrapped = WispText::new("a long line of text");
        let wrapped = WispText::new("a long line of text").with_wrap(0.5);
        let a = p.render(&app, &unwrapped, 64, 32);
        let b = p.render(&app, &wrapped, 64, 32);
        assert!(!Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn changing_dimensions_invalidates_cache() {
        let app = boot();
        let p = pipeline(&app);
        let a = p.render(&app, &WispText::new("hello"), 64, 32);
        let b = p.render(&app, &WispText::new("hello"), 128, 32);
        assert!(!Arc::ptr_eq(&a, &b));
        assert_eq!(p.cache_len(), 2);
    }

    #[test]
    fn changing_font_family_invalidates_cache() {
        let app = boot();
        let p = pipeline(&app);
        let a = p.render(&app, &WispText::new("hello"), 64, 32);
        let b = p.render(
            &app,
            &WispText::new("hello").with_font_family("Inter"),
            64,
            32,
        );
        assert!(!Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn cache_evicts_at_capacity() {
        let app = boot();
        let p = pipeline(&app);
        // Fill the cache + 1 to force eviction.
        for i in 0..=MAX_ENTRIES {
            // Distinct content per iteration → distinct keys.
            let _ = p.render(&app, &WispText::new(format!("entry-{i}")), 32, 32);
        }
        // After (MAX_ENTRIES + 1) misses with FIFO eviction, len caps at MAX_ENTRIES.
        assert_eq!(p.cache_len(), MAX_ENTRIES);
        let (hits, misses) = p.stats();
        assert_eq!(hits, 0);
        let expected_misses = u64::try_from(MAX_ENTRIES + 1).expect("fits");
        assert_eq!(misses, expected_misses);
    }

    #[test]
    fn clear_cache_drops_entries_and_refills_on_next_render() {
        let app = boot();
        let p = pipeline(&app);
        let _ = p.render(&app, &WispText::new("hello"), 64, 32);
        assert_eq!(p.cache_len(), 1);
        p.clear_cache();
        assert_eq!(p.cache_len(), 0);
        let _ = p.render(&app, &WispText::new("hello"), 64, 32);
        // After clear, the previously-warm entry is a miss again.
        assert_eq!(p.stats(), (0, 2));
    }

    #[test]
    fn rendered_texture_has_non_zero_glyph_pixels() {
        let app = boot();
        let p = pipeline(&app);
        let rt = p.render(&app, &WispText::new("hi"), 128, 64);
        let bytes = rt.read_pixels(&app);
        let non_zero = bytes.chunks_exact(4).filter(|p| p[3] > 0).count();
        assert!(
            non_zero > 0,
            "expected the text pipeline to paint glyph pixels"
        );
    }
}
