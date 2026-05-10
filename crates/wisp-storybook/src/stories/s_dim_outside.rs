//! Story: dim-outside strength variants (M-MASK.7 / AUT-29).
//!
//! Three side-by-side panels showing the same screen capture
//! focus-zoned with Light / Medium / Heavy `DimStrength`. The
//! `DimOutside` data API is what the editor inspector will eventually
//! drive.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, DimOutside, DimStrength, Fill, Graphics, RenderTexture, Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "dim-outside",
        category: "Mask",
        title: "Dim outside (variants)",
        milestone: "M-MASK.7 / AUT-29",
        writeup: include_str!("writeups/dim_outside.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");

    let base_rt = render_capture_with_target(&renderer, app, format);

    let strengths = [
        (DimStrength::Light, Vec2::new(-0.62, 0.0)),
        (DimStrength::Medium, Vec2::new(0.0, 0.0)),
        (DimStrength::Heavy, Vec2::new(0.62, 0.0)),
    ];

    let focus = Rect::new(-0.5, -0.55, 1.0, 0.7);

    for (strength, pos) in strengths {
        let dim = DimOutside::rounded_rect(focus, 0.08).with_strength(strength);
        let out_rt = RenderTexture::with_format(app, 192, 192, format);
        renderer.apply_dim_outside_data(app, &dim, &base_rt, &out_rt);
        let bytes = out_rt.read_pixels(app);
        let tex = Texture::from_rgba(app, 192, 192, &bytes);
        let mut sprite = Sprite::from_texture(tex).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.position = pos;
        sprite.container.transform.scale = Vec2::splat(0.28);
        let _ = stage.add_child(stage.root(), sprite);
    }

    // Strength labels.
    let label_y = -0.3;
    let label_w = 0.32;
    let label_h = 0.05;
    let labels = [
        (
            Vec2::new(-0.62, label_y),
            Color::rgba_u8(170, 200, 240, 230),
        ),
        (Vec2::new(0.0, label_y), Color::rgba_u8(220, 220, 130, 230)),
        (Vec2::new(0.62, label_y), Color::rgba_u8(60, 60, 70, 240)),
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
    let rt = RenderTexture::with_format(app, 192, 192, format);
    let _ = renderer.render_stage(app, rt.view(), Color::BLACK, &s);
    rt
}
