//! Story: 100 sprites batched into a single draw call (M0.9).

use glam::Vec2;
use wisp::application::Application;
use wisp::{Color, Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "sprite-batcher",
        category: "Scene Graph",
        title: "Sprite batcher",
        milestone: "M0.9",
        writeup: include_str!("writeups/sprite_batcher.md"),
        build,
        tick: Some(tick),
    }
}

fn build(app: &Application, stage: &mut Stage) {
    // 4×4 white texture — all 100 sprites share the same Arc, so they batch.
    let bytes = vec![255u8; 4 * 4 * 4];
    let texture = Texture::from_rgba(app, 4, 4, &bytes);

    let root = stage.root();
    for i in 0u16..100 {
        let f = f32::from(i) / 100.0;
        let mut sprite = Sprite::from_texture(texture.clone()).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.scale = Vec2::splat(0.06);
        // Pseudo-random arrangement on a Lissajous curve for visual interest.
        let angle = f * std::f32::consts::TAU;
        let x = (angle * 2.0).sin() * 0.7;
        let y = (angle * 3.0).sin() * 0.7;
        sprite.container.transform.position = Vec2::new(x, y);
        sprite.tint = Color::rgba(
            0.5 + 0.5 * (angle * 1.0).cos(),
            0.5 + 0.5 * (angle * 1.5).sin(),
            0.5 + 0.5 * (angle * 2.0).cos(),
            1.0,
        );
        let _ = stage.add_child(root, sprite);
    }
}

fn tick(stage: &mut Stage, t: f32) {
    let child_ids: Vec<_> = stage
        .get(stage.root())
        .map(|root| root.container().children().collect())
        .unwrap_or_default();

    for (i, id) in child_ids.iter().enumerate().take(100) {
        let f = u16::try_from(i).map_or(0.0, f32::from) / 100.0;
        let phase = t + f * std::f32::consts::TAU;
        let angle = (f * std::f32::consts::TAU) + t * 0.2;
        let x = (angle * 2.0).sin() * 0.7;
        let y = (angle * 3.0).sin() * 0.7;
        if let Some(node) = stage.get_mut(*id) {
            node.container_mut().transform.position = glam::Vec2::new(x, y);
            node.container_mut().transform.rotation = phase * 0.3;
        }
    }
}
