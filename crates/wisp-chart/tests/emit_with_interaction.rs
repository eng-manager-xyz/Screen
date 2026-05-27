//! `emit_with_interaction` integration tests (WI.9 / AUT-312).
//!
//! Each chart kind that opts in to the interaction shape gets a
//! parity test here: emit the chart, then assert that the elements
//! vector pins each primitive to the right
//! [`wisp_chart::ChartElementId`].

use glam::Vec2;
use wisp_chart::baseline::BaselineChart;
use wisp_chart::color::Color;
use wisp_chart::distributions::{Histogram, HistogramBin};
use wisp_chart::heatmap::{Histogram2D, SequentialPalette};
use wisp_chart::interaction::ChartElementId;
use wisp_chart::polar::{Pie, Slice};
use wisp_chart::theme::Theme;
use wisp_chart::topology::{Treemap, TreemapNode};

fn vp() -> Vec2 {
    Vec2::new(640.0, 480.0)
}

fn red() -> Color {
    Color::from_hex("#e74c3c").unwrap()
}
fn green() -> Color {
    Color::from_hex("#27ae60").unwrap()
}
fn blue() -> Color {
    Color::from_hex("#3498db").unwrap()
}

#[test]
fn pie_emits_one_slice_per_pickable_with_matching_indices() {
    let pie = Pie::new(vec![
        Slice::new(1.0, "a", red()),
        Slice::new(2.0, "b", green()),
        Slice::new(3.0, "c", blue()),
    ]);
    let emitted = pie.emit_with_interaction(&Theme::default(), vp());
    assert_eq!(emitted.elements.len(), 3, "3 slices ↔ 3 elements");
    assert_eq!(emitted.elements[0].1, ChartElementId::Slice(0));
    assert_eq!(emitted.elements[1].1, ChartElementId::Slice(1));
    assert_eq!(emitted.elements[2].1, ChartElementId::Slice(2));
    // Primitive indices are monotone (we drew in slice order).
    assert!(
        emitted.elements.windows(2).all(|w| w[0].0 < w[1].0),
        "primitive indices are strictly ascending"
    );
}

#[test]
fn pie_with_zero_value_slice_skips_only_that_slice() {
    let pie = Pie::new(vec![
        Slice::new(1.0, "a", red()),
        Slice::new(0.0, "skip", green()),
        Slice::new(2.0, "c", blue()),
    ]);
    let emitted = pie.emit_with_interaction(&Theme::default(), vp());
    assert_eq!(emitted.elements.len(), 2);
    assert_eq!(emitted.elements[0].1, ChartElementId::Slice(0));
    assert_eq!(emitted.elements[1].1, ChartElementId::Slice(2));
}

#[test]
fn histogram_emits_one_bin_per_pickable_in_index_order() {
    let h = Histogram {
        bins: vec![
            HistogramBin {
                lo: 0.0,
                hi: 1.0,
                count: 3,
            },
            HistogramBin {
                lo: 1.0,
                hi: 2.0,
                count: 5,
            },
            HistogramBin {
                lo: 2.0,
                hi: 3.0,
                count: 2,
            },
        ],
        color: red(),
    };
    let emitted = h.emit_with_interaction(&Theme::default(), vp());
    assert_eq!(emitted.elements.len(), 3);
    assert_eq!(emitted.elements[0].1, ChartElementId::Bin(0));
    assert_eq!(emitted.elements[1].1, ChartElementId::Bin(1));
    assert_eq!(emitted.elements[2].1, ChartElementId::Bin(2));
}

#[test]
fn baseline_emits_one_quad_per_segment() {
    let b = BaselineChart::new(vec![(0.0, 1.0), (1.0, 2.0), (2.0, 1.5), (3.0, 0.5)], 1.0);
    let emitted = b.emit_with_interaction(&Theme::default(), vp());
    // 4 points = 3 segments.
    assert_eq!(emitted.elements.len(), 3);
    assert_eq!(emitted.elements[0].1, ChartElementId::Bar(0));
    assert_eq!(emitted.elements[1].1, ChartElementId::Bar(1));
    assert_eq!(emitted.elements[2].1, ChartElementId::Bar(2));
}

#[test]
fn treemap_assigns_leaf_indices_in_depth_first_order() {
    let leaf_0 = TreemapNode::leaf("a", 1.0, red());
    let leaf_1 = TreemapNode::leaf("b", 2.0, green());
    let leaf_2 = TreemapNode::leaf("c", 3.0, blue());
    let child_b = TreemapNode::leaf("middle", 1.5, red());
    let child_a = TreemapNode::group("group_a", red(), vec![leaf_0, leaf_1]);
    let child_c = TreemapNode::group("group_c", blue(), vec![leaf_2]);
    let root = TreemapNode::group("root", red(), vec![child_a, child_b, child_c]);
    let tm = Treemap::new(root);
    let emitted = tm.emit_with_interaction(&Theme::default(), vp());
    // 4 leaves total.
    assert_eq!(emitted.elements.len(), 4);
    for (i, (_, id)) in emitted.elements.iter().enumerate() {
        assert_eq!(*id, ChartElementId::Leaf(i), "depth-first sequential");
    }
}

#[test]
fn histogram2d_emits_only_nonzero_cells_with_matching_row_col() {
    // Construct directly via the public fields to skip the binning path.
    let mut counts = vec![0_u32; 3 * 3];
    counts[0] = 5; // row 0, col 0
    counts[4] = 3; // row 1, col 1
    counts[8] = 7; // row 2, col 2
    let h2d = Histogram2D {
        counts,
        cols: 3,
        rows: 3,
        palette: SequentialPalette::magma(),
    };
    let emitted = h2d.emit_with_interaction(&Theme::default(), vp());
    assert_eq!(emitted.elements.len(), 3, "3 non-zero cells");
    assert_eq!(
        emitted.elements[0].1,
        ChartElementId::Cell { row: 0, col: 0 }
    );
    assert_eq!(
        emitted.elements[1].1,
        ChartElementId::Cell { row: 1, col: 1 }
    );
    assert_eq!(
        emitted.elements[2].1,
        ChartElementId::Cell { row: 2, col: 2 }
    );
}

#[test]
fn element_for_primitive_round_trips_on_real_chart() {
    let pie = Pie::new(vec![
        Slice::new(1.0, "a", red()),
        Slice::new(2.0, "b", green()),
    ]);
    let emitted = pie.emit_with_interaction(&Theme::default(), vp());
    let (idx, id) = emitted.elements[0];
    assert_eq!(emitted.element_for_primitive(idx), Some(&id));
    let last = emitted.elements.last().unwrap().0;
    assert_eq!(emitted.element_for_primitive(last + 100), None);
}

#[test]
fn emit_graphics_returns_same_primitives_as_emit_with_interaction() {
    let pie = Pie::new(vec![
        Slice::new(1.0, "a", red()),
        Slice::new(2.0, "b", green()),
    ]);
    let g_simple = pie.emit_graphics(&Theme::default(), vp());
    let g_full = pie.emit_with_interaction(&Theme::default(), vp()).graphics;
    assert_eq!(g_simple.primitive_count(), g_full.primitive_count());
}
