//! Story: path stroke (M-VEC.10 / AUT-62).
//!
//! Three flavors of stroked path:
//!
//! - Straight arrow (`Callout::arrow_to`).
//! - Bezier curve (quad) — adaptive flattening produces enough
//!   segments for a smooth visual.
//! - Polyline freehand stroke.

use glam::Vec2;
use wisp::application::Application;
use wisp::{Callout, Color, PathBuilder, Stage};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "path-stroke",
        category: "Vector",
        title: "Path stroke + arrows",
        milestone: "M-VEC.10 / AUT-62",
        writeup: include_str!("writeups/path_stroke.md"),
        build,
        tick: None,
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    let root = stage.root();

    // Arrow — Callout helper.
    let arrow = Callout::arrow_to(
        Vec2::new(-0.7, 0.5),
        Vec2::new(-0.1, 0.0),
        0.025,
        Color::rgba_u8(255, 230, 80, 255),
    );
    let _ = stage.add_child(root, arrow);

    // Quadratic Bezier curve.
    let curve = PathBuilder::new()
        .move_to(Vec2::new(-0.6, -0.4))
        .quad_to(Vec2::new(0.0, 0.6), Vec2::new(0.6, -0.4))
        .build()
        .stroke_to_graphics(0.025, Color::rgba_u8(80, 200, 240, 255), 0.005);
    let _ = stage.add_child(root, curve);

    // Polyline freehand stroke.
    let freehand = PathBuilder::new()
        .move_to(Vec2::new(0.1, 0.5))
        .line_to(Vec2::new(0.3, 0.6))
        .line_to(Vec2::new(0.5, 0.4))
        .line_to(Vec2::new(0.7, 0.55))
        .line_to(Vec2::new(0.85, 0.3))
        .build()
        .stroke_to_graphics(0.02, Color::rgba_u8(220, 110, 80, 255), 0.005);
    let _ = stage.add_child(root, freehand);
}
