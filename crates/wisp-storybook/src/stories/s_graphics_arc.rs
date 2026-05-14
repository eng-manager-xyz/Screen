//! Story: arc + annular sector SDF primitives (M-VEC.20 / AUT-224).
//!
//! Six samples in a 3×2 grid:
//!   * filled disc (full angular span, `r_inner = 0`),
//!   * pie slice (90° wedge),
//!   * full donut (`r_inner > 0`, full angular span),
//!   * annular sector (partial donut),
//!   * thin stroked arc via `draw_arc` (chart gauge needle / tick),
//!   * semicircular gauge-style arc with thicker stroke.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

use glam::Vec2;
use wisp::application::Application;
use wisp::{Color, Fill, Graphics, Stage};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "graphics-arc",
        category: "Graphics",
        title: "Arc + annular sector",
        milestone: "M-VEC.20",
        writeup: include_str!("writeups/graphics_arc.md"),
        build,
        tick: None,
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    let mut g = Graphics::new();

    // Layout: 3 columns × 2 rows in NDC [-1, 1].
    let col_x = [-0.66, 0.0, 0.66];
    let row_y = [0.5, -0.5];
    let radius = 0.22;

    // (0, 0): full filled disc — pie slice with span = 2π.
    g.fill(Fill::Solid(Color::rgba_u8(80, 200, 255, 255)));
    g.draw_annular_sector(Vec2::new(col_x[0], row_y[0]), 0.0, radius, 0.0, TAU);

    // (1, 0): 90° pie slice (wedge from 0 to π/2).
    g.fill(Fill::Solid(Color::rgba_u8(255, 200, 80, 255)));
    g.draw_annular_sector(Vec2::new(col_x[1], row_y[0]), 0.0, radius, 0.0, FRAC_PI_2);

    // (2, 0): full donut (r_inner > 0, full span).
    g.fill(Fill::Solid(Color::rgba_u8(160, 100, 220, 255)));
    g.draw_annular_sector(Vec2::new(col_x[2], row_y[0]), 0.12, radius, 0.0, TAU);

    // (0, 1): annular sector — partial donut (≈ 270° span).
    g.fill(Fill::Solid(Color::rgba_u8(120, 220, 140, 255)));
    g.draw_annular_sector(
        Vec2::new(col_x[0], row_y[1]),
        0.12,
        radius,
        FRAC_PI_4,
        FRAC_PI_4 + TAU * 0.75,
    );

    // (1, 1): thin stroked arc — gauge needle band style.
    g.fill(Fill::Solid(Color::rgba_u8(255, 100, 80, 255)));
    g.draw_arc(
        Vec2::new(col_x[1], row_y[1]),
        radius,
        -FRAC_PI_4,
        FRAC_PI_4,
        0.025,
    );

    // (2, 1): semicircular gauge — half circle with thick stroke.
    g.fill(Fill::Solid(Color::rgba_u8(80, 220, 200, 255)));
    g.draw_arc(Vec2::new(col_x[2], row_y[1]), radius, PI, TAU, 0.05);

    let _ = stage.add_child(stage.root(), g);
}
