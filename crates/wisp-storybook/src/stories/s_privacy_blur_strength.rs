//! Story: privacy blur strength variants (M-MASK.4 / AUT-22).
//!
//! Side-by-side: same shape, three named strengths (Soft / Medium /
//! Strong). Demonstrates that the editor's strength slider picks
//! between cinematic preset levels rather than exposing raw pixel
//! radii.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{
    BlurStrength, Color, Fill, Graphics, PrivacyBlur, RenderTexture, Sprite, Stage, Texture,
};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "privacy-blur-strength",
        category: "Mask",
        title: "Privacy blur strengths",
        milestone: "M-MASK.4 / AUT-22",
        writeup: include_str!("writeups/privacy_blur_strength.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");

    // One reusable base — colorful gradient + grid lines.
    let base_rt = render_capture(&renderer, app, format);

    // Three variants, three sprites along the X axis.
    let strengths = [
        (BlurStrength::Soft, Vec2::new(-0.62, 0.0)),
        (BlurStrength::Medium, Vec2::new(0.0, 0.0)),
        (BlurStrength::Strong, Vec2::new(0.62, 0.0)),
    ];

    // The privacy region is full-canvas — every shown sprite is
    // entirely covered by the blur, so the strength comparison is
    // visible across the whole sprite. Outline drawn separately.
    let region = Rect::new(-1.0, -1.0, 2.0, 2.0);

    for (strength, pos) in strengths {
        let blur = PrivacyBlur::rect(region).with_strength(strength);
        let out_rt = RenderTexture::with_format(app, 192, 192, format);
        // Re-render the base into an RT sized 192 to match this
        // sprite's pixel scale; reuse base_rt for now since it's
        // nominally same content (the renderer scales as the sprite
        // is laid out).
        renderer.apply_privacy_blur_data(app, &blur, &base_rt, &out_rt);
        let bytes = out_rt.read_pixels(app);
        let tex = Texture::from_rgba(app, 192, 192, &bytes);
        let mut sprite = Sprite::from_texture(tex).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.position = pos;
        sprite.container.transform.scale = Vec2::splat(0.28);
        let _ = stage.add_child(stage.root(), sprite);
    }

    // Strength labels (tiny tinted bars beneath each sprite) so the
    // viewer can tell which is which at a glance.
    let label_y = -0.32;
    let label_h = 0.05;
    let label_w = 0.32;
    let labels = [
        (
            Vec2::new(-0.62, label_y),
            Color::rgba_u8(140, 200, 240, 230),
        ), // Soft (cool blue)
        (Vec2::new(0.0, label_y), Color::rgba_u8(220, 220, 130, 230)), // Medium (yellow)
        (Vec2::new(0.62, label_y), Color::rgba_u8(240, 130, 130, 230)), // Strong (warm red)
    ];
    for (pos, color) in labels {
        let mut bar = Graphics::new();
        bar.fill(Fill::Solid(color));
        bar.draw_rounded_rect(
            Rect::new(
                pos.x - label_w * 0.5,
                pos.y - label_h * 0.5,
                label_w,
                label_h,
            ),
            0.012,
        );
        let _ = stage.add_child(stage.root(), bar);
    }
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
