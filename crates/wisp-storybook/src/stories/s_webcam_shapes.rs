//! Story: webcam shape masks (M-MASK.8 / AUT-30).
//!
//! Two webcam-style overlays side-by-side: circle (creator-style) and
//! rounded rectangle (professional walkthrough). Both clip a fake
//! "webcam frame" sprite via `apply_clip`, then composite over a
//! gradient backdrop so the masked silhouette is visible.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, MaskShape, RenderTexture, Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "webcam-shapes",
        category: "Mask",
        title: "Webcam shapes",
        milestone: "M-MASK.8 / AUT-30",
        writeup: include_str!("writeups/webcam_shapes.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");

    // Backdrop — a gradient panel so the clipped silhouettes pop.
    let mut bg = Graphics::new();
    bg.fill(Fill::LinearGradient {
        start: Vec2::new(0.0, 1.0),
        end: Vec2::new(0.0, -1.0),
        color_a: Color::rgba_u8(40, 50, 70, 255),
        color_b: Color::rgba_u8(20, 25, 35, 255),
    });
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), bg);

    // The "webcam frame" — a face-tinted gradient inside a checker
    // grid so the clip shape is unambiguous.
    let webcam_frame_rt = render_webcam_frame(&renderer, app, format);

    // Left: circle crop.
    let circle_clipped = RenderTexture::with_format(app, 256, 256, format);
    let circle = MaskShape::circle(Vec2::ZERO, 0.85);
    renderer.apply_clip(app, circle, &webcam_frame_rt, &circle_clipped);
    let circle_bytes = circle_clipped.read_pixels(app);
    let circle_tex = Texture::from_rgba(app, 256, 256, &circle_bytes);
    let mut circle_sprite = Sprite::from_texture(circle_tex).with_anchor(Vec2::splat(0.5));
    circle_sprite.container.transform.position = Vec2::new(-0.45, 0.0);
    circle_sprite.container.transform.scale = Vec2::splat(0.4);
    let _ = stage.add_child(stage.root(), circle_sprite);

    // Right: rounded-rectangle crop.
    let rounded_clipped = RenderTexture::with_format(app, 256, 256, format);
    let rounded = MaskShape::rounded_rect(Rect::new(-0.85, -0.85, 1.7, 1.7), 0.18);
    renderer.apply_clip(app, rounded, &webcam_frame_rt, &rounded_clipped);
    let rounded_bytes = rounded_clipped.read_pixels(app);
    let rounded_tex = Texture::from_rgba(app, 256, 256, &rounded_bytes);
    let mut rounded_sprite = Sprite::from_texture(rounded_tex).with_anchor(Vec2::splat(0.5));
    rounded_sprite.container.transform.position = Vec2::new(0.45, 0.0);
    rounded_sprite.container.transform.scale = Vec2::splat(0.4);
    let _ = stage.add_child(stage.root(), rounded_sprite);

    // Captions under each shape.
    let mut left_lbl = Graphics::new();
    left_lbl.fill(Fill::Solid(Color::rgba_u8(140, 200, 240, 220)));
    left_lbl.draw_rounded_rect(Rect::new(-0.7, -0.55, 0.5, 0.06), 0.014);
    let mut right_lbl = Graphics::new();
    right_lbl.fill(Fill::Solid(Color::rgba_u8(220, 220, 130, 220)));
    right_lbl.draw_rounded_rect(Rect::new(0.2, -0.55, 0.5, 0.06), 0.014);
    let _ = stage.add_child(stage.root(), left_lbl);
    let _ = stage.add_child(stage.root(), right_lbl);
}

fn render_webcam_frame(
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
    // Checker-ish grid for clip-shape visibility.
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
