//! Story: animated click-ripple — outlined ellipse animated by `tick` (M0.13).

use glam::Vec2;
use wisp::application::Application;
use wisp::{Color, Fill, Graphics, Stage, Stroke};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "graphics-ellipse",
        category: "Graphics",
        title: "Animated click ripple",
        milestone: "M0.13",
        writeup: include_str!("writeups/graphics_ellipse.md"),
        build,
        tick: Some(tick),
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    let g = Graphics::new();
    let _ = stage.add_child(stage.root(), g);
}

fn tick(stage: &mut Stage, t: f32) {
    let root_id = stage.root();
    let child_ids: Vec<_> = stage
        .get(root_id)
        .map(|n| n.container().children().collect())
        .unwrap_or_default();
    let Some(graphics_id) = child_ids.first().copied() else {
        return;
    };
    let Some(node) = stage.get_mut(graphics_id) else {
        return;
    };
    let wisp::Node::Graphics(g) = node else {
        return;
    };

    *g = Graphics::new();

    // Three ripples staggered in time — like multiple clicks.
    for i in 0u8..3 {
        let i_f = f32::from(i);
        let offset = i_f * 0.6;
        let phase = (t - offset).max(0.0) % 2.5;
        let progress = (phase / 2.0).min(1.0);
        let alpha = (1.0 - progress).max(0.0);
        let radius = 0.15 + progress * 0.6;
        let x = -0.6 + i_f * 0.6;

        g.fill(Fill::Solid(Color::rgba(1.0, 1.0, 1.0, alpha * 0.25)));
        g.stroke(Some(Stroke::new(
            0.025,
            Color::rgba(1.0, 1.0, 1.0, alpha * 0.9),
        )));
        g.draw_ellipse(Vec2::new(x, 0.0), Vec2::splat(radius));
    }
}
