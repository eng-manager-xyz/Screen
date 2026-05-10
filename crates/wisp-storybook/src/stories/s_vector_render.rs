//! Story: render vector primitives to scene geometry (M-VEC.2 / AUT-54).
//!
//! Five tiles showcasing the analytic shape variants of the
//! `Vector` primitive — rect, rounded rect, circle, ellipse, plus a
//! filled-with-stroke + opacity combo on the right. Path rendering
//! is deferred to M-VEC.10.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::{Color, Fill, Stage, Transform, Vector, VectorShape, VectorStroke};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "vector-render",
        category: "Vector",
        title: "Vector primitives",
        milestone: "M-VEC.2 / AUT-54",
        writeup: include_str!("writeups/vector_render.md"),
        build,
        tick: None,
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    let root = stage.root();

    // Tile positions across the canvas.
    let xs = [-0.7_f32, -0.35, 0.0, 0.35, 0.7];
    let scale = 0.14;
    let bounds = Rect::new(-1.0, -1.0, 2.0, 2.0);

    let entries = [
        // Rect — solid teal fill.
        Vector::new(VectorShape::rect(bounds))
            .with_fill(Fill::Solid(Color::rgba_u8(80, 180, 200, 255)))
            .with_transform(Transform {
                position: Vec2::new(xs[0], 0.0),
                scale: Vec2::splat(scale),
                ..Transform::default()
            }),
        // Rounded rect — solid amber fill.
        Vector::new(VectorShape::rounded_rect(bounds, 0.25))
            .with_fill(Fill::Solid(Color::rgba_u8(220, 170, 80, 255)))
            .with_transform(Transform {
                position: Vec2::new(xs[1], 0.0),
                scale: Vec2::splat(scale),
                ..Transform::default()
            }),
        // Circle — gradient fill.
        Vector::new(VectorShape::circle(Vec2::ZERO, 1.0))
            .with_fill(Fill::LinearGradient {
                start: Vec2::new(0.0, 1.0),
                end: Vec2::new(0.0, -1.0),
                color_a: Color::rgba_u8(220, 80, 130, 255),
                color_b: Color::rgba_u8(120, 40, 200, 255),
            })
            .with_transform(Transform {
                position: Vec2::new(xs[2], 0.0),
                scale: Vec2::splat(scale),
                ..Transform::default()
            }),
        // Ellipse — fill + stroke.
        Vector::new(VectorShape::ellipse(Vec2::ZERO, Vec2::new(1.0, 0.55)))
            .with_fill(Fill::Solid(Color::rgba_u8(120, 200, 130, 255)))
            .with_stroke(VectorStroke::new(0.06, Color::WHITE))
            .with_transform(Transform {
                position: Vec2::new(xs[3], 0.0),
                scale: Vec2::splat(scale),
                ..Transform::default()
            }),
        // Rounded rect — half opacity, demonstrates fill+opacity.
        Vector::new(VectorShape::rounded_rect(bounds, 0.4))
            .with_fill(Fill::Solid(Color::rgba_u8(255, 255, 255, 255)))
            .with_opacity(0.4)
            .with_transform(Transform {
                position: Vec2::new(xs[4], 0.0),
                scale: Vec2::splat(scale),
                ..Transform::default()
            }),
    ];

    for entry in entries {
        let _ = entry.add_to_stage(stage, root);
    }
}
