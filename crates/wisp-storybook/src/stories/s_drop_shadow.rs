//! Story: `DropShadowFilter` — rounded rect with a soft shadow (M0.17).

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{
    Color, DropShadowFilter, Fill, Graphics, RenderTexture, Sprite, Stage, Stroke, Texture,
};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "filter-drop-shadow",
        category: "Filters",
        title: "Drop shadow",
        milestone: "M0.17",
        writeup: include_str!("writeups/drop_shadow.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    // Build the source scene: a stack of two rounded rects.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");

    let mut source_stage = Stage::new();
    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(120, 200, 255, 255)));
    g.stroke(Some(Stroke::new(0.04, Color::rgba_u8(20, 50, 90, 255))));
    g.draw_rounded_rect(Rect::new(-0.5, -0.4, 1.0, 0.8), 0.18);
    let _ = source_stage.add_child(source_stage.root(), g);

    let input_rt = RenderTexture::with_format(app, 384, 384, format);
    let _ = renderer.render_stage(app, input_rt.view(), Color::TRANSPARENT, &source_stage);

    let output_rt = RenderTexture::with_format(app, 384, 384, format);
    let filter = DropShadowFilter {
        offset: Vec2::new(8.0, 8.0),
        blur: 6.0,
        color: Color::rgba(0.0, 0.0, 0.0, 0.55),
    };
    renderer.apply_filter(app, &filter, &input_rt, &output_rt);

    let shadowed_bytes = output_rt.read_pixels(app);
    let shadowed_texture = Texture::from_rgba(app, 384, 384, &shadowed_bytes);

    let mut sprite = Sprite::from_texture(shadowed_texture).with_anchor(Vec2::splat(0.5));
    sprite.container.transform.scale = Vec2::splat(0.7);
    let _ = stage.add_child(stage.root(), sprite);
}
