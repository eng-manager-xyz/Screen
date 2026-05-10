//! Story: vector highlight + callout overlays
//! (M-VEC.8 / AUT-60 + M-VEC.9 / AUT-61).
//!
//! Composite scene with a "fake UI" backdrop and several overlay
//! presets demonstrating both modules:
//!
//! - Highlight outline ring around a button.
//! - Highlight filled pill behind a label.
//! - Callout label box for an annotation.
//! - Callout numbered badge.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::{Callout, Color, Highlight, Stage, VectorShape, VectorStroke};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "vector-overlays",
        category: "Vector",
        title: "Highlight + callout overlays",
        milestone: "M-VEC.8 / M-VEC.9",
        writeup: include_str!("writeups/vector_overlays.md"),
        build,
        tick: None,
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    let root = stage.root();

    // Outline highlight — yellow ring around the upper-left "button" rect.
    let outline = Highlight::outline(
        VectorShape::rounded_rect(Rect::new(-0.7, 0.25, 0.45, 0.25), 0.06),
        Color::rgba_u8(255, 230, 80, 255),
        0.025,
    );
    let _ = outline.add_to_stage(stage, root);

    // Filled pill highlight — translucent cyan over a rectangular label area.
    let pill = Highlight::pill(
        Rect::new(0.05, 0.30, 0.55, 0.12),
        Color::rgba_u8(80, 220, 240, 255),
        0.5,
    );
    let _ = pill.add_to_stage(stage, root);

    // Callout label box — amber rounded card with a white outline.
    let label = Callout::label_box(
        Rect::new(-0.6, -0.4, 0.6, 0.25),
        Color::rgba_u8(220, 170, 80, 230),
        Some(VectorStroke::new(0.012, Color::WHITE)),
        0.04,
    );
    let _ = label.add_to_stage(stage, root);

    // Callout numbered badge — red circle in the upper right.
    let badge = Callout::badge(
        Vec2::new(0.55, 0.5),
        0.085,
        Color::rgba_u8(220, 60, 50, 255),
    );
    let _ = badge.add_to_stage(stage, root);

    // Caption pill at the bottom.
    let caption = Callout::caption_pill(
        Rect::new(-0.55, -0.7, 1.1, 0.09),
        Color::rgba_u8(40, 50, 70, 240),
    );
    let _ = caption.add_to_stage(stage, root);

    // Glow approximation around the badge — wider stroke with low alpha.
    let glow = Highlight::glow(
        VectorShape::circle(Vec2::new(0.55, 0.5), 0.1),
        Color::rgba_u8(255, 80, 70, 255),
        0.05,
    );
    let _ = glow.add_to_stage(stage, root);
}
