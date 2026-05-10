//! Story: text rendered into a `RenderTexture` and composed into the
//! scene via a `Sprite` (M-TEXT.5).

use glam::Vec2;
use wisp::application::Application;
use wisp::text::{TextTexturePipeline, WispText, WispTextStyle};
use wisp::{Color, Sprite, Stage, WispFontWeight};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "text-texture",
        category: "Scene Graph",
        title: "Text → RenderTexture → Sprite",
        milestone: "M-TEXT.5",
        writeup: include_str!("writeups/text_texture.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    // Match the storybook's target format so the sampled view + the
    // surface gamma agree.
    let pipeline = TextTexturePipeline::new(app, wgpu::TextureFormat::Rgba8UnormSrgb);

    // Hero line. Pick RT dims that fit "Text → texture" at size_ndc
    // 0.20 — ≈14 chars × 100 px ≈ 1400 px wide, ~240 px tall.
    let hero = WispText::new("Text → texture").with_style(
        WispTextStyle::default()
            .with_size(0.20)
            .with_weight(WispFontWeight::Bold)
            .with_color(Color::rgba(1.0, 0.9, 0.4, 1.0)),
    );

    let rt = pipeline.render(app, &hero, 1536, 320);
    let texture = rt.as_texture();

    // 256×256 storybook canvas. Sprite scale.y is negative so the
    // glyphon-rendered (+y-down) texture displays upright through the
    // sprite pipeline (+y-up NDC). This is the standard
    // render-target-as-texture convention.
    let mut sprite = Sprite::from_texture(texture).with_anchor(Vec2::new(0.5, 0.5));
    sprite.container.transform.position = Vec2::new(0.0, 0.3);
    sprite.container.transform.scale = Vec2::new(1.7, -0.85);
    let _ = stage.add_child(stage.root(), sprite);

    // Render a second piece into another texture to prove the cache
    // isn't tied to a single output. Different content + dims produces
    // a distinct cached entry that participates in the same batch.
    let caption = WispText::new("cached + composed").with_style(
        WispTextStyle::default()
            .with_size(0.30)
            .with_color(Color::rgba(0.55, 0.85, 1.0, 1.0)),
    );
    let caption_rt = pipeline.render(app, &caption, 768, 256);
    let caption_tex = caption_rt.as_texture();
    let mut caption_sprite = Sprite::from_texture(caption_tex).with_anchor(Vec2::new(0.5, 0.5));
    caption_sprite.container.transform.position = Vec2::new(0.0, -0.55);
    caption_sprite.container.transform.scale = Vec2::new(1.5, -0.5);
    let _ = stage.add_child(stage.root(), caption_sprite);
}
