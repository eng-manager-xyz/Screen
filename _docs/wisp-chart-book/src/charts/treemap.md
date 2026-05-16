# Treemap

Show hierarchical proportions by area — directory sizes, budget
breakdown, taxonomic counts. Each node is a rectangle sized to
its value; children pack inside their parent's rectangle.

The demo squarifies the **Apollo program $25.4 B cost hierarchy**
— same data as the [sunburst chart](./sunburst.md), different
geometry. The Saturn V leaf claims the largest rectangle in the
plot; CSM + LM share the second-biggest group; ground ops and
R&D fill the remainder. The treemap's strength is that area
ratios are directly comparable across nesting levels — the
Saturn V's rectangle is genuinely 5× the size of the LM's
because $9.3 B is 5× $3.0 B.

<div style="position: relative; aspect-ratio: 480 / 300; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/treemap.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-treemap" src="../demo/?chart=treemap" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: Apollo program cost treemap"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/Apollo_program#Costs" target="_blank" rel="noopener">Source: Apollo program — Costs — Wikipedia</a>
</p>

## Public surface

```rust,ignore
use wisp_chart::topology::{Treemap, TreemapNode};
use wisp_chart::color::Color;

let c = |hex| Color::from_hex(hex).unwrap();
let t = Treemap::new(TreemapNode::group("root", c("#888888"), vec![
    TreemapNode::group("Sales", c("#0072b2"), vec![
        TreemapNode::leaf("NA",   30.0, c("#56b4e9")),
        TreemapNode::leaf("EU",   20.0, c("#7faedc")),
    ]),
    TreemapNode::group("Eng", c("#d55e00"), vec![
        TreemapNode::leaf("Platform", 25.0, c("#e8853d")),
        TreemapNode::leaf("App",      18.0, c("#eea063")),
    ]),
]));
let g = t.emit_graphics(&theme, Vec2::new(480.0, 300.0));
```

## Layout — slice-and-dice (v1)

```admonish info
v1 uses **slice-and-dice**: even-depth nodes split vertically
(rows stacked top-down), odd-depth nodes split horizontally
(columns stacked left-to-right). Predictable, pixel-stable,
dependency-free.
```

```admonish note
Slice-and-dice produces visually weaker rectangles for wildly
imbalanced trees (long thin strips) than squarify. Squarify
support is a follow-on; today's layout is "good enough" for
most product-data hierarchies.
```
