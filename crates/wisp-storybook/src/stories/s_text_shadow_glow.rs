//! Story: drop shadow + glow on rendered text (M-TEXT.8 / AUT-82).
//!
//! Re-uses the existing `wisp::DropShadowFilter` — alpha-extract →
//! blur → composite — applied to a text texture. Glow is just a
//! drop shadow with `offset = (0, 0)` and a bright color.

use glam::Vec2;
use wisp::application::Application;
use wisp::render::Renderer;
use wisp::text::{TextTexturePipeline, WispText, WispTextStyle};
use wisp::{Color, DropShadowFilter, RenderTexture, Sprite, Stage, Texture, WispFontWeight};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "text-shadow-glow",
        category: "Text",
        title: "Drop shadow + glow on text",
        milestone: "M-TEXT.8",
        writeup: include_str!("writeups/text_shadow_glow.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");
    let pipeline = TextTexturePipeline::new(app, wgpu::TextureFormat::Rgba8UnormSrgb);
    let root = stage.root();

    // Paper-white backdrop so the shadow is visible (drop shadows are
    // invisible against the storybook's near-black clear).
    let backdrop_rt = RenderTexture::with_format(app, 256, 256, format);
    let _ = renderer.render_stage(
        app,
        backdrop_rt.view(),
        Color::rgba(0.92, 0.93, 0.95, 1.0),
        &Stage::new(),
    );
    let backdrop_bytes = backdrop_rt.read_pixels(app);
    let backdrop_tex = Texture::from_rgba(app, 256, 256, &backdrop_bytes);
    let mut backdrop = Sprite::from_texture(backdrop_tex).with_anchor(Vec2::splat(0.5));
    backdrop.container.transform.scale = Vec2::splat(2.0);
    let _ = stage.add_child(root, backdrop);

    // Render text once into a linear-format RT (glyphon writes +y
    // down; we use the standard `scale.y = -1` flip when sampling
    // back via Sprite). Filter math wants linear, so we keep the
    // input RT in `Rgba8Unorm`.
    let text = WispText::new("Glow").with_style(
        WispTextStyle::default()
            .with_size(0.50)
            .with_weight(WispFontWeight::Bold)
            .with_color(Color::WHITE),
    );
    let text_rt_arc = pipeline.render(app, &text, 512, 384);
    let text_bytes = text_rt_arc.read_pixels(app);
    // input_rt has the text bytes baked in directly via render_stage —
    // sprite anchor at top-left, scale (2, 2) (no flip) so input_rt
    // has glyphon's +y-down orientation preserved. Filter respects
    // input orientation; final sprites apply the y-flip when sampling.
    let staging = Texture::from_rgba(app, 512, 384, &text_bytes);
    let input_rt = RenderTexture::with_format(app, 512, 384, format);
    let mut staging_stage = Stage::new();
    let mut staging_sprite = Sprite::from_texture(staging.clone()).with_anchor(Vec2::new(0.0, 0.0));
    // Negative-y flips glyphon's +y-down texture into a +y-up RT so
    // the filter math + final sprite consume the standard orientation.
    staging_sprite.container.transform.position = Vec2::new(-1.0, 1.0);
    staging_sprite.container.transform.scale = Vec2::new(2.0, -2.0);
    let _ = staging_stage.add_child(staging_stage.root(), staging_sprite);
    let _ = renderer.render_stage(app, input_rt.view(), Color::TRANSPARENT, &staging_stage);

    // ── Drop shadow (offset down/right, dark) ───────────────────
    let shadow_rt = RenderTexture::with_format(app, 512, 384, format);
    renderer.apply_filter(
        app,
        &DropShadowFilter {
            offset: Vec2::new(6.0, 6.0),
            blur: 5.0,
            color: Color::rgba(0.0, 0.0, 0.0, 0.6),
        },
        &input_rt,
        &shadow_rt,
    );
    let shadow_bytes = shadow_rt.read_pixels(app);
    let shadow_tex = Texture::from_rgba(app, 512, 384, &shadow_bytes);
    let mut shadow_sprite = Sprite::from_texture(shadow_tex).with_anchor(Vec2::splat(0.5));
    shadow_sprite.container.transform.position = Vec2::new(-0.45, 0.20);
    // Negative y-scale flips the glyphon-oriented texture upright.
    shadow_sprite.container.transform.scale = Vec2::new(0.9, 0.7);
    let _ = stage.add_child(root, shadow_sprite);

    // ── Glow (zero offset, bright color) ────────────────────────
    let glow_rt = RenderTexture::with_format(app, 512, 384, format);
    renderer.apply_filter(
        app,
        &DropShadowFilter {
            offset: Vec2::new(0.0, 0.0),
            blur: 8.0,
            color: Color::rgba(1.0, 0.80, 0.30, 1.0),
        },
        &input_rt,
        &glow_rt,
    );
    let glow_bytes = glow_rt.read_pixels(app);
    let glow_tex = Texture::from_rgba(app, 512, 384, &glow_bytes);
    let mut glow_sprite = Sprite::from_texture(glow_tex).with_anchor(Vec2::splat(0.5));
    glow_sprite.container.transform.position = Vec2::new(0.45, -0.20);
    glow_sprite.container.transform.scale = Vec2::new(0.9, 0.7);
    let _ = stage.add_child(root, glow_sprite);
}
