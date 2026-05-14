# Sunburst chart

Radial hierarchical layout — root at the centre, each depth
radiates outward as a concentric ring. Child segments span the
angular range of their parent.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-chart-web/sunburst.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=sunburst" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: sunburst"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::polar::{Sunburst, SunburstNode};
use wisp_chart::color::Color;

let c = |hex| Color::from_hex(hex).unwrap();
let s = Sunburst::new(SunburstNode::group("root", c("#888"), vec![
    SunburstNode::group("Sales", c("#0072b2"), vec![
        SunburstNode::leaf("NA",   30.0, c("#56b4e9")),
        SunburstNode::leaf("EU",   20.0, c("#7faedc")),
        SunburstNode::leaf("APAC", 15.0, c("#a3c7ea")),
    ]),
    SunburstNode::group("Eng", c("#009e73"), vec![
        SunburstNode::leaf("Platform", 25.0, c("#3eb893")),
        SunburstNode::leaf("App",      20.0, c("#71cba8")),
    ]),
]))
.ring_width_px(30.0);
let g = s.emit_graphics(&theme, Vec2::new(320.0, 320.0));
```

## Layout

```admonish info
Depth 0 (the root) is NOT drawn — only its descendants. Depth 1
is the inner ring. Each child's angular span is
`child_weight / parent_weight * parent_span`. Leaf weights are
the supplied `value`; internal-node weights are computed as the
sum of descendant leaves.
```

## When sunburst beats treemap

```admonish tip
Sunburst is the right call when **depth matters more than area
precision**. Reading a 4-level hierarchy is easier in concentric
rings than in nested rectangles because the rings give visual
isolation. Use [treemap](./treemap.md) when leaf-area
proportions are the primary read.
```
