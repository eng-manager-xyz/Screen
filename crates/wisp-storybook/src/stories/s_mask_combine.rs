//! Story: mask boolean operations (M-VEC.11 / AUT-63).
//!
//! Three side-by-side tiles showing the result of combining two
//! overlapping circle masks with `Union`, `Intersect`, and `Subtract`.

use glam::Vec2;
use wisp::application::Application;
use wisp::render::Renderer;
use wisp::{MaskCombineOp, MaskShape, RenderTexture, Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "mask-combine",
        category: "Mask",
        title: "Mask boolean ops",
        milestone: "M-VEC.11 / AUT-63",
        writeup: include_str!("writeups/mask_combine.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");

    let tile_size: u32 = 192;

    let a_shape = MaskShape::circle(Vec2::new(-0.25, 0.0), 0.55);
    let b_shape = MaskShape::circle(Vec2::new(0.25, 0.0), 0.55);
    let a = renderer.generate_mask_texture(app, a_shape, tile_size, tile_size);
    let b = renderer.generate_mask_texture(app, b_shape, tile_size, tile_size);

    let ops_and_positions = [
        (MaskCombineOp::Union, Vec2::new(-0.62, 0.0)),
        (MaskCombineOp::Intersect, Vec2::new(0.0, 0.0)),
        (MaskCombineOp::Subtract, Vec2::new(0.62, 0.0)),
    ];

    for (op, pos) in ops_and_positions {
        let combined = RenderTexture::with_format(app, tile_size, tile_size, format);
        renderer.combine_masks(app, &a, &b, op, &combined);

        let bytes = combined.read_pixels(app);
        let tex = Texture::from_rgba(app, tile_size, tile_size, &bytes);
        let mut sprite = Sprite::from_texture(tex).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.position = pos;
        sprite.container.transform.scale = Vec2::splat(0.28);
        let _ = stage.add_child(stage.root(), sprite);
    }
}
