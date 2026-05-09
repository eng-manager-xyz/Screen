//! Story: rounded-rect graphics primitive with SDF anti-aliasing (M0.12).

use wisp::application::Application;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics, Stage, Stroke};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "graphics-rounded",
        category: "Graphics",
        title: "Rounded rect with stroke",
        milestone: "M0.12 / M0.13",
        writeup: include_str!("writeups/graphics_rounded.md"),
        build,
        tick: None,
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    let mut g = Graphics::new();

    // Filled rounded rect (no stroke).
    g.fill(Fill::Solid(Color::rgba_u8(80, 200, 255, 255)));
    g.draw_rounded_rect(Rect::new(-0.7, -0.6, 0.6, 0.6), 0.18);

    // Outlined rounded rect — fill + stroke.
    g.fill(Fill::Solid(Color::rgba_u8(255, 100, 80, 200)));
    g.stroke(Some(Stroke::new(0.04, Color::rgba_u8(40, 20, 30, 255))));
    g.draw_rounded_rect(Rect::new(0.1, -0.6, 0.6, 0.6), 0.12);

    // Sharp rect (radius=0).
    g.stroke(None);
    g.fill(Fill::Solid(Color::rgba_u8(140, 220, 140, 255)));
    g.draw_rect(Rect::new(-0.3, 0.2, 0.6, 0.4));

    let _ = stage.add_child(stage.root(), g);
}
