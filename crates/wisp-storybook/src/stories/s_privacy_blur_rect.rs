//! Story: rectangle privacy blur (M-MASK.2 / AUT-20).
//!
//! Pre-render a "screen capture" full of legible content (gradient
//! panels + grid lines), then privacy-blur a sub-rectangle so a
//! sensitive region of the recording is occluded while the rest stays
//! sharp. Demonstrates `Renderer::apply_privacy_blur` end-to-end.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, RenderTexture, Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "privacy-blur-rect",
        category: "Mask",
        title: "Rectangle privacy blur",
        milestone: "M-MASK.2 / AUT-20",
        writeup: include_str!("writeups/privacy_blur_rect.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");

    // Pretend "screen capture" RT — a colorful gradient with a grid
    // overlay so the blur is visible (sharp lines disappear inside the
    // privacy region).
    let mut capture_stage = Stage::new();

    let mut bg = Graphics::new();
    bg.fill(Fill::LinearGradient {
        start: Vec2::new(-1.0, 1.0),
        end: Vec2::new(1.0, -1.0),
        color_a: Color::rgba_u8(80, 130, 210, 255),
        color_b: Color::rgba_u8(220, 110, 80, 255),
    });
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = capture_stage.add_child(capture_stage.root(), bg);

    // Vertical grid lines — these reveal the blur sharply.
    for i in -4i16..=4 {
        let x = f32::from(i) * 0.2;
        let mut line = Graphics::new();
        line.fill(Fill::Solid(Color::rgba_u8(255, 255, 255, 200)));
        line.draw_rect(Rect::new(x - 0.005, -1.0, 0.01, 2.0));
        let _ = capture_stage.add_child(capture_stage.root(), line);
    }
    for j in -4i16..=4 {
        let y = f32::from(j) * 0.2;
        let mut line = Graphics::new();
        line.fill(Fill::Solid(Color::rgba_u8(255, 255, 255, 200)));
        line.draw_rect(Rect::new(-1.0, y - 0.005, 2.0, 0.01));
        let _ = capture_stage.add_child(capture_stage.root(), line);
    }

    let base_rt = RenderTexture::with_format(app, 256, 256, format);
    let _ = renderer.render_stage(app, base_rt.view(), Color::BLACK, &capture_stage);

    // Apply the privacy blur — center-right rect, a chunk of the grid
    // ends up obscured.
    let output_rt = RenderTexture::with_format(app, 256, 256, format);
    let region = Rect::new(0.05, -0.45, 0.55, 0.6);
    renderer.apply_privacy_blur(app, region, 12.0, &base_rt, &output_rt);

    let bytes = output_rt.read_pixels(app);
    let result_tex = Texture::from_rgba(app, 256, 256, &bytes);

    let mut shown = Sprite::from_texture(result_tex).with_anchor(Vec2::splat(0.5));
    shown.container.transform.scale = Vec2::splat(0.9);
    let _ = stage.add_child(stage.root(), shown);

    // Outline the privacy region so viewers can see what was redacted.
    let mut outline = Graphics::new();
    outline.fill(Fill::Solid(Color::rgba_u8(255, 240, 0, 200)));
    let inner = Rect::new(0.05 * 0.9, -0.45 * 0.9, 0.55 * 0.9, 0.6 * 0.9);
    let t = 0.008;
    outline.draw_rect(Rect::new(inner.min.x, inner.min.y, inner.size.x, t));
    outline.draw_rect(Rect::new(
        inner.min.x,
        inner.min.y + inner.size.y - t,
        inner.size.x,
        t,
    ));
    outline.draw_rect(Rect::new(inner.min.x, inner.min.y, t, inner.size.y));
    outline.draw_rect(Rect::new(
        inner.min.x + inner.size.x - t,
        inner.min.y,
        t,
        inner.size.y,
    ));
    let _ = stage.add_child(stage.root(), outline);
}
