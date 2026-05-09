//! Story: Mesh with Y-axis perspective rotation, animated (M0.19).

use glam::Vec2;
use wisp::application::Application;
use wisp::{Mesh, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "mesh-perspective",
        category: "Scene Graph",
        title: "Perspective rotation",
        milestone: "M0.19",
        writeup: include_str!("writeups/mesh_perspective.md"),
        build,
        tick: Some(tick),
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let texture = checker_texture(app);
    let mut mesh = Mesh::from_texture(texture).with_perspective(0.5);
    mesh.container.transform.scale = Vec2::splat(0.5);
    let _ = stage.add_child(stage.root(), mesh);
}

fn tick(stage: &mut Stage, t: f32) {
    let child_ids: Vec<_> = stage
        .get(stage.root())
        .map(|n| n.container().children().collect())
        .unwrap_or_default();
    if let Some(id) = child_ids.first()
        && let Some(node) = stage.get_mut(*id)
        && let wisp::Node::Mesh(mesh) = node
    {
        mesh.rotation_y = t * 0.6;
    }
}

fn checker_texture(app: &Application) -> Texture {
    let size = 64u32;
    let mut bytes = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let cell_x = (x / 8) % 2;
            let cell_y = (y / 8) % 2;
            let dark = (cell_x ^ cell_y) == 1;
            if dark {
                bytes.extend_from_slice(&[80, 200, 255, 255]);
            } else {
                bytes.extend_from_slice(&[255, 220, 100, 255]);
            }
        }
    }
    Texture::from_rgba(app, size, size, &bytes)
}
