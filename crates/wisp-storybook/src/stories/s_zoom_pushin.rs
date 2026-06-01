//! Story: editor zoom push-in, animated (ED.16 / M-EDIT).
//!
//! The "rostrum push-in" — the single gesture that reads as *cinematic* in a
//! screen recording. A mock app card stands in for the recorded screen; the
//! camera eases in toward a focal button, holds on it, then eases back out —
//! the three-phase profile [`edit::zoom_anim::zoom_at`] computes for real.
//! Here it is reproduced inline (an eased ramp-in → hold → ramp-out) so the
//! story stays a pure `wisp` demo, scaling the content node about the focal
//! point exactly as `EditorPreview::render_framed` pins it in the editor:
//! `position = focal · (1 − z)` keeps the target glued in place while only the
//! scale animates.
//!
//! Single-bind-group graphics (no blur) → runs on every CI OS; NOT in
//! `LAVAPIPE_INCOMPATIBLE`. Animated (`tick: Some`) → exported to MP4 by
//! `wisp-export-animated` for the ED.16 chapter.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics, Node, Stage};

use crate::story::Story;

/// Focal point of the push-in, in NDC — the centre of the accent button the
/// camera tightens toward. Shared by `build` (where the button is drawn) and
/// `tick` (where the zoom is pinned).
const FOCAL: Vec2 = Vec2::new(0.40, -0.30);
/// Peak zoom factor at the hold.
const AMOUNT: f32 = 2.4;

pub fn story() -> Story {
    Story {
        id: "editor-zoom-pushin",
        category: "Editor",
        title: "Zoom push-in — the rostrum move",
        milestone: "ED.16",
        writeup: include_str!("writeups/editor_zoom.md"),
        build,
        tick: Some(tick),
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    let mut g = Graphics::new();

    // Desktop backdrop — a full-NDC cool gradient. Because the content scales
    // about the focal point at z ≥ 1, this always covers the frame (no black
    // edges creep in as the camera pushes in).
    g.fill(Fill::LinearGradient {
        start: Vec2::new(0.0, 1.0),
        end: Vec2::new(0.0, -1.0),
        color_a: Color::rgb_u8(70, 82, 104),
        color_b: Color::rgb_u8(40, 47, 62),
    });
    g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));

    // App window card.
    g.fill(Fill::Solid(Color::rgb_u8(248, 249, 251)));
    g.draw_rounded_rect(Rect::new(-0.82, -0.72, 1.64, 1.44), 0.07);
    // Title bar.
    g.fill(Fill::Solid(Color::rgb_u8(228, 232, 238)));
    g.draw_rect(Rect::new(-0.80, 0.52, 1.60, 0.18));
    // Traffic-light dots.
    for (x, c) in [
        (-0.720, Color::rgb_u8(237, 106, 94)),
        (-0.645, Color::rgb_u8(244, 191, 79)),
        (-0.570, Color::rgb_u8(98, 197, 84)),
    ] {
        g.fill(Fill::Solid(c));
        g.draw_ellipse(Vec2::new(x, 0.61), Vec2::splat(0.022));
    }

    // Heading + placeholder text rows.
    g.fill(Fill::Solid(Color::rgb_u8(78, 88, 104)));
    g.draw_rounded_rect(Rect::new(-0.66, 0.34, 0.62, 0.09), 0.03);
    g.fill(Fill::Solid(Color::rgb_u8(199, 205, 214)));
    for (y, w) in [(0.20, 1.18), (0.07, 0.96), (-0.06, 1.10)] {
        g.draw_rounded_rect(Rect::new(-0.66, y, w, 0.055), 0.025);
    }

    // The focal accent button — the thing the camera pushes into. Drawn last
    // so it sits above the card.
    let bw = 0.34;
    let bh = 0.15;
    g.fill(Fill::Solid(Color::rgb_u8(56, 120, 242)));
    g.draw_rounded_rect(
        Rect::new(FOCAL.x - bw / 2.0, FOCAL.y - bh / 2.0, bw, bh),
        0.045,
    );
    // Button label bar (a light pill inside the button).
    g.fill(Fill::Solid(Color::rgb_u8(232, 240, 255)));
    g.draw_rounded_rect(
        Rect::new(FOCAL.x - 0.10, FOCAL.y - 0.022, 0.20, 0.044),
        0.022,
    );

    let _ = stage.add_child(stage.root(), g);
}

/// Eased zoom factor at story time `t` (seconds, `0..3`): a cubic ramp-in to
/// [`AMOUNT`], a flat hold, and a symmetric ramp-out — the same three-phase
/// shape `edit::zoom_anim::zoom_at` produces, reproduced here without the dep.
fn zoom_scale(t: f32) -> f32 {
    // In-out cubic, pinned f(0)=0, f(1)=1.
    let ease = |p: f32| {
        if p < 0.5 {
            4.0 * p * p * p
        } else {
            1.0 - (-2.0 * p + 2.0).powi(3) / 2.0
        }
    };
    if t < 0.9 {
        1.0 + (AMOUNT - 1.0) * ease((t / 0.9).clamp(0.0, 1.0))
    } else if t < 2.1 {
        AMOUNT
    } else {
        AMOUNT - (AMOUNT - 1.0) * ease(((t - 2.1) / 0.9).clamp(0.0, 1.0))
    }
}

fn tick(stage: &mut Stage, t: f32) {
    let z = zoom_scale(t);
    let child = stage
        .get(stage.root())
        .and_then(|n| n.container().children().next());
    if let Some(id) = child
        && let Some(Node::Graphics(g)) = stage.get_mut(id)
    {
        // Scale about the focal point: world = position + scale·local, so
        // pinning `FOCAL` in place needs position = FOCAL·(1 − z). Identical
        // to the editor's `render_framed` focal-pin math.
        g.container.transform.scale = Vec2::splat(z);
        g.container.transform.position = FOCAL * (1.0 - z);
    }
}
