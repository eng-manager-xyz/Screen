//! Story: freehand path mask (M-MASK.10 / AUT-35).
//!
//! A five-pointed-star polygon clips a textured frame and a redacted
//! frame side-by-side, demonstrating both `apply_path_clip` (alpha
//! cutout) and `apply_solid_redaction_path` (filled redaction).

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, RenderTexture, Sprite, Stage, Texture};
// No Graphics backdrop — see s_ellipse for the rationale (Graphics
// paints after Sprites in render_stage's batched pipeline order).

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "path-mask",
        category: "Mask",
        title: "Freehand path mask",
        milestone: "M-MASK.10 / AUT-35",
        writeup: include_str!("writeups/path_mask.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");

    let frame_rt = render_textured_frame(&renderer, app, format);
    let star = star_polygon();

    // Left: alpha-cutout via apply_path_clip.
    let cutout = RenderTexture::with_format(app, 256, 256, format);
    renderer.apply_path_clip(app, &star, &frame_rt, &cutout);
    let cb = cutout.read_pixels(app);
    let ct = Texture::from_rgba(app, 256, 256, &cb);
    let mut left = Sprite::from_texture(ct).with_anchor(Vec2::splat(0.5));
    left.container.transform.position = Vec2::new(-0.45, 0.0);
    left.container.transform.scale = Vec2::splat(0.4);
    let _ = stage.add_child(stage.root(), left);

    // Right: solid redaction via apply_solid_redaction_path over the
    // textured frame (which becomes the "base" here).
    let redacted = RenderTexture::with_format(app, 256, 256, format);
    renderer.apply_solid_redaction_path(
        app,
        &star,
        Color::rgba_u8(20, 20, 20, 255),
        &frame_rt,
        &redacted,
    );
    let rb = redacted.read_pixels(app);
    let rt = Texture::from_rgba(app, 256, 256, &rb);
    let mut right = Sprite::from_texture(rt).with_anchor(Vec2::splat(0.5));
    right.container.transform.position = Vec2::new(0.45, 0.0);
    right.container.transform.scale = Vec2::splat(0.4);
    let _ = stage.add_child(stage.root(), right);
}

fn star_polygon() -> Vec<Vec2> {
    let mut pts = Vec::with_capacity(10);
    for i in 0i16..10 {
        let r = if i % 2 == 0 { 0.85 } else { 0.35 };
        let theta = f32::from(i) * std::f32::consts::PI / 5.0 - std::f32::consts::FRAC_PI_2;
        pts.push(Vec2::new(theta.cos() * r, theta.sin() * r));
    }
    pts
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
