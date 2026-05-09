//! Story: nested containers demonstrating parent-child transform composition (M0.7 + M0.8).

use glam::Vec2;
use wisp::application::Application;
use wisp::{Container, Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "transform-nesting",
        category: "Scene Graph",
        title: "Nested transforms",
        milestone: "M0.7 / M0.8",
        writeup: include_str!("writeups/transform_nesting.md"),
        build,
        tick: Some(tick),
    }
}

fn build(app: &Application, stage: &mut Stage) {
    // Three colored 4×4 textures so each ring is visually distinct.
    let red = solid_texture(app, [255, 100, 100, 255]);
    let green = solid_texture(app, [100, 255, 100, 255]);
    let blue = solid_texture(app, [100, 180, 255, 255]);

    // Ring 1: outer container with 3 sprites.
    let outer = Container::default();
    let outer_id = stage.add_child(stage.root(), outer).expect("outer");
    add_orbiters(stage, outer_id, &red, 0.55, 6);

    // Ring 2: nested container, smaller orbit.
    let mid = Container::default();
    let mid_id = stage.add_child(outer_id, mid).expect("mid");
    add_orbiters(stage, mid_id, &green, 0.35, 4);

    // Ring 3: deepest container, smallest orbit.
    let inner = Container::default();
    let inner_id = stage.add_child(mid_id, inner).expect("inner");
    add_orbiters(stage, inner_id, &blue, 0.18, 3);
}

fn solid_texture(app: &Application, rgba: [u8; 4]) -> Texture {
    let mut bytes = Vec::with_capacity(4 * 4 * 4);
    for _ in 0..(4 * 4) {
        bytes.extend_from_slice(&rgba);
    }
    Texture::from_rgba(app, 4, 4, &bytes)
}

fn add_orbiters(stage: &mut Stage, parent: wisp::NodeId, texture: &Texture, radius: f32, n: u8) {
    for i in 0..n {
        let theta = std::f32::consts::TAU * f32::from(i) / f32::from(n);
        let mut sprite = Sprite::from_texture(texture.clone()).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.position = Vec2::new(theta.cos() * radius, theta.sin() * radius);
        sprite.container.transform.scale = Vec2::splat(0.08);
        let _ = stage.add_child(parent, sprite);
    }
}

fn tick(stage: &mut Stage, t: f32) {
    // Rotate each ring at a different rate; outer slowest, inner fastest.
    let root_children: Vec<_> = stage
        .get(stage.root())
        .map(|n| n.container().children().collect())
        .unwrap_or_default();
    let Some(outer_id) = root_children.first().copied() else {
        return;
    };
    set_rotation(stage, outer_id, t * 0.3);

    let mid_children: Vec<_> = stage
        .get(outer_id)
        .map(|n| n.container().children().collect())
        .unwrap_or_default();
    // The first 6 children are sprites (the orbiters). The 7th is the next ring.
    if let Some(mid_id) = mid_children.iter().next_back().copied() {
        set_rotation(stage, mid_id, t * -0.7);
        let inner_children: Vec<_> = stage
            .get(mid_id)
            .map(|n| n.container().children().collect())
            .unwrap_or_default();
        if let Some(inner_id) = inner_children.iter().next_back().copied() {
            set_rotation(stage, inner_id, t * 1.3);
        }
    }
}

fn set_rotation(stage: &mut Stage, id: wisp::NodeId, angle: f32) {
    if let Some(node) = stage.get_mut(id) {
        node.container_mut().transform.rotation = angle;
    }
}
