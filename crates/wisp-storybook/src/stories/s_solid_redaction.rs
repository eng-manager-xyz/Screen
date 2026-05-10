//! Story: solid-color redaction (M-MASK.5 / AUT-23).
//!
//! Two redaction variants side-by-side over the same screen-capture
//! backdrop: a hard rectangle (left) and a rounded rectangle (right).
//! Both are filled with opaque black — the cinematic "this content
//! is sensitive, full stop" treatment.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, MaskShape, RenderTexture, Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "solid-redaction",
        category: "Mask",
        title: "Solid redaction",
        milestone: "M-MASK.5 / AUT-23",
        writeup: include_str!("writeups/solid_redaction.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");

    // Shared screen-capture backdrop (gradient + grid lines).
    let base_rt = render_capture(&renderer, app, format);

    // Left sprite — sharp rectangle.
    let rect_region = Rect::new(-0.5, -0.3, 1.0, 0.5);
    let left_rt = RenderTexture::with_format(app, 192, 192, format);
    renderer.apply_solid_redaction(
        app,
        MaskShape::rect(rect_region),
        Color::rgba_u8(20, 20, 20, 255),
        &base_rt,
        &left_rt,
    );
    let left_bytes = left_rt.read_pixels(app);
    let left_tex = Texture::from_rgba(app, 192, 192, &left_bytes);
    let mut left = Sprite::from_texture(left_tex).with_anchor(Vec2::splat(0.5));
    left.container.transform.position = Vec2::new(-0.46, 0.05);
    left.container.transform.scale = Vec2::splat(0.42);
    let _ = stage.add_child(stage.root(), left);

    // Right sprite — rounded rectangle.
    let rounded_rt = RenderTexture::with_format(app, 192, 192, format);
    renderer.apply_solid_redaction(
        app,
        MaskShape::rounded_rect(rect_region, 0.12),
        Color::rgba_u8(20, 20, 20, 255),
        &base_rt,
        &rounded_rt,
    );
    let right_bytes = rounded_rt.read_pixels(app);
    let right_tex = Texture::from_rgba(app, 192, 192, &right_bytes);
    let mut right = Sprite::from_texture(right_tex).with_anchor(Vec2::splat(0.5));
    right.container.transform.position = Vec2::new(0.46, 0.05);
    right.container.transform.scale = Vec2::splat(0.42);
    let _ = stage.add_child(stage.root(), right);

    // Captions under each variant.
    let mut sharp_lbl = Graphics::new();
    sharp_lbl.fill(Fill::Solid(Color::rgba_u8(220, 220, 130, 230)));
    sharp_lbl.draw_rounded_rect(Rect::new(-0.7, -0.46, 0.48, 0.06), 0.014);
    let _ = stage.add_child(stage.root(), sharp_lbl);

    let mut rounded_lbl = Graphics::new();
    rounded_lbl.fill(Fill::Solid(Color::rgba_u8(140, 200, 240, 230)));
    rounded_lbl.draw_rounded_rect(Rect::new(0.22, -0.46, 0.48, 0.06), 0.014);
    let _ = stage.add_child(stage.root(), rounded_lbl);
}

fn render_capture(
    renderer: &Renderer,
    app: &Application,
    format: wgpu::TextureFormat,
) -> RenderTexture {
    let mut s = Stage::new();
    let mut bg = Graphics::new();
    bg.fill(Fill::LinearGradient {
        start: Vec2::new(-1.0, 1.0),
        end: Vec2::new(1.0, -1.0),
        color_a: Color::rgba_u8(80, 130, 210, 255),
        color_b: Color::rgba_u8(220, 110, 80, 255),
    });
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = s.add_child(s.root(), bg);
    for i in -4i16..=4 {
        let x = f32::from(i) * 0.2;
        let mut line = Graphics::new();
        line.fill(Fill::Solid(Color::rgba_u8(255, 255, 255, 200)));
        line.draw_rect(Rect::new(x - 0.005, -1.0, 0.01, 2.0));
        let _ = s.add_child(s.root(), line);
    }
    for j in -4i16..=4 {
        let y = f32::from(j) * 0.2;
        let mut line = Graphics::new();
        line.fill(Fill::Solid(Color::rgba_u8(255, 255, 255, 200)));
        line.draw_rect(Rect::new(-1.0, y - 0.005, 2.0, 0.01));
        let _ = s.add_child(s.root(), line);
    }
    let rt = RenderTexture::with_format(app, 192, 192, format);
    let _ = renderer.render_stage(app, rt.view(), Color::BLACK, &s);
    rt
}
