//! Sunburst chart — radial hierarchical layout. Root at the
//! centre, depth radiates outward as concentric rings, child
//! segments span the angular range of their parent.

use glam::Vec2;
use wisp::{Color, Fill, Graphics};

use crate::color::Color as ChartColor;
use crate::theme::Theme;

/// One node in the sunburst tree. Tree is built by linking
/// `children` recursively. Leaves have empty `children`.
#[derive(Clone, Debug)]
pub struct SunburstNode {
    /// Display label (not rendered by `emit_graphics`; caller adds
    /// `wisp::Text` separately).
    pub label: String,
    /// Numeric weight. Internal nodes' weight is `sum(children)`
    /// for layout purposes; only leaves' weights need to be
    /// supplied — internal nodes are computed at render time.
    pub value: f32,
    /// Per-node fill colour.
    pub color: ChartColor,
    /// Child nodes — must sum (by `value`) to ≤ parent's value.
    pub children: Vec<SunburstNode>,
}

impl SunburstNode {
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

    /// Internal-node constructor — value is computed from
    /// children at render time, so the supplied value is just
    /// the starting estimate / total.
    #[must_use]
    pub fn group(label: impl Into<String>, color: ChartColor, children: Vec<Self>) -> Self {
        let value = children.iter().map(|c| c.value).sum();
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

/// A sunburst chart — root node + render config.
#[derive(Clone, Debug)]
pub struct Sunburst {
    /// Root node. Its direct children become the first ring.
    pub root: SunburstNode,
    /// Pixel radius of the innermost ring start. `0` puts the
    /// root at the centre (typical); larger values leave a hole.
    pub inner_radius_px: f32,
    /// Pixel thickness per ring.
    pub ring_width_px: f32,
}

impl Sunburst {
    /// Construct with sensible defaults — flat root, 36 px ring
    /// thickness, no central hole.
    #[must_use]
    pub fn new(root: SunburstNode) -> Self {
        Self {
            root,
            inner_radius_px: 0.0,
            ring_width_px: 36.0,
        }
    }

    /// Override ring thickness.
    #[must_use]
    pub const fn ring_width_px(mut self, width: f32) -> Self {
        self.ring_width_px = width;
        self
    }

    /// Override inner radius (for a hollow centre).
    #[must_use]
    pub const fn inner_radius_px(mut self, radius: f32) -> Self {
        self.inner_radius_px = radius;
        self
    }

    /// Emit nested annular sectors as a `wisp::Graphics`.
    /// Layout: each node spans `[parent_start, parent_end]` in
    /// angle and `[depth * ring_width, (depth+1) * ring_width]`
    /// in radius. Depth 0 = direct children of root.
    #[must_use]
    pub fn emit_graphics(&self, theme: &Theme, viewport_px: Vec2) -> Graphics {
        let _ = theme;
        let mut g = Graphics::new();
        let total = self.root.rendered_weight();
        if total.abs() < f32::EPSILON || self.root.children.is_empty() {
            return g;
        }
        let centre_ndc = pixel_to_ndc(viewport_px * 0.5, viewport_px);
        let _ = total;
        emit_node(
            &mut g,
            &self.root,
            self.inner_radius_px,
            self.ring_width_px,
            0,
            0.0,
            std::f32::consts::TAU,
            centre_ndc,
            viewport_px,
        );
        g
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "recursive emit_node passes the entire layout state down; bundling into a struct would just rename the fields"
)]
fn emit_node(
    g: &mut Graphics,
    node: &SunburstNode,
    inner_radius_px: f32,
    ring_width_px: f32,
    depth: u32,
    angle_start: f32,
    angle_end: f32,
    centre_ndc: Vec2,
    viewport_px: Vec2,
) {
    // Don't draw the root itself — only its descendants. The
    // first ring is depth=1.
    if depth >= 1 {
        let r_inner_px = inner_radius_px + (f32_from_u32(depth - 1)) * ring_width_px;
        let r_outer_px = r_inner_px + ring_width_px;
        let r_inner = r_inner_px / viewport_px.y * 2.0;
        let r_outer = r_outer_px / viewport_px.y * 2.0;
        if angle_end - angle_start > 1e-5 {
            g.fill(Fill::Solid(chart_to_wisp(node.color)));
            g.draw_annular_sector(centre_ndc, r_inner, r_outer, angle_start, angle_end);
        }
    }

    if node.children.is_empty() {
        return;
    }
    let weight: f32 = node
        .children
        .iter()
        .map(SunburstNode::rendered_weight)
        .sum();
    if weight.abs() < f32::EPSILON {
        return;
    }
    let span = angle_end - angle_start;
    let mut cursor = angle_start;
    for child in &node.children {
        let child_weight = child.rendered_weight();
        let child_span = child_weight / weight * span;
        let child_end = cursor + child_span;
        emit_node(
            g,
            child,
            inner_radius_px,
            ring_width_px,
            depth + 1,
            cursor,
            child_end,
            centre_ndc,
            viewport_px,
        );
        cursor = child_end;
    }
}

fn pixel_to_ndc(p: Vec2, viewport_px: Vec2) -> Vec2 {
    Vec2::new(
        p.x / viewport_px.x * 2.0 - 1.0,
        1.0 - p.y / viewport_px.y * 2.0,
    )
}

fn chart_to_wisp(c: ChartColor) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn f32_from_u32(v: u32) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "sunburst depth ≤ ~6 in practice; well within f32 mantissa"
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

    fn fixture() -> Sunburst {
        Sunburst::new(SunburstNode::group(
            "root",
            c("#888888"),
            vec![
                SunburstNode::group(
                    "A",
                    c("#e74c3c"),
                    vec![
                        SunburstNode::leaf("A1", 10.0, c("#e57373")),
                        SunburstNode::leaf("A2", 20.0, c("#ef9a9a")),
                    ],
                ),
                SunburstNode::group(
                    "B",
                    c("#27ae60"),
                    vec![
                        SunburstNode::leaf("B1", 15.0, c("#81c784")),
                        SunburstNode::leaf("B2", 5.0, c("#a5d6a7")),
                    ],
                ),
            ],
        ))
    }

    #[test]
    fn sunburst_emits_one_sector_per_descendant() {
        let s = fixture();
        let theme = Theme::light();
        let g = s.emit_graphics(&theme, Vec2::new(240.0, 240.0));
        // Root not drawn. 2 children + 4 grandchildren = 6 sectors.
        assert_eq!(g.primitive_count(), 6);
    }

    #[test]
    fn sunburst_rendered_weight_sums_leaves() {
        let s = fixture();
        // 10 + 20 + 15 + 5 = 50.
        assert!((s.root.rendered_weight() - 50.0).abs() < 1e-5);
    }

    #[test]
    fn empty_root_emits_no_sectors() {
        let s = Sunburst::new(SunburstNode::group("root", c("#888888"), vec![]));
        let theme = Theme::light();
        let g = s.emit_graphics(&theme, Vec2::new(240.0, 240.0));
        assert_eq!(g.primitive_count(), 0);
    }
}
