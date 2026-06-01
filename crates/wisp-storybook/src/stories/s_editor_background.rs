//! Story: editor background framing (ED.18 / M-EDIT).
//!
//! The cinematic "framed screen" look: the recording sits on a colored
//! backdrop, lifted off it by a uniform padding and clipped to a
//! rounded-corner window. This mirrors the editor's real compose — a
//! full-NDC gradient `Graphics` backdrop drawn behind a rounded-rect-clipped
//! "screen" — exactly the structure `EditorPreview::set_background` builds in
//! `wisp::RecordingScene` (backdrop Phase 1, clipped screen Phase 2).
//!
//! Single-bind-group clip + graphics (no blur), so it runs on every CI OS —
//! NOT in `LAVAPIPE_INCOMPATIBLE`.

use wisp::application::Application;
use wisp::math::Rect;
use wisp::{Color, Container, Fill, Graphics, MaskShape, Node, Stage, Stroke};

pub fn story() -> crate::story::Story {
    crate::story::Story {
        id: "editor-background-framing",
        category: "Editor",
        title: "Background framing — backdrop · padding · rounded corners",
        milestone: "ED.18",
        writeup: include_str!("writeups/editor_background.md"),
        build,
        tick: None,
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    // Backdrop: the default "Aurora" warm→cool diagonal gradient, filling the
    // whole canvas. In the real compose this is the non-clipped Phase-1 node.
    let mut backdrop = Graphics::new();
    let theta = 135.0_f32.to_radians();
    let dir = glam::Vec2::new(theta.cos(), theta.sin());
    backdrop.fill(Fill::LinearGradient {
        start: -dir,
        end: dir,
        color_a: Color::rgb_u8(255, 138, 128),
        color_b: Color::rgb_u8(40, 53, 147),
    });
    backdrop.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), backdrop);

    // Drop shadow (ED.18): a dark rounded-rect the shape of the frame window,
    // offset down-right, drawn behind the screen so the offset sliver reads as
    // a shadow lifting the screen off the backdrop. Phase-1 (unclipped),
    // composited after the backdrop and under the screen.
    let mut shadow = Graphics::new();
    shadow.fill(Fill::Solid(Color::rgba(0.0, 0.0, 0.0, 0.45)));
    shadow.draw_rounded_rect(Rect::new(-0.84 + 0.03, -0.78 - 0.05, 1.68, 1.56), 0.06);
    let _ = stage.add_child(stage.root(), shadow);

    // The framed screen: a rounded-rect window, inset by the padding. In the
    // real compose this is the clipped screen sprite (dispatched Phase 2, so
    // it composites over the backdrop). Defaults (padding 64, corner 14 on a
    // 1920-wide canvas) → window ≈ [-0.93, 0.93] × [-0.88, 0.88], radius
    // ≈ 0.015; widened a touch here so the inset reads at thumbnail size.
    let mut window = Container::new();
    window.clip = Some(MaskShape::rounded_rect(
        Rect::new(-0.84, -0.78, 1.68, 1.56),
        0.06,
    ));
    let window_id = stage
        .add_child(stage.root(), Node::Container(window))
        .expect("window container");

    // Light "screen" content so the colored backdrop margin + rounded corners
    // read clearly (the storybook backdrop-visibility convention: light
    // content over a colored frame). A pale vertical gradient with a single
    // accent bar stands in for a screenshot.
    let mut screen = Graphics::new();
    screen.fill(Fill::LinearGradient {
        start: glam::Vec2::new(0.0, 1.0),
        end: glam::Vec2::new(0.0, -1.0),
        color_a: Color::rgb_u8(248, 250, 252),
        color_b: Color::rgb_u8(214, 222, 232),
    });
    screen.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    // Accent "title bar" so the screen reads as content, not a flat panel.
    screen.fill(Fill::Solid(Color::rgb_u8(60, 110, 220)));
    screen.draw_rect(Rect::new(-0.84, 0.58, 1.68, 0.2));
    let _ = stage.add_child(window_id, screen);

    // Inset border (ED.18): a rounded-rect stroke tracing the frame window,
    // over the screen. A full-NDC clip forces Phase-2 dispatch so it composites
    // on top (mirrors RecordingScene::set_frame_border).
    let mut border = Graphics::new();
    border.container.clip = Some(MaskShape::rect(Rect::new(-1.0, -1.0, 2.0, 2.0)));
    border.fill(Fill::Solid(Color::rgba(0.0, 0.0, 0.0, 0.0)));
    border.stroke(Some(Stroke::new(0.012, Color::rgba(1.0, 1.0, 1.0, 0.85))));
    border.draw_rounded_rect(Rect::new(-0.84, -0.78, 1.68, 1.56), 0.06);
    let _ = stage.add_child(stage.root(), border);
}
