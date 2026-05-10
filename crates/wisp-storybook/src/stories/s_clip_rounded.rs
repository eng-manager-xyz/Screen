//! Story: rounded-rect clip foundation (M-MASK.1 / AUT-31).
//!
//! A solid-color graphics primitive fills the whole NDC viewport.
//! A `Container` wraps it with `clip = MaskShape::RoundedRect(...)`,
//! producing the "cinematic recording-quad crop" shape the recorder
//! will use for its primary screen-capture surface.

use wisp::application::Application;
use wisp::math::Rect;
use wisp::{Color, Container, Fill, Graphics, MaskShape, Node, Stage};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "clip-rounded",
        category: "Mask",
        title: "Rounded crop foundation",
        milestone: "M-MASK.1 / AUT-31",
        writeup: include_str!("writeups/clip_rounded.md"),
        build,
        tick: None,
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    // Backdrop graphics — gradient panel so the rounded edge is
    // visible against a varying color.
    let mut bg = Graphics::new();
    bg.fill(Fill::LinearGradient {
        start: glam::Vec2::new(0.0, 1.0),
        end: glam::Vec2::new(0.0, -1.0),
        color_a: Color::rgba_u8(40, 50, 70, 255),
        color_b: Color::rgba_u8(20, 25, 35, 255),
    });
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), bg);

    // The clipped container — anything inside it is rounded-cropped.
    let mut clip_container = Container::new();
    clip_container.clip = Some(MaskShape::RoundedRect {
        rect: Rect::new(-0.75, -0.55, 1.5, 1.1),
        radius: 0.14,
    });
    let clip_id = stage
        .add_child(stage.root(), Node::Container(clip_container))
        .expect("clip container");

    // The "recording surface" — a checkered pattern that shows the
    // crop is honoring the rounded corners.
    let mut recording = Graphics::new();
    recording.fill(Fill::LinearGradient {
        start: glam::Vec2::new(-1.0, 0.0),
        end: glam::Vec2::new(1.0, 0.0),
        color_a: Color::rgba_u8(180, 200, 230, 255),
        color_b: Color::rgba_u8(255, 200, 100, 255),
    });
    recording.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(clip_id, recording);
}
