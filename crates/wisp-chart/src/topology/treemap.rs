//! Treemap — hierarchical rectangle nesting. v1 uses a
//! slice-and-dice layout (alternate horizontal / vertical splits
//! by depth) rather than full squarify; visually weaker for
//! wildly-imbalanced trees but pixel-stable and dependency-free.

use glam::Vec2;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One node in the treemap tree.
#[derive(Clone, Debug)]
pub struct TreemapNode {
    /// Display label.
    pub label: String,
    /// Leaf weight. For internal nodes the rendered weight is
    /// computed as the sum of all descendant leaves.
    pub value: f32,
    /// Fill colour. Internal-node colour is shown when the node
    /// has no children left to subdivide.
    pub color: ChartColor,
    /// Children — empty for leaves.
    pub children: Vec<TreemapNode>,
}

impl TreemapNode {
    /// Leaf-node constructor.
    #[must_use]
    pub fn leaf(label: impl Into<String>, value: f32, color: ChartColor) -> Self {
        Self {
            label: label.into(),
            value,
            color,
            children: Vec::new(),
        }
    }

    /// Internal-node constructor — weight is computed from
    /// children automatically.
    #[must_use]
    pub fn group(label: impl Into<String>, color: ChartColor, children: Vec<Self>) -> Self {
        let value = children.iter().map(Self::rendered_weight).sum();
        Self {
            label: label.into(),
            value,
            color,
            children,
        }
    }

    fn rendered_weight(&self) -> f32 {
        if self.children.is_empty() {
            self.value
        } else {
            self.children.iter().map(Self::rendered_weight).sum()
        }
    }
}

/// Treemap value type.
#[derive(Clone, Debug)]
pub struct Treemap {
    /// Root node — its children become the first-level rects.
    pub root: TreemapNode,
}

impl Treemap {
    /// Construct from a root node.
    #[must_use]
    pub const fn new(root: TreemapNode) -> Self {
        Self { root }
    }

    /// Emit one filled rect per leaf. `1 px` gap between
    /// adjacent rects so the boundary reads even when adjacent
    /// rects share a fill colour.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        if self.root.children.is_empty() && self.root.rendered_weight().abs() < f32::EPSILON {
            return g;
        }
        let pad = 16.0_f32;
        let rect = (
            pad,
            pad,
            viewport_px.x - pad * 2.0,
            viewport_px.y - pad * 2.0,
        );
        emit_node(&mut g, &self.root, rect, 0, viewport_px);
        g
    }
}

fn emit_node(
    out: &mut Graphics,
    node: &TreemapNode,
    rect: (f32, f32, f32, f32),
    depth: u32,
    viewport_px: Vec2,
) {
    let (rx, ry, rw, rh) = rect;
    if rw <= 1.0 || rh <= 1.0 {
        return;
    }
    if node.children.is_empty() {
        out.fill(Fill::Solid(chart_to_wisp(node.color)));
        let ndc = px_rect_to_ndc(
            rx,
            ry,
            (rw - 1.0).max(0.0),
            (rh - 1.0).max(0.0),
            viewport_px,
        );
        out.draw_rect(ndc);
        return;
    }
    let total: f32 = node.children.iter().map(TreemapNode::rendered_weight).sum();
    if total.abs() < f32::EPSILON {
        return;
    }
    // Slice-and-dice — even depth splits vertically (rows of
    // children stacked downward); odd depth splits horizontally.
    if depth.is_multiple_of(2) {
        let mut cursor = ry;
        for child in &node.children {
            let child_h = child.rendered_weight() / total * rh;
            emit_node(
                out,
                child,
                (rx, cursor, rw, child_h),
                depth + 1,
                viewport_px,
            );
            cursor += child_h;
        }
    } else {
        let mut cursor = rx;
        for child in &node.children {
            let child_w = child.rendered_weight() / total * rw;
            emit_node(
                out,
                child,
                (cursor, ry, child_w, rh),
                depth + 1,
                viewport_px,
            );
            cursor += child_w;
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn c(hex: &str) -> ChartColor {
        ChartColor::from_hex(hex).unwrap()
    }

    fn fixture() -> Treemap {
        Treemap::new(TreemapNode::group(
            "root",
            c("#888888"),
            vec![
                TreemapNode::group(
                    "A",
                    c("#0072b2"),
                    vec![
                        TreemapNode::leaf("A1", 20.0, c("#56b4e9")),
                        TreemapNode::leaf("A2", 15.0, c("#7faedc")),
                    ],
                ),
                TreemapNode::group(
                    "B",
                    c("#d55e00"),
                    vec![
                        TreemapNode::leaf("B1", 25.0, c("#e8853d")),
                        TreemapNode::leaf("B2", 10.0, c("#eea063")),
                    ],
                ),
            ],
        ))
    }

    #[test]
    fn treemap_emits_one_rect_per_leaf() {
        let theme = Theme::light();
        let g = fixture().emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 4);
    }

    #[test]
    fn rendered_weight_sums_leaves() {
        assert!((fixture().root.rendered_weight() - 70.0).abs() < 1e-5);
    }

    #[test]
    fn empty_treemap_emits_nothing() {
        let t = Treemap::new(TreemapNode::group("root", c("#888888"), vec![]));
        let theme = Theme::light();
        let g = t.emit_graphics(&theme, Vec2::new(400.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
