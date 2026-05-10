//! Story: dynamic alpha-mask textures (M-DYN.1 / AUT-43).
//!
//! Contact sheet of the four SDF shapes plus the path variant, each
//! generated as a standalone alpha-mask texture and displayed as a
//! grayscale tile against the renderer's clear color. Demonstrates
//! that the mask primitive is decoupled from foreground sampling —
//! the texture itself is the deliverable.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{MaskShape, Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "mask-texture",
        category: "Mask",
        title: "Dynamic mask textures",
        milestone: "M-DYN.1 / AUT-43",
        writeup: include_str!("writeups/mask_texture.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");

    // No backdrop — Graphics primitives draw AFTER Sprites in the
    // batched renderer, so a full-canvas Graphics backdrop would
    // cover the mask tiles. We rely on the clear color instead.

    let tile_size: u32 = 192;
    let scale = 0.30;
    let positions = [-0.7, -0.35, 0.0, 0.35, 0.7];

    let bounds = Rect::new(-0.85, -0.85, 1.7, 1.7);
    let shapes = [
        MaskShape::rect(bounds),
        MaskShape::rounded_rect(bounds, 0.25),
        MaskShape::circle(Vec2::ZERO, 0.85),
        MaskShape::ellipse(Vec2::ZERO, Vec2::new(0.85, 0.5)),
    ];

    for (shape, x) in shapes.into_iter().zip(positions.iter().copied()) {
        let mask_rt = renderer.generate_mask_texture(app, shape, tile_size, tile_size);
        let bytes = mask_rt.read_pixels(app);
        let tex = Texture::from_rgba(app, tile_size, tile_size, &bytes);
        let mut sprite = Sprite::from_texture(tex).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.position = Vec2::new(x, 0.05);
        sprite.container.transform.scale = Vec2::splat(scale);
        let _ = stage.add_child(stage.root(), sprite);
    }

    // Path variant — five-pointed star at the rightmost slot.
    let star: Vec<Vec2> = (0i16..10)
        .map(|i| {
            let r = if i % 2 == 0 { 0.85 } else { 0.35 };
            let theta = f32::from(i) * std::f32::consts::PI / 5.0 - std::f32::consts::FRAC_PI_2;
            Vec2::new(theta.cos() * r, theta.sin() * r)
        })
        .collect();
    let star_rt = renderer.generate_path_mask_texture(app, &star, tile_size, tile_size);
    let star_bytes = star_rt.read_pixels(app);
    let star_tex = Texture::from_rgba(app, tile_size, tile_size, &star_bytes);
    let mut sprite = Sprite::from_texture(star_tex).with_anchor(Vec2::splat(0.5));
    sprite.container.transform.position = Vec2::new(positions[4], 0.05);
    sprite.container.transform.scale = Vec2::splat(scale);
    let _ = stage.add_child(stage.root(), sprite);
}
