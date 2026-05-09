//! Story: `ColorMatrixFilter` — sharp source vs grayscale + brightness (M0.18).

use glam::Vec2;
use wisp::application::Application;
use wisp::render::Renderer;
use wisp::{Color, ColorMatrixFilter, RenderTexture, Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "filter-color-matrix",
        category: "Filters",
        title: "Color matrix",
        milestone: "M0.18",
        writeup: include_str!("writeups/color_matrix.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");
    let source = stripe_texture(app);

    let make_filtered = |filter: ColorMatrixFilter| -> Texture {
        let input = RenderTexture::with_format(app, 256, 256, format);
        let output = RenderTexture::with_format(app, 256, 256, format);
        let mut sprite_stage = Stage::new();
        let mut sprite = Sprite::from_texture(source.clone()).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.scale = Vec2::splat(0.95);
        let _ = sprite_stage.add_child(sprite_stage.root(), sprite);
        let _ = renderer.render_stage(app, input.view(), Color::BLACK, &sprite_stage);
        renderer.apply_filter(app, &filter, &input, &output);
        let bytes = output.read_pixels(app);
        Texture::from_rgba(app, 256, 256, &bytes)
    };

    let identity = make_filtered(ColorMatrixFilter::identity());
    let grayscale = make_filtered(ColorMatrixFilter::grayscale());
    let brightened = make_filtered(ColorMatrixFilter::brightness(1.4));

    place(stage, identity, Vec2::new(-0.6, 0.0));
    place(stage, grayscale, Vec2::new(0.0, 0.0));
    place(stage, brightened, Vec2::new(0.6, 0.0));
}

fn place(stage: &mut Stage, texture: Texture, position: Vec2) {
    let mut sprite = Sprite::from_texture(texture).with_anchor(Vec2::splat(0.5));
    sprite.container.transform.position = position;
    sprite.container.transform.scale = Vec2::splat(0.28);
    let _ = stage.add_child(stage.root(), sprite);
}

fn stripe_texture(app: &Application) -> Texture {
    let size = 64u32;
    let mut bytes = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let band = x * 4 / size;
            let rgb = match band {
                0 => [255, 80, 80, 255],
                1 => [80, 255, 80, 255],
                2 => [80, 80, 255, 255],
                _ => [255, 220, 80, 255],
            };
            let _ = y; // used implicitly via row sequencing
            bytes.extend_from_slice(&rgb);
        }
    }
    Texture::from_rgba(app, size, size, &bytes)
}
