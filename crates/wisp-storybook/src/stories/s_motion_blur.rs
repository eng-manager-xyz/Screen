//! Story: `MotionBlurFilter` — diagonal-velocity smear (M0.18).

use glam::Vec2;
use wisp::application::Application;
use wisp::render::Renderer;
use wisp::{Color, MotionBlurFilter, RenderTexture, Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "filter-motion-blur",
        category: "Filters",
        title: "Motion blur",
        milestone: "M0.18",
        writeup: include_str!("writeups/motion_blur.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");
    let source = dot_texture(app);

    // Source on the left.
    let mut sharp = Sprite::from_texture(source.clone()).with_anchor(Vec2::splat(0.5));
    sharp.container.transform.position = Vec2::new(-0.5, 0.0);
    sharp.container.transform.scale = Vec2::splat(0.35);
    let _ = stage.add_child(stage.root(), sharp);

    // Motion-blurred copy on the right.
    let input = RenderTexture::with_format(app, 256, 256, format);
    let output = RenderTexture::with_format(app, 256, 256, format);
    let mut sprite_stage = Stage::new();
    let mut sprite = Sprite::from_texture(source).with_anchor(Vec2::splat(0.5));
    sprite.container.transform.scale = Vec2::splat(0.95);
    let _ = sprite_stage.add_child(sprite_stage.root(), sprite);
    let _ = renderer.render_stage(app, input.view(), Color::BLACK, &sprite_stage);

    // Dramatic kernel so the static-frame PNG shows the smear
    // clearly. At 14 px the trail was barely visible on the 96-px
    // dot; 60 px gives a clear comet shape.
    let filter = MotionBlurFilter {
        velocity: Vec2::new(900.0, 600.0),
        peak_velocity_pps: 1100.0,
        max_kernel_px: 60.0,
    };
    renderer.apply_filter(app, &filter, &input, &output);

    let bytes = output.read_pixels(app);
    let blurred_texture = Texture::from_rgba(app, 256, 256, &bytes);
    let mut blurred = Sprite::from_texture(blurred_texture).with_anchor(Vec2::splat(0.5));
    blurred.container.transform.position = Vec2::new(0.5, 0.0);
    blurred.container.transform.scale = Vec2::splat(0.35);
    let _ = stage.add_child(stage.root(), blurred);
}

fn dot_texture(app: &Application) -> Texture {
    let size = 96u32;
    let mut bytes = Vec::with_capacity((size * size * 4) as usize);
    let center = (
        i32::try_from(size / 2).unwrap_or(48),
        i32::try_from(size / 2).unwrap_or(48),
    );
    let radius_sq = 28 * 28;
    for y in 0..size {
        for x in 0..size {
            let dx = i32::try_from(x).unwrap_or(0) - center.0;
            let dy = i32::try_from(y).unwrap_or(0) - center.1;
            if dx * dx + dy * dy < radius_sq {
                bytes.extend_from_slice(&[255, 200, 80, 255]);
            } else {
                bytes.extend_from_slice(&[20, 20, 30, 255]);
            }
        }
    }
    Texture::from_rgba(app, size, size, &bytes)
}
