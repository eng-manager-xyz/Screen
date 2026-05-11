//! Story: stroked text — light glyphs with a dark outline over a
//! busy / colorful background (M-TEXT.7 / AUT-81).
//!
//! Demonstrates the M-TEXT.7 stroke API: a `TextTexturePipeline` renders
//! the glyphs once, then `stroked_text_sprites` stamps the texture
//! eight times in stroke color around a central fill stamp. The text
//! stays readable over the chaotic backdrop without any shader
//! modification.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::text::{
    StrokedTextLayer, TextTexturePipeline, WispText, WispTextStyle, stroked_text_sprites,
};
use wisp::texture::render_texture::RenderTexture;
use wisp::{Color, Container, Fill, Graphics, Sprite, Stage, WispFontWeight};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "text-stroke",
        category: "Text",
        title: "Stroked text on busy backdrop",
        milestone: "M-TEXT.7",
        writeup: include_str!("writeups/text_stroke.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let root = stage.root();

    // ── Busy backdrop ──────────────────────────────────────────
    // Pre-render to a RenderTexture so the backdrop ships through the
    // sprite pipeline (and stays UNDER the text sprites). Graphics
    // primitives paint AFTER sprites in `render_stage`, so a raw
    // Graphics backdrop would overpaint the text. See CLAUDE.md
    // "Renderer batching / draw order".
    let backdrop_rt = {
        let renderer = Renderer::new(app, wgpu::TextureFormat::Rgba8UnormSrgb).expect("renderer");
        let rt = RenderTexture::with_format(app, 256, 256, wgpu::TextureFormat::Rgba8UnormSrgb);
        let mut bg_stage = Stage::new();
        let mut bg = Graphics::new();
        bg.fill(Fill::Solid(Color::rgba_u8(232, 60, 100, 255))); // pink
        bg.draw_rect(Rect::new(-1.0, -1.0, 1.0, 2.0));
        bg.fill(Fill::Solid(Color::rgba_u8(255, 220, 60, 255))); // yellow
        bg.draw_rect(Rect::new(0.0, -1.0, 1.0, 2.0));
        bg.fill(Fill::Solid(Color::rgba_u8(40, 180, 220, 255))); // cyan strip
        bg.draw_rounded_rect(Rect::new(-0.85, -0.7, 1.7, 0.45), 0.05);
        bg.fill(Fill::Solid(Color::rgba_u8(30, 30, 40, 255))); // dark strip
        bg.draw_rounded_rect(Rect::new(-0.85, 0.15, 1.7, 0.45), 0.05);
        let _ = bg_stage.add_child(bg_stage.root(), bg);
        let _ = renderer.render_stage(app, rt.view(), Color::rgba(0.1, 0.1, 0.12, 1.0), &bg_stage);
        rt
    };
    let mut backdrop = Sprite::from_texture(backdrop_rt.as_texture());
    backdrop.container.transform.position = Vec2::new(-1.0, -1.0);
    backdrop.container.transform.scale = Vec2::new(2.0, 2.0);
    let _ = stage.add_child(root, backdrop);

    // ── Rendered text texture ──────────────────────────────────
    let pipeline = TextTexturePipeline::new(app, wgpu::TextureFormat::Rgba8UnormSrgb);
    let text = WispText::new("READ ME").with_style(
        WispTextStyle::default()
            .with_size(0.22)
            .with_weight(WispFontWeight::Bold)
            .with_color(Color::WHITE),
    );
    let rt = pipeline.render(app, &text, 1024, 256);

    // ── Stroked + centered (the "with stroke" demo) ────────────
    let mut layer_container = Container::new();
    layer_container.transform.position = Vec2::new(0.0, 0.30);
    layer_container.transform.scale = Vec2::new(1.6, -0.40);
    let layer_id = stage
        .add_child(root, layer_container)
        .expect("attach container");

    let layer = StrokedTextLayer {
        fill: Color::WHITE,
        stroke: Color::rgba_u8(10, 10, 18, 255),
        stroke_width_ndc: 0.04,
    };
    for sprite in stroked_text_sprites(&rt, &layer) {
        let _ = stage.add_child(layer_id, sprite);
    }

    // ── No-stroke control below — same content, no outline ────
    let control = WispText::new("no stroke").with_style(
        WispTextStyle::default()
            .with_size(0.22)
            .with_weight(WispFontWeight::Bold)
            .with_color(Color::WHITE),
    );
    let ctl_rt = pipeline.render(app, &control, 1024, 256);
    let mut ctl_sprite = Sprite::from_texture(ctl_rt.as_texture()).with_anchor(Vec2::splat(0.5));
    ctl_sprite.container.transform.position = Vec2::new(0.0, -0.40);
    ctl_sprite.container.transform.scale = Vec2::new(1.6, -0.40);
    let _ = stage.add_child(root, ctl_sprite);
}
