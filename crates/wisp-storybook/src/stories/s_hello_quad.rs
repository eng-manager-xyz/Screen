//! Story: textured quad as a single sprite (M0.6 + M0.9).

use glam::Vec2;
use wisp::application::Application;
use wisp::{Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "hello-quad",
        category: "Renderer Foundation",
        title: "Textured quad",
        milestone: "M0.6",
        writeup: include_str!("writeups/hello_quad.md"),
        build,
        tick: Some(tick),
    }
}

const CHECKER: u32 = 64;
const CELL: u32 = 8;

fn build(app: &Application, stage: &mut Stage) {
    let mut bytes = Vec::with_capacity((CHECKER * CHECKER * 4) as usize);
    for y in 0..CHECKER {
        for x in 0..CHECKER {
            let cell_x = (x / CELL) % 2;
            let cell_y = (y / CELL) % 2;
            let dark = (cell_x ^ cell_y) == 1;
            let v = if dark { 64 } else { 224 };
            bytes.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let texture = Texture::from_rgba(app, CHECKER, CHECKER, &bytes);

    let mut sprite = Sprite::from_texture(texture).with_anchor(Vec2::splat(0.5));
    sprite.container.transform.scale = Vec2::splat(0.6);
    let _ = stage.add_child(stage.root(), sprite);
}

fn tick(stage: &mut Stage, t: f32) {
    let child_ids: Vec<_> = stage
        .get(stage.root())
        .map(|n| n.container().children().collect())
        .unwrap_or_default();
    if let Some(id) = child_ids.first()
        && let Some(node) = stage.get_mut(*id)
    {
        node.container_mut().transform.rotation = t * 0.4;
    }
}
