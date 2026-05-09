//! Story: linear + radial gradient fills (M0.14).

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics, Stage};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "graphics-gradients",
        category: "Graphics",
        title: "Gradient fills",
        milestone: "M0.14",
        writeup: include_str!("writeups/graphics_gradients.md"),
        build,
        tick: None,
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    let mut g = Graphics::new();

    // Linear gradient — left side of the canvas.
    g.fill(Fill::LinearGradient {
        start: Vec2::new(0.0, 0.4),
        end: Vec2::new(0.0, -0.4),
        color_a: Color::rgba_u8(255, 110, 80, 255),
        color_b: Color::rgba_u8(80, 110, 255, 255),
    });
    g.draw_rounded_rect(Rect::new(-0.85, -0.4, 0.7, 0.8), 0.08);

    // Radial gradient — right side of the canvas, centered ellipse.
    g.fill(Fill::RadialGradient {
        center: Vec2::ZERO,
        radius: 0.4,
        color_a: Color::rgba_u8(255, 240, 200, 255),
        color_b: Color::rgba_u8(40, 30, 80, 255),
    });
    g.draw_ellipse(Vec2::new(0.5, 0.0), Vec2::new(0.4, 0.4));

    let _ = stage.add_child(stage.root(), g);
}
