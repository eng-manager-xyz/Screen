//! Story: ellipse mask (M-MASK.9 / AUT-34).
//!
//! Three side-by-side ellipse cutouts — wide, tall, square (which is
//! a circle) — over a textured webcam-style backdrop, demonstrating
//! the new anisotropic SDF branch.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, MaskShape, RenderTexture, Sprite, Stage, Texture};
// Note: the storybook exporter clears to BLACK before render_stage,
// so masked sprites pop against black. We do NOT add a Graphics
// backdrop here — Graphics paints AFTER Sprites in `render_stage`'s
// pipeline-batched submission order, so a full-canvas backdrop would
// overlay the masked sprites (CLAUDE.md "Renderer batching / draw
// order" note).

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "ellipse-mask",
        category: "Mask",
        title: "Ellipse mask",
        milestone: "M-MASK.9 / AUT-34",
        writeup: include_str!("writeups/ellipse_mask.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");

    let frame_rt = render_textured_frame(&renderer, app, format);

    let variants = [
        // (half_extents, position)
        (Vec2::new(0.85, 0.4), Vec2::new(-0.62, 0.0)), // wide
        (Vec2::new(0.4, 0.85), Vec2::new(0.0, 0.0)),   // tall
        (Vec2::new(0.7, 0.7), Vec2::new(0.62, 0.0)),   // round
    ];

    for (half, pos) in variants {
        let clipped = RenderTexture::with_format(app, 256, 256, format);
        let shape = MaskShape::ellipse(Vec2::ZERO, half);
        renderer.apply_clip(app, shape, &frame_rt, &clipped);
        let bytes = clipped.read_pixels(app);
        let tex = Texture::from_rgba(app, 256, 256, &bytes);
        let mut sprite = Sprite::from_texture(tex).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.position = pos;
        sprite.container.transform.scale = Vec2::splat(0.28);
        let _ = stage.add_child(stage.root(), sprite);
    }
}

fn render_textured_frame(
    renderer: &Renderer,
    app: &Application,
    format: wgpu::TextureFormat,
) -> RenderTexture {
    let mut s = Stage::new();
    let mut tone = Graphics::new();
    tone.fill(Fill::LinearGradient {
        start: Vec2::new(-1.0, 1.0),
        end: Vec2::new(1.0, -1.0),
        color_a: Color::rgba_u8(255, 195, 150, 255),
        color_b: Color::rgba_u8(150, 100, 80, 255),
    });
    tone.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = s.add_child(s.root(), tone);
    for i in -3i16..=3 {
        let x = f32::from(i) * 0.25;
        let mut line = Graphics::new();
        line.fill(Fill::Solid(Color::rgba_u8(255, 255, 255, 180)));
        line.draw_rect(Rect::new(x - 0.01, -1.0, 0.02, 2.0));
        let _ = s.add_child(s.root(), line);
    }
    for j in -3i16..=3 {
        let y = f32::from(j) * 0.25;
        let mut line = Graphics::new();
        line.fill(Fill::Solid(Color::rgba_u8(255, 255, 255, 180)));
        line.draw_rect(Rect::new(-1.0, y - 0.01, 2.0, 0.02));
        let _ = s.add_child(s.root(), line);
    }
    let rt = RenderTexture::with_format(app, 256, 256, format);
    let _ = renderer.render_stage(app, rt.view(), Color::TRANSPARENT, &s);
    rt
}
