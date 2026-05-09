//! Story: bitmap text rendering — Hello world + multi-line keyboard chip (M0.15).

use glam::Vec2;
use wisp::application::Application;
use wisp::{Color, Font, Stage, Text};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "text-bitmap",
        category: "Scene Graph",
        title: "Bitmap text",
        milestone: "M0.15",
        writeup: include_str!("writeups/text.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let font = Font::bitmap_8x8(app);

    // Big "Hello, world!" near the top.
    let mut hello = Text::new(font.clone(), "Hello, world!")
        .with_cell_size(0.025)
        .with_color(Color::rgba_u8(240, 240, 250, 255));
    hello.container.transform.position = Vec2::new(-0.7, 0.3);
    let _ = stage.add_child(stage.root(), hello);

    // Multi-line keyboard chip below.
    let mut chip = Text::new(font, "Cmd+Shift+.\nQuick action")
        .with_cell_size(0.018)
        .with_color(Color::rgba_u8(160, 220, 255, 255));
    chip.container.transform.position = Vec2::new(-0.45, -0.1);
    let _ = stage.add_child(stage.root(), chip);
}
