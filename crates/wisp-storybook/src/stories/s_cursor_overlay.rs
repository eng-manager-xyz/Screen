//! Story: editor cursor overlay (ED.19 / M-EDIT).
//!
//! The cursor is the only performer on a screen recording's stage, so the
//! editor grooms it: a scaled, dark-outlined white pointer with an expanding
//! click ripple, drawn as a `wisp` `Graphics` overlay over the framed screen.
//! This mirrors `wisp::RecordingScene::set_cursor` — the pointer triangle +
//! ripple discs the cursor node draws each frame at the captured position
//! (mapped through the same transform as the screen, so it rides the zoom).
//!
//! Single-bind-group graphics (no blur) → runs on every CI OS; NOT in
//! `LAVAPIPE_INCOMPATIBLE`.

use wisp::application::Application;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics, Stage};

pub fn story() -> crate::story::Story {
    crate::story::Story {
        id: "editor-cursor-overlay",
        category: "Editor",
        title: "Cursor overlay — pointer + click ripple",
        milestone: "ED.19",
        writeup: include_str!("writeups/editor_cursor.md"),
        build,
        tick: None,
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    // A light "screen" so the cursor + ripple read clearly (the storybook
    // light-backdrop convention for alpha-blended overlays).
    let mut screen = Graphics::new();
    screen.fill(Fill::LinearGradient {
        start: glam::Vec2::new(0.0, 1.0),
        end: glam::Vec2::new(0.0, -1.0),
        color_a: Color::rgb_u8(244, 247, 250),
        color_b: Color::rgb_u8(206, 216, 228),
    });
    screen.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), screen);

    // The cursor overlay node: a click ripple under a dark-outlined white
    // pointer, at the click point.
    let point = glam::Vec2::new(0.08, 0.04);
    let half = 0.13;
    let mut overlay = Graphics::new();
    // Expanding click ripple — two fading discs (mid-age + young).
    overlay.fill(Fill::Solid(Color::rgba(0.25, 0.5, 0.95, 0.18)));
    overlay.draw_ellipse(point, glam::Vec2::splat(half * 2.6));
    overlay.fill(Fill::Solid(Color::rgba(0.25, 0.5, 0.95, 0.30)));
    overlay.draw_ellipse(point, glam::Vec2::splat(half * 1.6));
    // Pointer triangle (CCW), dark outline behind a white fill — same shape
    // RecordingScene::set_cursor draws.
    let arrow = |s: f32| {
        let scale = half * s;
        [
            point + glam::Vec2::new(0.0, 0.0) * scale,
            point + glam::Vec2::new(0.0, -1.7) * scale,
            point + glam::Vec2::new(1.15, -1.05) * scale,
        ]
    };
    overlay.fill(Fill::Solid(Color::rgb_u8(20, 20, 20)));
    overlay.draw_polygon(&arrow(1.3));
    overlay.fill(Fill::Solid(Color::WHITE));
    overlay.draw_polygon(&arrow(1.0));
    let _ = stage.add_child(stage.root(), overlay);
}
