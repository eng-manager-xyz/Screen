//! Trellis / small multiples — tile a grid of mini sub-plots.
//!
//! v1 takes pre-built per-cell `wisp::Graphics` and arranges
//! them on a regular grid. The caller produces each cell with
//! whatever chart type fits — bar, scatter, line, etc.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One cell of a trellis — a label + a pre-built `Graphics`.
#[derive(Clone, Debug)]
pub struct TrellisCell {
    /// Title displayed on top of the cell.
    pub label: String,
    /// Pre-built scene-graph subtree. The caller is responsible
    /// for sizing this to roughly `viewport_px / (cols, rows)`
    /// since the trellis just translates the entire subtree to
    /// its cell origin.
    pub graphics: Graphics,
}

impl TrellisCell {
    /// Construct from a label + pre-built `Graphics`.
    #[must_use]
    pub fn new(label: impl Into<String>, graphics: Graphics) -> Self {
        Self {
            label: label.into(),
            graphics,
        }
    }
}

/// Trellis chart — `rows × cols` grid of sub-plots.
#[derive(Clone, Debug)]
pub struct Trellis {
    /// Row count.
    pub rows: usize,
    /// Column count.
    pub cols: usize,
    /// Cell graphics in row-major order. Length must equal
    /// `rows × cols` for the grid to fill out — missing entries
    /// render as blank cells.
    pub cells: Vec<TrellisCell>,
}

impl Trellis {
    /// Construct from a row/col count + cell list.
    #[must_use]
    pub const fn new(rows: usize, cols: usize, cells: Vec<TrellisCell>) -> Self {
        Self { rows, cols, cells }
    }

    /// Emit each cell's graphics positioned in its grid slot.
    /// Returns a single top-level `Graphics` containing cell
    /// border rects; sub-plot Graphics nodes are returned via
    /// [`Self::positioned_cells`] so the caller can add them
    /// individually to the stage (each as its own
    /// `Graphics` node with a translated transform).
    #[must_use]
    #[allow(
        clippy::many_single_char_names,
        reason = "geometry code with conventional short names — a/b for line endpoints, x/y for pixel coords."
    )]
    pub fn emit_grid_borders(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.rows == 0 || self.cols == 0 {
            return g;
        }
        let pad = 16.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let cell_w = (plot_right - plot_left) / usize_to_f32(self.cols);
        let cell_h = (plot_bottom - plot_top) / usize_to_f32(self.rows);
        let border_color = chart_to_wisp(theme.plot.gridline_minor.color);
        let border_w_ndc = 1.0 / viewport_px.y * 2.0;
        for row in 0..self.rows {
            for col in 0..self.cols {
                let x = plot_left + usize_to_f32(col) * cell_w;
                let y = plot_top + usize_to_f32(row) * cell_h;
                g.fill(Fill::Solid(border_color));
                // Top edge.
                let a = pixel_to_ndc(Vec2::new(x, y), viewport_px);
                let b = pixel_to_ndc(Vec2::new(x + cell_w, y), viewport_px);
                g.draw_line(a, b, border_w_ndc);
                // Left edge.
                let a = pixel_to_ndc(Vec2::new(x, y), viewport_px);
                let b = pixel_to_ndc(Vec2::new(x, y + cell_h), viewport_px);
                g.draw_line(a, b, border_w_ndc);
            }
        }
        // Right + bottom outer borders.
        let a = pixel_to_ndc(Vec2::new(plot_right, plot_top), viewport_px);
        let b = pixel_to_ndc(Vec2::new(plot_right, plot_bottom), viewport_px);
        g.draw_line(a, b, border_w_ndc);
        let a = pixel_to_ndc(Vec2::new(plot_left, plot_bottom), viewport_px);
        let b = pixel_to_ndc(Vec2::new(plot_right, plot_bottom), viewport_px);
        g.draw_line(a, b, border_w_ndc);
        g
    }

    /// Position each cell's `Graphics` into its grid slot by
    /// applying a translation transform. The returned list can
    /// be added to the scene stage one node at a time.
    ///
    /// Cell graphics are expected to have been emitted using
    /// the cell's *own* viewport size (i.e. the caller builds a
    /// fixture sized to `cell_viewport_px()` first, then passes
    /// that `Graphics` into [`TrellisCell`]).
    #[must_use]
    #[allow(
        clippy::similar_names,
        reason = "centre_x_ndc / centre_y_ndc are orthogonal axes — the pair is the conventional naming."
    )]
    pub fn positioned_cells(&self, viewport_px: Vec2) -> Vec<Graphics> {
        if self.rows == 0 || self.cols == 0 {
            return Vec::new();
        }
        let pad = 16.0_f32;
        let cell_w = (viewport_px.x - pad * 2.0) / usize_to_f32(self.cols);
        let cell_h = (viewport_px.y - pad * 2.0) / usize_to_f32(self.rows);
        let mut out = Vec::with_capacity(self.cells.len());
        for (idx, cell) in self.cells.iter().enumerate() {
            let row = idx / self.cols;
            let col = idx % self.cols;
            let px = pad + usize_to_f32(col) * cell_w + cell_w * 0.5;
            let py = pad + usize_to_f32(row) * cell_h + cell_h * 0.5;
            let centre_x_ndc = px / viewport_px.x * 2.0 - 1.0;
            let centre_y_ndc = 1.0 - py / viewport_px.y * 2.0;
            // Each cell's Graphics is built against
            // `cell_viewport_px` so its NDC range is [-1, 1]
            // mapped onto the cell. We scale + translate.
            let mut g = cell.graphics.clone();
            g.container.transform.position = Vec2::new(centre_x_ndc, centre_y_ndc);
            g.container.transform.scale = Vec2::new(cell_w / viewport_px.x, cell_h / viewport_px.y);
            out.push(g);
        }
        out
    }

    /// Pixel dimensions of a single trellis cell when laid out
    /// at `viewport_px`. Use this to size per-cell fixtures.
    #[must_use]
    pub fn cell_viewport_px(&self, viewport_px: Vec2) -> Vec2 {
        if self.rows == 0 || self.cols == 0 {
            return Vec2::ZERO;
        }
        let pad = 16.0_f32;
        Vec2::new(
            (viewport_px.x - pad * 2.0) / usize_to_f32(self.cols),
            (viewport_px.y - pad * 2.0) / usize_to_f32(self.rows),
        )
    }
}

fn pixel_to_ndc(p: Vec2, viewport_px: Vec2) -> Vec2 {
    Vec2::new(
        p.x / viewport_px.x * 2.0 - 1.0,
        1.0 - p.y / viewport_px.y * 2.0,
    )
}

fn chart_to_wisp(c: ChartColor) -> wisp::Color {
    wisp::Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn usize_to_f32(v: usize) -> f32 {
    #[allow(clippy::cast_precision_loss, reason = "trellis dims ≤ ~20 in practice")]
    {
        v as f32
    }
}

// `Rect` is imported only so callers can match the trellis's
// per-cell pixel geometry to their fixtures.
#[allow(dead_code)]
const _RECT_ANCHOR: Option<Rect> = None;

#[cfg(test)]
mod tests {
    use super::*;
    use wisp::Graphics;

    fn cell(label: &str) -> TrellisCell {
        let mut g = Graphics::new();
        g.fill(Fill::Solid(wisp::Color::rgba(0.5, 0.5, 0.5, 1.0)));
        g.draw_rect(Rect::new(-0.5, -0.5, 1.0, 1.0));
        TrellisCell::new(label, g)
    }

    #[test]
    fn trellis_grid_borders_count_matches_rows_cols() {
        let t = Trellis::new(
            2,
            3,
            vec![
                cell("a"),
                cell("b"),
                cell("c"),
                cell("d"),
                cell("e"),
                cell("f"),
            ],
        );
        let theme = Theme::light();
        let g = t.emit_grid_borders(&theme, Vec2::new(480.0, 320.0));
        // 2 lines per cell (top + left) × 6 cells + 2 outer = 14.
        assert_eq!(g.primitive_count(), 14);
    }

    #[test]
    fn trellis_cell_viewport_is_pad_aware() {
        let t = Trellis::new(2, 4, vec![]);
        let v = t.cell_viewport_px(Vec2::new(480.0, 320.0));
        // (480 - 32) / 4 = 112; (320 - 32) / 2 = 144.
        assert!((v.x - 112.0).abs() < 1e-3);
        assert!((v.y - 144.0).abs() < 1e-3);
    }

    #[test]
    fn empty_trellis_has_no_borders() {
        let t = Trellis::new(0, 0, vec![]);
        let theme = Theme::light();
        let g = t.emit_grid_borders(&theme, Vec2::new(480.0, 320.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
