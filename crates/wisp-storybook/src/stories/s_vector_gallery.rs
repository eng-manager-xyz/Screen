//! Story: vector primitive examples gallery (M-VEC.12 / AUT-64).
//!
//! Single-canvas overview of the M-VEC catalog. Each existing
//! storybook entry deep-dives one primitive; this one shows the
//! whole stack in one glance so reviewers can scan the surface
//! quickly before clicking through.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::{Callout, Color, Fill, Highlight, PathBuilder, Stage, VectorShape, VectorStroke};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "vector-gallery",
        category: "Vector",
        title: "Examples gallery",
        milestone: "M-VEC.12 / AUT-64",
        writeup: include_str!("writeups/vector_gallery.md"),
        build,
        tick: None,
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    let root = stage.root();

    // Top row — basic shape primitives.
    add_shape_tile(
        stage,
        root,
        VectorShape::rect(Rect::new(-1.0, -1.0, 2.0, 2.0)),
        Color::rgba_u8(80, 180, 200, 255),
        Vec2::new(-0.7, 0.55),
        0.08,
    );
    add_shape_tile(
        stage,
        root,
        VectorShape::rounded_rect(Rect::new(-1.0, -1.0, 2.0, 2.0), 0.25),
        Color::rgba_u8(220, 170, 80, 255),
        Vec2::new(-0.4, 0.55),
        0.08,
    );
    add_shape_tile(
        stage,
        root,
        VectorShape::circle(Vec2::ZERO, 1.0),
        Color::rgba_u8(220, 80, 130, 255),
        Vec2::new(-0.1, 0.55),
        0.08,
    );
    add_shape_tile(
        stage,
        root,
        VectorShape::ellipse(Vec2::ZERO, Vec2::new(1.0, 0.55)),
        Color::rgba_u8(120, 200, 130, 255),
        Vec2::new(0.2, 0.55),
        0.08,
    );

    // Top right — highlight outline.
    let outline = Highlight::outline(
        VectorShape::rounded_rect(Rect::new(0.45, 0.45, 0.25, 0.20), 0.05),
        Color::rgba_u8(255, 230, 80, 255),
        0.018,
    );
    let _ = outline.add_to_stage(stage, root);

    // Middle row — arrow + Bezier curve.
    let arrow = Callout::arrow_to(
        Vec2::new(-0.7, 0.15),
        Vec2::new(-0.1, 0.15),
        0.022,
        Color::rgba_u8(255, 230, 80, 255),
    );
    let _ = stage.add_child(root, arrow);

    let curve = PathBuilder::new()
        .move_to(Vec2::new(0.05, 0.1))
        .quad_to(Vec2::new(0.35, 0.3), Vec2::new(0.65, 0.1))
        .build()
        .stroke_to_graphics(0.022, Color::rgba_u8(80, 200, 240, 255), 0.005);
    let _ = stage.add_child(root, curve);

    // Lower row — callout label box + badge.
    let label = Callout::label_box(
        Rect::new(-0.7, -0.35, 0.55, 0.18),
        Color::rgba_u8(220, 170, 80, 230),
        Some(VectorStroke::new(0.01, Color::WHITE)),
        0.04,
    );
    let _ = label.add_to_stage(stage, root);

    let badge = Callout::badge(
        Vec2::new(0.0, -0.26),
        0.085,
        Color::rgba_u8(220, 60, 50, 255),
    );
    let _ = badge.add_to_stage(stage, root);

    // Translucent pill highlight.
    let pill = Highlight::pill(
        Rect::new(0.2, -0.32, 0.55, 0.13),
        Color::rgba_u8(80, 220, 240, 255),
        0.5,
    );
    let _ = pill.add_to_stage(stage, root);

    // Bottom — caption pill.
    let caption = Callout::caption_pill(
        Rect::new(-0.5, -0.7, 1.0, 0.08),
        Color::rgba_u8(40, 50, 70, 240),
    );
    let _ = caption.add_to_stage(stage, root);
}

fn add_shape_tile(
    stage: &mut Stage,
    parent: wisp::NodeId,
    shape: VectorShape,
    color: Color,
    pos: Vec2,
    scale: f32,
) {
    let v = wisp::Vector::new(shape)
        .with_fill(Fill::Solid(color))
        .with_transform(wisp::Transform {
            position: pos,
            scale: Vec2::splat(scale),
            ..wisp::Transform::default()
        });
    let _ = v.add_to_stage(stage, parent);
}
