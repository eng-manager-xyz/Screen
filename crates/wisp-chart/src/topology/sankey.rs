//! Sankey diagram — node-bar columns + flow ribbons whose
//! thickness encodes magnitude. v1 places nodes by per-stage
//! cumulative offset and emits each ribbon as a single convex
//! quadrilateral connecting the source's slot to the target's
//! slot. (Bezier ribbons land in a follow-on once curved-path
//! tessellation is convex-safe in `wisp::Graphics`.)

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One Sankey node — a bar in some column.
#[derive(Clone, Debug, PartialEq)]
pub struct SankeyNode {
    /// Display label.
    pub label: String,
    /// Column index — 0-based. Determines the node's
    /// horizontal position.
    pub column: usize,
    /// Fill colour.
    pub color: ChartColor,
}

impl SankeyNode {
    /// Construct from label + column + colour.
    #[must_use]
    pub fn new(label: impl Into<String>, column: usize, color: ChartColor) -> Self {
        Self {
            label: label.into(),
            column,
            color,
        }
    }
}

/// One flow edge — source → target with a magnitude.
#[derive(Clone, Debug, PartialEq)]
pub struct SankeyLink {
    /// Index into `nodes` of the source.
    pub source: usize,
    /// Index into `nodes` of the target.
    pub target: usize,
    /// Flow magnitude — drives the ribbon's vertical thickness.
    pub value: f32,
    /// Ribbon fill colour. A soft grey is a reasonable default.
    pub color: ChartColor,
}

impl SankeyLink {
    /// Construct from source / target / value / colour.
    #[must_use]
    pub fn new(source: usize, target: usize, value: f32, color: ChartColor) -> Self {
        Self {
            source,
            target,
            value,
            color,
        }
    }
}

/// Sankey diagram value type.
#[derive(Clone, Debug)]
pub struct Sankey {
    /// Nodes — bars in each column.
    pub nodes: Vec<SankeyNode>,
    /// Flow edges between nodes.
    pub links: Vec<SankeyLink>,
}

impl Sankey {
    /// Construct from node + link lists.
    #[must_use]
    pub const fn new(nodes: Vec<SankeyNode>, links: Vec<SankeyLink>) -> Self {
        Self { nodes, links }
    }

    /// Emit per-column node bars + per-link ribbons.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.nodes.is_empty() {
            return g;
        }
        let pad = 24.0_f32;
        let plot_left = pad;
        let plot_right = viewport_px.x - pad;
        let plot_top = pad;
        let plot_bottom = viewport_px.y - pad;
        let plot_w = plot_right - plot_left;
        let plot_h = plot_bottom - plot_top;
        let max_column = self.nodes.iter().map(|n| n.column).max().unwrap_or(0);
        let column_count = max_column + 1;
        let node_w = 12.0_f32;
        let column_step = if column_count > 1 {
            (plot_w - node_w) / usize_to_f32(column_count - 1)
        } else {
            0.0
        };

        // Compute per-node total in/out value (max of the two
        // halves) and per-column scale.
        let mut node_value = vec![0.0_f32; self.nodes.len()];
        for link in &self.links {
            if link.source < node_value.len() {
                node_value[link.source] += link.value;
            }
            if link.target < node_value.len() {
                node_value[link.target] += link.value;
            }
        }
        // Per-column total → vertical pixel scale.
        let mut column_totals = vec![0.0_f32; column_count];
        for (i, n) in self.nodes.iter().enumerate() {
            column_totals[n.column] += node_value[i];
        }
        let max_col_total = column_totals.iter().copied().fold(f32::EPSILON, f32::max);
        let value_to_px = (plot_h - 16.0) / max_col_total;

        // Per-node top-y (cumulative within its column).
        let mut col_cursor = vec![plot_top + 8.0_f32; column_count];
        let mut node_y = vec![0.0_f32; self.nodes.len()];
        let mut node_height = vec![0.0_f32; self.nodes.len()];
        for (i, n) in self.nodes.iter().enumerate() {
            let h = node_value[i] * value_to_px;
            node_height[i] = h;
            node_y[i] = col_cursor[n.column];
            col_cursor[n.column] += h + 4.0; // 4-px node gap.
        }

        // Track running offset on source-side + target-side of
        // each node so multiple ribbons stack inside the bar.
        let mut src_used = vec![0.0_f32; self.nodes.len()];
        let mut dst_used = vec![0.0_f32; self.nodes.len()];

        // Ribbons (drawn first so node bars composite on top).
        for link in &self.links {
            if link.source >= self.nodes.len() || link.target >= self.nodes.len() {
                continue;
            }
            let src_col = self.nodes[link.source].column;
            let dst_col = self.nodes[link.target].column;
            let src_x = plot_left + usize_to_f32(src_col) * column_step + node_w;
            let dst_x = plot_left + usize_to_f32(dst_col) * column_step;
            let h = link.value * value_to_px;
            let src_top = node_y[link.source] + src_used[link.source];
            let src_bot = src_top + h;
            let dst_top = node_y[link.target] + dst_used[link.target];
            let dst_bot = dst_top + h;
            src_used[link.source] += h;
            dst_used[link.target] += h;
            g.fill(Fill::Solid(chart_to_wisp(link.color)));
            // CCW from bottom-left.
            let bl = pixel_to_ndc(Vec2::new(src_x, src_bot), viewport_px);
            let br = pixel_to_ndc(Vec2::new(dst_x, dst_bot), viewport_px);
            let tr = pixel_to_ndc(Vec2::new(dst_x, dst_top), viewport_px);
            let tl = pixel_to_ndc(Vec2::new(src_x, src_top), viewport_px);
            g.draw_polygon(&[bl, br, tr, tl]);
        }

        // Node bars.
        for (i, n) in self.nodes.iter().enumerate() {
            let x = plot_left + usize_to_f32(n.column) * column_step;
            let y = node_y[i];
            let h = node_height[i];
            if h < 1.0 {
                continue;
            }
            g.fill(Fill::Solid(chart_to_wisp(n.color)));
            let rect = px_rect_to_ndc(x, y, node_w, h, viewport_px);
            g.draw_rect(rect);
        }
        g
    }
}

fn pixel_to_ndc(p: Vec2, viewport_px: Vec2) -> Vec2 {
    Vec2::new(
        p.x / viewport_px.x * 2.0 - 1.0,
        1.0 - p.y / viewport_px.y * 2.0,
    )
}

fn px_rect_to_ndc(x: f32, y: f32, w: f32, h: f32, viewport_px: Vec2) -> Rect {
    let nx = x / viewport_px.x * 2.0 - 1.0;
    let ny = 1.0 - (y + h) / viewport_px.y * 2.0;
    Rect::new(nx, ny, w / viewport_px.x * 2.0, h / viewport_px.y * 2.0)
}

fn chart_to_wisp(c: ChartColor) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn usize_to_f32(v: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "column counts ≤ ~10 in practice"
    )]
    {
        v as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(hex: &str) -> ChartColor {
        ChartColor::from_hex(hex).unwrap()
    }

    fn fixture() -> Sankey {
        let nodes = vec![
            SankeyNode::new("A", 0, c("#0072b2")),
            SankeyNode::new("B", 0, c("#d55e00")),
            SankeyNode::new("C", 1, c("#009e73")),
            SankeyNode::new("D", 1, c("#cc79a7")),
        ];
        let links = vec![
            SankeyLink::new(0, 2, 10.0, c("#999999")),
            SankeyLink::new(0, 3, 5.0, c("#999999")),
            SankeyLink::new(1, 2, 8.0, c("#999999")),
            SankeyLink::new(1, 3, 3.0, c("#999999")),
        ];
        Sankey::new(nodes, links)
    }

    #[test]
    fn sankey_emits_one_polygon_per_link_plus_one_rect_per_node() {
        let theme = Theme::light();
        let g = fixture().emit_graphics(&theme, Vec2::new(480.0, 240.0));
        // 4 ribbons + 4 node bars = 8.
        assert_eq!(g.primitive_count(), 8);
    }

    #[test]
    fn empty_sankey_emits_nothing() {
        let theme = Theme::light();
        let g = Sankey::new(Vec::new(), Vec::new()).emit_graphics(&theme, Vec2::new(480.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
