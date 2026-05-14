//! Story: convex polygon SDF primitive (M-VEC.21 / AUT-225).
//!
//! Six convex shapes that exercise the fan-triangulated polygon path —
//! the chart-side polygons (area fills, sankey ribbons, funnel bands,
//! ternary triangle, parallel-coord polylines-as-polygons) are all
//! convex by construction in v1.

use std::f32::consts::{FRAC_PI_2, TAU};

use glam::Vec2;
use wisp::application::Application;
use wisp::{Color, Fill, Graphics, Stage};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "graphics-polygon",
        category: "Graphics",
        title: "Convex polygon shapes",
        milestone: "M-VEC.21",
        writeup: include_str!("writeups/graphics_polygon.md"),
        build,
        tick: None,
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    let mut g = Graphics::new();

    let col_x = [-0.66, 0.0, 0.66];
    let row_y = [0.5, -0.5];

    // (0, 0): square — minimal 4-vertex baseline.
    g.fill(Fill::Solid(Color::rgba_u8(80, 200, 255, 255)));
    g.draw_polygon(&square_centred(col_x[0], row_y[0], 0.22));

    // (1, 0): equilateral triangle (CCW from top).
    g.fill(Fill::Solid(Color::rgba_u8(255, 200, 80, 255)));
    g.draw_polygon(&regular_ngon(col_x[1], row_y[0], 0.22, 3));

    // (2, 0): regular pentagon.
    g.fill(Fill::Solid(Color::rgba_u8(160, 100, 220, 255)));
    g.draw_polygon(&regular_ngon(col_x[2], row_y[0], 0.22, 5));

    // (0, 1): regular hexagon.
    g.fill(Fill::Solid(Color::rgba_u8(120, 220, 140, 255)));
    g.draw_polygon(&regular_ngon(col_x[0], row_y[1], 0.22, 6));

    // (1, 1): funnel-area-style trapezoid (wide bottom).
    g.fill(Fill::Solid(Color::rgba_u8(255, 100, 80, 255)));
    g.draw_polygon(&[
        Vec2::new(col_x[1] - 0.15, row_y[1] - 0.22),
        Vec2::new(col_x[1] + 0.15, row_y[1] - 0.22),
        Vec2::new(col_x[1] + 0.28, row_y[1] + 0.22),
        Vec2::new(col_x[1] - 0.28, row_y[1] + 0.22),
    ]);

    // (2, 1): elongated octagon — exercises a higher vertex count.
    g.fill(Fill::Solid(Color::rgba_u8(80, 220, 200, 255)));
    g.draw_polygon(&regular_ngon(col_x[2], row_y[1], 0.22, 8));

    let _ = stage.add_child(stage.root(), g);
}

fn square_centred(cx: f32, cy: f32, half: f32) -> [Vec2; 4] {
    [
        Vec2::new(cx - half, cy - half),
        Vec2::new(cx + half, cy - half),
        Vec2::new(cx + half, cy + half),
        Vec2::new(cx - half, cy + half),
    ]
}

#[allow(
    clippy::cast_precision_loss,
    reason = "n is a tiny ngon vertex count (<=8 in this story) — well below f32 precision"
)]
fn regular_ngon(cx: f32, cy: f32, radius: f32, n: usize) -> Vec<Vec2> {
    (0..n)
        .map(|i| {
            // Start at top (+y); fix winding so it stays CCW for the
            // renderer's `+Y` up NDC.
            let theta = FRAC_PI_2 + (i as f32) * TAU / (n as f32);
            Vec2::new(cx + radius * theta.cos(), cy + radius * theta.sin())
        })
        .collect()
}
