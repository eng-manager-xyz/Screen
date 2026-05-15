//! Standalone "Animation / Easing Functions" demo — a 4-row grid
//! of 36 mini charts, each plotting one [`Ease`] curve in its own
//! card with a moving dot riding the curve. Top-left shows a
//! `Progress: 0.00` readout. The animation cycle is 2 s of
//! progress 0 → 1 followed by a 3 s hold, then loops.
//!
//! The layout / curve / dot / label construction is pulled out of
//! `web.rs` so the native hero test in
//! `tests/render_easing_gallery_hero.rs` can render the exact same
//! scene at a frozen `progress` value.
//!
//! Color palette is a uniform-hue rainbow across the grid index so
//! the eye groups by row (curve shape) rather than by column.

use glam::Vec2;
use wisp::Color;
use wisp::Font;
use wisp::math::Rect;
use wisp::scene::{Fill, Graphics, Stroke, Text};
use wisp_animation::Ease;

/// Backdrop colour underneath the grid — dark slate so the bright
/// curves and white progress text both pop.
pub const BACKDROP: Color = Color::rgba(0.13, 0.13, 0.16, 1.0);

/// Columns in the grid. Rows are implied by `easing_table().len()
/// / COLS` rounded up.
pub const COLS: usize = 10;

const CARD_W: f32 = 0.17;
const CARD_H: f32 = 0.22;
const COL_STEP: f32 = 0.19;
const ROW_STEP: f32 = 0.36;
const GRID_TOP: f32 = 0.52;
const CARD_INSET: f32 = 0.006;
const CURVE_WIDTH: f32 = 0.0028;
const BORDER_WIDTH: f32 = 0.0028;
const DOT_HALF: f32 = 0.0058;

/// NDC per atlas-pixel for per-card labels.
///
/// At the demo's authoritative 1600×1200 render size this gives
/// ~11.5 px × 8.6 px glyphs (the 8×8 atlas stretches with the
/// canvas aspect). 12-char labels fit inside `COL_STEP` with
/// margin, which is the cap satisfied by the table.
const LABEL_CELL: f32 = 0.0018;

/// NDC per atlas-pixel for the top-left `Progress: 0.00` readout.
const PROGRESS_CELL: f32 = 0.0065;

/// Canvas pixel size used for both the live WebGPU demo and the
/// hero snapshot. 4:3 keeps glyph stretch tolerable and gives the
/// 10-column grid room for legible labels.
pub const CANVAS_W: u32 = 1600;
/// See [`CANVAS_W`].
pub const CANVAS_H: u32 = 1200;

#[allow(
    clippy::cast_precision_loss,
    reason = "COLS = 10, fits f32 mantissa losslessly"
)]
const fn grid_left() -> f32 {
    // Centred grid: shift so column 0..(COLS-1) is symmetric about x=0.
    -(COLS as f32 - 1.0) * 0.5 * COL_STEP
}

/// The 36 easing curves shown in the grid, in scan-line order.
///
/// - Row 0 — `In*` family (10 variants)
/// - Row 1 — `Out*` family (10 variants)
/// - Row 2 — `InOut*` family (10 variants)
/// - Row 3 — misc / parametric (6 variants)
///
/// The data lives in this module so both the WebGPU dispatch in
/// `web.rs` and the native hero snapshot iterate the *same* list.
#[must_use]
pub fn easing_table() -> Vec<(&'static str, Ease)> {
    vec![
        ("InQuad", Ease::InQuad),
        ("InCubic", Ease::InCubic),
        ("InQuart", Ease::InQuart),
        ("InQuint", Ease::InQuint),
        ("InSine", Ease::InSine),
        ("InExpo", Ease::InExpo),
        ("InCirc", Ease::InCirc),
        ("InBack", Ease::InBack),
        ("InElastic", Ease::InElastic),
        ("InBounce", Ease::InBounce),
        ("OutQuad", Ease::OutQuad),
        ("OutCubic", Ease::OutCubic),
        ("OutQuart", Ease::OutQuart),
        ("OutQuint", Ease::OutQuint),
        ("OutSine", Ease::OutSine),
        ("OutExpo", Ease::OutExpo),
        ("OutCirc", Ease::OutCirc),
        ("OutBack", Ease::OutBack),
        ("OutElastic", Ease::OutElastic),
        ("OutBounce", Ease::OutBounce),
        ("InOutQuad", Ease::InOutQuad),
        ("InOutCubic", Ease::InOutCubic),
        ("InOutQuart", Ease::InOutQuart),
        ("InOutQuint", Ease::InOutQuint),
        ("InOutSine", Ease::InOutSine),
        ("InOutExpo", Ease::InOutExpo),
        ("InOutCirc", Ease::InOutCirc),
        ("InOutBack", Ease::InOutBack),
        ("InOutElastic", Ease::InOutElastic),
        ("InOutBounce", Ease::InOutBounce),
        ("Linear", Ease::Linear),
        ("Steps(4)", Ease::Steps(4)),
        ("Steps(8)", Ease::Steps(8)),
        ("ThereBack", Ease::ThereAndBack),
        ("Bezier IO", Ease::CubicBezier(0.65, 0.0, 0.35, 1.0)),
        ("Bezier OB", Ease::CubicBezier(0.34, 1.56, 0.64, 1.0)),
    ]
}

/// Card axis-aligned rectangle in NDC for the easing at `idx`.
#[must_use]
pub fn card_rect(idx: usize) -> Rect {
    #[allow(clippy::cast_precision_loss, reason = "idx < 64; fits f32 losslessly")]
    let row = (idx / COLS) as f32;
    #[allow(clippy::cast_precision_loss, reason = "idx < 64; fits f32 losslessly")]
    let col = (idx % COLS) as f32;
    let cx = grid_left() + col * COL_STEP;
    let cy = GRID_TOP - row * ROW_STEP;
    Rect::from_min_max(
        Vec2::new(cx - CARD_W * 0.5, cy - CARD_H * 0.5),
        Vec2::new(cx + CARD_W * 0.5, cy + CARD_H * 0.5),
    )
}

/// Per-card rainbow accent colour. `value_boost` brightens the
/// dot vs. the curve so foreground beats background at the same
/// hue.
#[must_use]
pub fn card_color(idx: usize, total: usize, value_boost: f32) -> Color {
    #[allow(clippy::cast_precision_loss, reason = "total + idx are small ints")]
    let hue = (idx as f32) / (total as f32);
    hsv(hue, 0.7, (0.85 + value_boost).clamp(0.0, 1.0))
}

#[must_use]
#[allow(
    clippy::many_single_char_names,
    reason = "h/s/v inputs and r/g/b outputs are the standard HSV-to-RGB names; renaming would obscure the maths"
)]
fn hsv(h: f32, s: f32, v: f32) -> Color {
    let h6 = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - (h6.rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "h6 in [0, 6); truncates to 0..=5"
    )]
    let segment = h6 as u32;
    let (r, g, b) = match segment {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::rgba(r + m, g + m, b + m, 1.0)
}

/// Build the static layer — full-viewport backdrop + 36 bordered
/// cards each with its eased curve drawn as a 64-segment polyline.
/// Constant per session; the dot + progress text layers refresh
/// each frame.
#[must_use]
pub fn build_static_layer() -> Graphics {
    let table = easing_table();
    let total = table.len();
    let mut g = Graphics::new();

    // Backdrop first so insertion order keeps it underneath.
    g.fill(Fill::Solid(BACKDROP));
    g.stroke(None);
    g.draw_rect(Rect::from_min_max(
        Vec2::new(-1.0, -1.0),
        Vec2::new(1.0, 1.0),
    ));

    for (idx, (_, ease)) in table.iter().enumerate() {
        let rect = card_rect(idx);
        let color = card_color(idx, total, 0.0);

        g.fill(Fill::Solid(Color::rgba(0.0, 0.0, 0.0, 0.0)));
        g.stroke(Some(Stroke::new(BORDER_WIDTH, color)));
        g.draw_rect(rect);

        g.stroke(None);
        g.fill(Fill::Solid(card_color(idx, total, 0.05)));
        let lb = rect.min + Vec2::splat(CARD_INSET);
        let ub = rect.max() - Vec2::splat(CARD_INSET);
        draw_curve(&mut g, lb, ub, *ease);
    }
    g
}

fn draw_curve(g: &mut Graphics, lb: Vec2, ub: Vec2, ease: Ease) {
    const SAMPLES: u32 = 64;
    let mut prev: Option<Vec2> = None;
    for i in 0..=SAMPLES {
        #[allow(clippy::cast_precision_loss, reason = "i <= 64")]
        let t = i as f32 / SAMPLES as f32;
        let y = ease.eval(t).clamp(-0.4, 1.4);
        let p = Vec2::new(lb.x + t * (ub.x - lb.x), lb.y + y * (ub.y - lb.y));
        if let Some(prev_p) = prev {
            g.draw_line(prev_p, p, CURVE_WIDTH);
        }
        prev = Some(p);
    }
}

/// Build the dot layer for one frame: a small square per card at
/// the current `(progress, ease(progress))` point on its curve.
#[must_use]
pub fn build_dot_layer(progress: f32) -> Graphics {
    let table = easing_table();
    let total = table.len();
    let progress = progress.clamp(0.0, 1.0);
    let mut g = Graphics::new();
    g.stroke(None);
    for (idx, (_, ease)) in table.iter().enumerate() {
        let rect = card_rect(idx);
        let lb = rect.min + Vec2::splat(CARD_INSET);
        let ub = rect.max() - Vec2::splat(CARD_INSET);
        let color = card_color(idx, total, 0.1);
        g.fill(Fill::Solid(color));
        let y = ease.eval(progress).clamp(-0.4, 1.4);
        let cx = lb.x + progress * (ub.x - lb.x);
        let cy = lb.y + y * (ub.y - lb.y);
        g.draw_rect(Rect::from_min_max(
            Vec2::new(cx - DOT_HALF, cy - DOT_HALF),
            Vec2::new(cx + DOT_HALF, cy + DOT_HALF),
        ));
    }
    g
}

/// Build per-card labels centred just below each card. Run once
/// — labels don't change frame to frame.
#[must_use]
pub fn build_labels(font: &Font) -> Vec<Text> {
    let table = easing_table();
    let total = table.len();
    let glyph_w = LABEL_CELL * 8.0;
    table
        .into_iter()
        .enumerate()
        .map(|(idx, (name, _))| {
            let rect = card_rect(idx);
            let color = card_color(idx, total, 0.05);
            let mut text = Text::new(font.clone(), name).with_cell_size(LABEL_CELL);
            text.color = color;
            #[allow(
                clippy::cast_precision_loss,
                reason = "label length capped by table contents (< 20)"
            )]
            let label_w = glyph_w * name.chars().count() as f32;
            let centre_x = (rect.min.x + rect.max().x) * 0.5;
            text.container.transform.position =
                Vec2::new(centre_x - label_w * 0.5, rect.min.y - LABEL_CELL * 2.5);
            text
        })
        .collect()
}

/// Build the top-left "Progress: 0.00" readout text for one frame.
#[must_use]
pub fn build_progress_label(font: &Font, progress: f32) -> Text {
    let progress = progress.clamp(0.0, 1.0);
    let mut text =
        Text::new(font.clone(), format!("Progress: {progress:.2}")).with_cell_size(PROGRESS_CELL);
    text.color = Color::WHITE;
    text.container.transform.position = Vec2::new(-0.93, 0.92);
    text
}

/// Convert the 5 s loop wall-clock `elapsed` to the current
/// `progress ∈ [0, 1]`. The first 2 s ramps linearly; the next 3 s
/// hold at 1.0 before the wall clock wraps.
#[must_use]
pub fn progress_for_elapsed(elapsed: std::time::Duration) -> f32 {
    const CYCLE_S: f32 = 5.0;
    const ANIM_S: f32 = 2.0;
    let t = elapsed.as_secs_f32().rem_euclid(CYCLE_S);
    if t < ANIM_S {
        (t / ANIM_S).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn easing_table_is_36_entries() {
        let table = easing_table();
        assert_eq!(table.len(), 36);
        // Sanity: column count divides 30 (rows 0..=2 are full).
        let full_rows = 3;
        assert_eq!(full_rows * COLS, 30);
    }

    #[test]
    fn card_rects_are_within_viewport() {
        let table = easing_table();
        for idx in 0..table.len() {
            let r = card_rect(idx);
            assert!(r.min.x > -1.0 && r.max().x < 1.0, "idx={idx} x out of NDC");
            // Labels sit ~0.018 below; rect bottoms cluster around -0.78 in row 3.
            assert!(
                r.min.y > -0.95 && r.max().y < 0.85,
                "idx={idx} y out of NDC"
            );
        }
    }

    #[test]
    fn progress_ramps_linearly_then_holds() {
        assert!((progress_for_elapsed(Duration::ZERO)).abs() < 1e-5);
        assert!((progress_for_elapsed(Duration::from_secs_f32(1.0)) - 0.5).abs() < 1e-5);
        assert!((progress_for_elapsed(Duration::from_secs_f32(2.0)) - 1.0).abs() < 1e-5);
        // Hold phase.
        assert!((progress_for_elapsed(Duration::from_secs_f32(3.5)) - 1.0).abs() < 1e-5);
        assert!((progress_for_elapsed(Duration::from_secs_f32(4.9)) - 1.0).abs() < 1e-5);
        // Wraps at 5s — back to 0.
        assert!((progress_for_elapsed(Duration::from_secs_f32(5.0))).abs() < 1e-5);
        assert!((progress_for_elapsed(Duration::from_secs_f32(6.0)) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn dot_layer_has_one_primitive_per_card() {
        let g = build_dot_layer(0.5);
        // 36 cards → 36 rect primitives.
        assert_eq!(g.primitive_count(), 36);
    }

    #[test]
    fn static_layer_has_backdrop_plus_two_per_card() {
        let g = build_static_layer();
        // 1 backdrop + 36 borders + 36 × 64 line segments = 1 + 36 + 2_304 = 2_341.
        // (One primitive per segment in our `draw_line` path.)
        assert_eq!(g.primitive_count(), 1 + 36 + 36 * 64);
    }
}
