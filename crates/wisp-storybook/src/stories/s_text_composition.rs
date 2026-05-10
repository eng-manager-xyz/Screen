//! Story: text composed with filters + non-Normal blend modes
//! (M-TEXT.6 / AUT-80).

use glam::Vec2;
use wisp::application::Application;
use wisp::blend::BlendMode;
use wisp::filter::ColorMatrixFilter;
use wisp::render::Renderer;
use wisp::text::{TextTexturePipeline, WispText, WispTextStyle};
use wisp::texture::render_texture::RenderTexture;
use wisp::{Color, Sprite, Stage, WispFontWeight};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "text-composition",
        category: "Scene Graph",
        title: "Text + filter + blend composition",
        milestone: "M-TEXT.6",
        writeup: include_str!("writeups/text_composition.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let pipeline = TextTexturePipeline::new(app, wgpu::TextureFormat::Rgba8UnormSrgb);

    // Backdrop sprite — a warm gradient-y RT we draw into via a stage
    // render. Demonstrates blend modes need something to blend against.
    let backdrop_rt = {
        let renderer = Renderer::new(app, wgpu::TextureFormat::Rgba8UnormSrgb).expect("renderer");
        let rt = RenderTexture::with_format(app, 256, 256, wgpu::TextureFormat::Rgba8UnormSrgb);
        let empty = Stage::new();
        let _ = renderer.render_stage(app, rt.view(), Color::rgba(0.85, 0.35, 0.25, 1.0), &empty);
        rt
    };
    let mut backdrop = Sprite::from_texture(backdrop_rt.as_texture());
    backdrop.container.transform.position = Vec2::new(-1.0, -1.0);
    backdrop.container.transform.scale = Vec2::new(2.0, 2.0);
    let _ = stage.add_child(stage.root(), backdrop);

    // Top half: white text at Normal blend — the baseline.
    let normal_text = WispText::new("Normal").with_style(
        WispTextStyle::default()
            .with_size(0.35)
            .with_weight(WispFontWeight::Bold)
            .with_color(Color::WHITE),
    );
    let normal_rt = pipeline.render(app, &normal_text, 512, 192);
    let mut normal_sprite =
        Sprite::from_texture(normal_rt.as_texture()).with_anchor(Vec2::new(0.5, 0.5));
    normal_sprite.container.transform.position = Vec2::new(0.0, 0.5);
    normal_sprite.container.transform.scale = Vec2::new(1.4, -0.45);
    normal_sprite.container.blend_mode = BlendMode::Normal;
    let _ = stage.add_child(stage.root(), normal_sprite);

    // Middle: text at Subtract — `dst - src` clamps to a dark stamp
    // where the glyph alpha is high. Picked over Multiply because
    // Multiply with white text against a warm backdrop gives back
    // the backdrop color (invisible).
    let sub_text = WispText::new("Subtract").with_style(
        WispTextStyle::default()
            .with_size(0.35)
            .with_weight(WispFontWeight::Bold)
            .with_color(Color::WHITE),
    );
    let sub_rt = pipeline.render(app, &sub_text, 512, 192);
    let mut sub_sprite = Sprite::from_texture(sub_rt.as_texture()).with_anchor(Vec2::new(0.5, 0.5));
    sub_sprite.container.transform.position = Vec2::new(0.0, 0.0);
    sub_sprite.container.transform.scale = Vec2::new(1.4, -0.45);
    sub_sprite.container.blend_mode = BlendMode::Subtract;
    let _ = stage.add_child(stage.root(), sub_sprite);

    // Bottom: text → grayscale color-matrix → sprite. Demonstrates a
    // filter chain feeding the sprite pipeline. The text is colored
    // orange in the source but the filter erases the hue.
    let orange_text = WispText::new("Filtered").with_style(
        WispTextStyle::default()
            .with_size(0.35)
            .with_weight(WispFontWeight::Bold)
            .with_color(Color::rgba(1.0, 0.4, 0.1, 1.0)),
    );
    let source = pipeline.render(app, &orange_text, 512, 192);
    let filtered = RenderTexture::with_format(app, 512, 192, wgpu::TextureFormat::Rgba8UnormSrgb);
    {
        let renderer = Renderer::new(app, wgpu::TextureFormat::Rgba8UnormSrgb).expect("renderer");
        renderer.apply_filter(app, &ColorMatrixFilter::grayscale(), &source, &filtered);
    }
    let mut filtered_sprite =
        Sprite::from_texture(filtered.as_texture()).with_anchor(Vec2::new(0.5, 0.5));
    filtered_sprite.container.transform.position = Vec2::new(0.0, -0.5);
    filtered_sprite.container.transform.scale = Vec2::new(1.4, -0.45);
    let _ = stage.add_child(stage.root(), filtered_sprite);
}
