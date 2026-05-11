//! Story: word-wrapped caption block with rounded background
//! (M-TEXT.9 / AUT-83).
//!
//! Demonstrates `wisp::text::CaptionBlock` — a composed scene fragment
//! that wraps text inside a fixed width and pads a rounded-rect
//! background around it.

use glam::Vec2;
use wisp::application::Application;
use wisp::render::Renderer;
use wisp::text::{CaptionBlock, TextPreset, TextTexturePipeline, WispText};
use wisp::texture::render_texture::RenderTexture;
use wisp::{Color, Container, Fill, Graphics, Sprite, Stage};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "text-caption-block",
        category: "Text",
        title: "Word-wrapped caption block",
        milestone: "M-TEXT.9",
        writeup: include_str!("writeups/text_caption_block.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let pipeline = TextTexturePipeline::new(app, wgpu::TextureFormat::Rgba8UnormSrgb);
    let root = stage.root();

    // ── Subdued cool backdrop (sprite-based, so it stays behind) ──
    let backdrop_rt = {
        let renderer = Renderer::new(app, wgpu::TextureFormat::Rgba8UnormSrgb).expect("renderer");
        let rt = RenderTexture::with_format(app, 256, 256, wgpu::TextureFormat::Rgba8UnormSrgb);
        let mut bg_stage = Stage::new();
        let mut g = Graphics::new();
        g.fill(Fill::Solid(Color::rgba_u8(50, 70, 100, 255)));
        g.draw_rect(wisp::math::Rect::new(-1.0, -1.0, 2.0, 2.0));
        g.fill(Fill::Solid(Color::rgba_u8(60, 90, 130, 255)));
        g.draw_rounded_rect(wisp::math::Rect::new(-0.6, -0.3, 1.2, 0.6), 0.08);
        let _ = bg_stage.add_child(bg_stage.root(), g);
        let _ = renderer.render_stage(
            app,
            rt.view(),
            Color::rgba(0.15, 0.18, 0.22, 1.0),
            &bg_stage,
        );
        rt
    };
    let mut backdrop = Sprite::from_texture(backdrop_rt.as_texture());
    backdrop.container.transform.position = Vec2::new(-1.0, -1.0);
    backdrop.container.transform.scale = Vec2::new(2.0, 2.0);
    let _ = stage.add_child(root, backdrop);

    // ── Caption block #1: short, single-line ──────────────────
    let short = CaptionBlock::from_text(
        WispText::new("Recording").with_style(TextPreset::Caption.style().with_color(Color::WHITE)),
    )
    .with_width(0.85)
    .with_padding(0.05)
    .with_radius(0.06)
    .with_background(Color::rgba_u8(20, 25, 30, 220));

    let short_layout = short.layout(app, &pipeline);
    let short_position = Vec2::new(-0.425, 0.65);
    let mut short_container = Container::new();
    short_container.transform.position = short_position;
    let short_id = stage
        .add_child(root, short_container)
        .expect("short container");
    let _ = stage.add_child(short_id, short_layout.background);
    let _ = stage.add_child(short_id, short_layout.text_sprite);

    // ── Caption block #2: multi-line wrap ─────────────────────
    let long = CaptionBlock::from_text(
        WispText::new("This longer caption wraps inside the fixed width and pads cleanly.")
            .with_style(TextPreset::Caption.style().with_color(Color::WHITE)),
    )
    .with_width(0.85)
    .with_padding(0.05)
    .with_radius(0.06)
    .with_background(Color::rgba_u8(20, 25, 30, 220));

    let long_layout = long.layout(app, &pipeline);
    let mut long_container = Container::new();
    long_container.transform.position =
        Vec2::new(-0.425, short_position.y - short_layout.height_ndc - 0.06);
    let long_id = stage
        .add_child(root, long_container)
        .expect("long container");
    let _ = stage.add_child(long_id, long_layout.background);
    let _ = stage.add_child(long_id, long_layout.text_sprite);
}
