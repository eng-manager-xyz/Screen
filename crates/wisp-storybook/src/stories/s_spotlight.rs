//! Story: spotlight / highlight mask (M-MASK.6 / AUT-28).
//!
//! Pretend "screen capture" with grid lines, plus a yellow "highlight
//! me" rectangle in the lower right. The spotlight primitive dims
//! everything outside a focus shape over the highlight, drawing the
//! viewer's eye there.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, MaskShape, RenderTexture, Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "spotlight",
        category: "Mask",
        title: "Spotlight / highlight",
        milestone: "M-MASK.6 / AUT-28",
        writeup: include_str!("writeups/spotlight.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");

    let base_rt = render_capture_with_target(&renderer, app, format);

    let output_rt = RenderTexture::with_format(app, 256, 256, format);
    // Focus on the lower-right "important button" area.
    let focus = Rect::new(0.18, -0.6, 0.55, 0.45);
    renderer.apply_spotlight(
        app,
        MaskShape::rounded_rect(focus, 0.06),
        Color::rgba(0.0, 0.0, 0.0, 0.7),
        &base_rt,
        &output_rt,
    );

    let bytes = output_rt.read_pixels(app);
    let tex = Texture::from_rgba(app, 256, 256, &bytes);
    let mut shown = Sprite::from_texture(tex).with_anchor(Vec2::splat(0.5));
    shown.container.transform.scale = Vec2::splat(0.9);
    let _ = stage.add_child(stage.root(), shown);
}

fn render_capture_with_target(
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
    // The "highlight me" target — a yellow button-like shape inside
    // the focus area.
    let mut button = Graphics::new();
    button.fill(Fill::Solid(Color::rgba_u8(255, 230, 80, 255)));
    button.draw_rounded_rect(Rect::new(0.25, -0.5, 0.4, 0.25), 0.06);
    let _ = s.add_child(s.root(), button);
    let rt = RenderTexture::with_format(app, 256, 256, format);
    let _ = renderer.render_stage(app, rt.view(), Color::BLACK, &s);
    rt
}
