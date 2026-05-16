//! Inter-font text rendering for chart labels.
//!
//! Charts historically emitted text as `wisp::scene::Text` nodes
//! backed by the chunky 8×8 bitmap font; that font is unreadable at
//! the axis-label sizes book chapters need. This module wraps wisp's
//! flexible-text path ([`TextTexturePipeline`] →
//! [`RenderTexture::as_texture`] → [`Sprite`]) and is what every
//! axis-renderer / legend / KPI / gauge label flows through today.
//!
//! ## Why a single helper
//!
//! Each chart family used to do its own bitmap-text plumbing —
//! computing per-label NDC sizes, building `Text` nodes, positioning
//! them. The flex path needs more boilerplate (allocate RT → render →
//! convert to texture → build sprite → scale to NDC) so consolidating
//! the boilerplate here keeps every chart's text path one line.
//!
//! ## Sizing + caching
//!
//! Each call allocates a `RenderTexture` sized to fit `content` at
//! `size_px`; the over-allocation is generous (60 % of `size_px` per
//! char + 60 % padding) so cosmic-text's metrics always fit. The
//! pipeline caches by `(content, w, h)` — same call site, same string
//! at the same size = cache hit on subsequent frames.
//!
//! ## Coordinate system
//!
//! Inputs are pixel-space (top-left origin). The returned [`Sprite`]
//! is positioned in scene NDC with a negative-Y scale that flips
//! cosmic-text's top-down render so glyphs land right-side-up.

use glam::Vec2;
use wisp::Sprite;
use wisp::application::Application;
use wisp::text::{TextTexturePipeline, WispFontWeight, WispText, WispTextStyle};

use crate::color::Color as ChartColor;

/// Bundled Inter font bytes — regular + bold weights. Embedded
/// via `include_bytes!` so the pipeline works identically on native
/// and `wasm32-unknown-unknown` targets. Both files originate from
/// the [Inter project](https://github.com/rsms/inter) under the SIL
/// Open Font License 1.1.
const INTER_REGULAR_TTF: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_BOLD_TTF: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");

/// Build a [`TextTexturePipeline`] seeded with the bundled Inter
/// font set (Regular + Bold). Every chart that emits text via
/// [`build_text_sprite`] should route through a pipeline built this
/// way — the family lookup in `WispText::with_font_family("Inter")`
/// requires Inter to be in the font database.
#[must_use]
pub fn pipeline_with_inter(
    app: &wisp::application::Application,
    format: wisp::wgpu::TextureFormat,
) -> TextTexturePipeline {
    TextTexturePipeline::from_font_bytes(
        app,
        format,
        [INTER_REGULAR_TTF.to_vec(), INTER_BOLD_TTF.to_vec()],
    )
}

/// The single font family every chart renders text in. Mirrors the
/// `crates/wisp-storybook/assets/fonts/Inter-*.ttf` files; if the
/// flexible engine can't resolve "Inter" it falls back to whatever
/// the backend treats as the default sans (cosmic-text picks the
/// closest installed family).
pub const INTER_FONT_FAMILY: &str = "Inter";

/// Where the sprite's reference point lives on its quad. Maps to the
/// `anchor: Vec2` field on [`Sprite`] (`0.0` = top/left, `1.0` =
/// bottom/right within the texture's local rect).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAnchor {
    /// Top-left of the rendered glyph box at the position.
    TopLeft,
    /// Top edge / X-centred — typical for X-axis tick labels below
    /// the axis line.
    TopCentre,
    /// Top edge / right-aligned.
    TopRight,
    /// Y-centred / left edge.
    MiddleLeft,
    /// Geometric centre of the sprite quad.
    MiddleCentre,
    /// Y-centred / right edge — typical for Y-axis tick labels left
    /// of the axis line.
    MiddleRight,
}

impl TextAnchor {
    /// Local-rect coordinates (texture-local, top-left origin).
    #[must_use]
    fn to_vec2(self) -> Vec2 {
        match self {
            Self::TopLeft => Vec2::new(0.0, 0.0),
            Self::TopCentre => Vec2::new(0.5, 0.0),
            Self::TopRight => Vec2::new(1.0, 0.0),
            Self::MiddleLeft => Vec2::new(0.0, 0.5),
            Self::MiddleCentre => Vec2::splat(0.5),
            Self::MiddleRight => Vec2::new(1.0, 0.5),
        }
    }
}

/// One chart-text label spec — the inputs every chart family already
/// has at the point it needs to place text (string, anchor pixel,
/// font size, color, alignment).
#[derive(Clone, Debug)]
pub struct ChartTextSpec {
    /// Display string.
    pub content: String,
    /// Anchor pixel coordinate in viewport-pixel space (top-left
    /// origin). The sprite's [`TextAnchor`] decides which corner of
    /// the glyph box aligns to this point.
    pub anchor_px: Vec2,
    /// Font size in display pixels.
    pub size_px: f32,
    /// Fill color.
    pub color: ChartColor,
    /// Which corner of the sprite quad to align to `anchor_px`.
    pub anchor: TextAnchor,
    /// Font weight. Axis / legend labels are `Regular`; KPI / gauge
    /// big numbers pass `Bold`.
    pub weight: WispFontWeight,
}

impl ChartTextSpec {
    /// Convenience constructor for axis / legend labels — Regular
    /// weight, the chart's `text_muted` colour, top-centre anchor.
    #[must_use]
    pub fn axis_tick(
        content: impl Into<String>,
        anchor_px: Vec2,
        size_px: f32,
        color: ChartColor,
    ) -> Self {
        Self {
            content: content.into(),
            anchor_px,
            size_px,
            color,
            anchor: TextAnchor::TopCentre,
            weight: WispFontWeight::Regular,
        }
    }
}

/// Build a single text sprite for `spec` against `pipeline` and add
/// it to the caller's sprite list. Returns `None` only when the
/// engine produces an empty layout for `spec.content` (e.g. all
/// whitespace) — every legitimate label produces `Some`.
#[must_use]
pub fn build_text_sprite(
    app: &Application,
    pipeline: &TextTexturePipeline,
    viewport_px: Vec2,
    spec: &ChartTextSpec,
) -> Sprite {
    // RT allocation. The flexible renderer scales glyph output by
    // `rt_height_px / REFERENCE_PX` (= 1000), so the `size_ndc` we
    // pass into `WispTextStyle::with_size` must be expressed as the
    // *fraction of the RT height* the glyph cap should occupy. We
    // derive that below, after picking the RT dims.
    let char_count = spec.content.chars().count();
    #[allow(
        clippy::cast_precision_loss,
        reason = "char_count bounded by `content` length (< 64 in practice)"
    )]
    let char_count_f = char_count as f32;
    // Allocate generously — Inter glyph widths run 0.45–0.85 ems
    // depending on character. ~0.7 em per char + a 1 em pad keeps
    // every short label fitting in its RT with margin to spare.
    let rt_width_px = ((char_count_f * 0.7 + 1.0) * spec.size_px).ceil().max(16.0);
    let rt_height_px = (spec.size_px * 1.6).ceil().max(16.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "RT dims clamped to [16, viewport_px]; safe for u32."
    )]
    let (rt_width_u, rt_height_u) = (rt_width_px as u32, rt_height_px as u32);

    let size_ndc = spec.size_px / rt_height_px;
    let style = WispTextStyle::default()
        .with_size(size_ndc)
        .with_color(wisp::Color::rgba(
            spec.color.r,
            spec.color.g,
            spec.color.b,
            spec.color.a,
        ))
        .with_weight(spec.weight);
    let text = WispText::new(spec.content.clone())
        .with_style(style)
        .with_font_family(INTER_FONT_FAMILY);

    let rt = pipeline.render(app, &text, rt_width_u, rt_height_u);
    let texture = rt.as_texture();

    // Scale + flip: cosmic-text renders top-down into the RT; the
    // Sprite quad samples bottom-up in scene NDC, so the `-Y`
    // multiplier on `scale.y` makes glyphs land right-side-up.
    let scale = Vec2::new(
        rt_width_px / viewport_px.x * 2.0,
        -rt_height_px / viewport_px.y * 2.0,
    );

    // Anchor pixel coord → NDC. The sprite's `anchor` field then
    // shifts the local rect so this NDC point coincides with the
    // chosen corner / midpoint of the glyph box.
    let anchor_ndc = Vec2::new(
        spec.anchor_px.x / viewport_px.x * 2.0 - 1.0,
        1.0 - spec.anchor_px.y / viewport_px.y * 2.0,
    );

    let mut sprite = Sprite::from_texture(texture).with_anchor(spec.anchor.to_vec2());
    sprite.container.transform.position = anchor_ndc;
    sprite.container.transform.scale = scale;
    sprite
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use pollster::block_on;
    use wisp::application::AppConfig;

    fn boot() -> (Application, TextTexturePipeline) {
        let app = block_on(Application::new(AppConfig::default())).expect("Application::new");
        let pipeline = TextTexturePipeline::new(&app, wgpu::TextureFormat::Rgba8UnormSrgb);
        (app, pipeline)
    }

    #[test]
    fn build_text_sprite_positions_top_centre() {
        let (app, pipeline) = boot();
        let viewport = Vec2::new(800.0, 400.0);
        let spec = ChartTextSpec::axis_tick("100", Vec2::new(400.0, 50.0), 14.0, Color::BLACK);
        let sprite = build_text_sprite(&app, &pipeline, viewport, &spec);
        // Anchor at viewport centre x = NDC 0.0; top y at 50/400*2 - 1 = -0.75, but pixel→NDC
        // inverts: 1 - 50/400*2 = 1 - 0.25 = 0.75.
        assert!(
            (sprite.container.transform.position.x - 0.0).abs() < 1e-5,
            "x = {}",
            sprite.container.transform.position.x
        );
        assert!(
            (sprite.container.transform.position.y - 0.75).abs() < 1e-5,
            "y = {}",
            sprite.container.transform.position.y
        );
        assert_eq!(sprite.anchor, Vec2::new(0.5, 0.0));
    }

    #[test]
    fn pipeline_with_inter_actually_rasterises_glyphs() {
        // Smoke test: render one short Inter label and verify the
        // resulting `RenderTexture` contains *some* opaque pixels.
        // Catches the size-ndc / REFERENCE_PX trap (text rendered at
        // 0.3 px) before it propagates to every chart's hero snapshot.
        use wisp::text::WispText;
        let app = block_on(Application::new(AppConfig::default())).expect("Application::new");
        let pipeline = pipeline_with_inter(&app, wgpu::TextureFormat::Rgba8UnormSrgb);
        let text = WispText::new("12.34")
            .with_style(
                WispTextStyle::default()
                    .with_size(0.7)
                    .with_color(wisp::Color::WHITE),
            )
            .with_font_family(INTER_FONT_FAMILY);
        let rt = pipeline.render(&app, &text, 96, 32);
        let bytes = rt.read_pixels(&app);
        let alpha_count = bytes.iter().skip(3).step_by(4).filter(|&&a| a > 0).count();
        assert!(
            alpha_count > 10,
            "Inter rasteriser produced an empty RT — text won't appear in charts. Alpha hits: {alpha_count}"
        );
    }

    #[test]
    fn build_text_sprite_flips_y_scale() {
        let (app, pipeline) = boot();
        let viewport = Vec2::new(800.0, 400.0);
        let spec = ChartTextSpec::axis_tick("12.34", Vec2::ZERO, 14.0, Color::BLACK);
        let sprite = build_text_sprite(&app, &pipeline, viewport, &spec);
        // Y-scale must be negative to flip cosmic-text's top-down render.
        assert!(sprite.container.transform.scale.y < 0.0);
        // X-scale stays positive.
        assert!(sprite.container.transform.scale.x > 0.0);
    }
}
