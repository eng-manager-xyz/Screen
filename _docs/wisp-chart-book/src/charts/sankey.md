# Sankey flow diagram

Show flows between nodes laid out in columns — sources on the
left, sinks on the right, intermediate nodes in between. Ribbon
thickness encodes flow magnitude. Useful for conversion funnels,
budget allocation, energy flow, attribution.

The demo plots the **NASA astronaut career flow, Groups 1–3
(Mercury / Gemini / Apollo eras)**. ~30 astronauts; sources on
the left are the service branches they came from (USAF, Navy /
USMC); the middle column groups them by training cohort
(Mercury or Gemini); the right shows the ultimate Apollo
outcome (Walked on Moon vs Did not). Twelve astronauts
ultimately walked on the Moon — the right-hand ribbon converging
into that node is the visible bottom-line of the whole program.

<div style="position: relative; aspect-ratio: 3 / 2; max-width: 540px; margin: 1rem 0; background: url('../assets/wisp-chart-web/sankey.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-sankey" src="../demo/?chart=sankey" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: NASA astronaut career flow"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/NASA_Astronaut_Group_1" target="_blank" rel="noopener">Source: NASA Astronaut Group 1 — Wikipedia</a>
</p>

## Public surface

```rust,ignore
use wisp_chart::topology::{Sankey, SankeyLink, SankeyNode};

let nodes = vec![
    SankeyNode::new("Organic",   /* column */ 0, blue),
    SankeyNode::new("Paid",      0, orange),
    SankeyNode::new("Signed Up", 1, green),
    SankeyNode::new("Trial",     1, pink),
    SankeyNode::new("Converted", 2, sky),
    SankeyNode::new("Lost",      2, gold),
];
let links = vec![
    SankeyLink::new(0, 2, 40.0, grey),
    SankeyLink::new(0, 3, 25.0, grey),
    // ...
];
let s = Sankey::new(nodes, links);
let g = s.emit_graphics(&theme, viewport);
```

```admonish info title="Layout"
v1 uses a **column-based layout**: every node names its column
explicitly, link Y-positions stack within each column in
declaration order, and ribbon ribbons are drawn as convex quads
(not Bezier curves). Quads are visually noisier than Beziers at
crossings but are deterministic, cheap, and easy to test —
appropriate for v1.
```

```admonish tip title="When to reach for Sankey"
- Conversion / funnel narratives where you also want to show
  the lost flow at each step.
- Budget allocation between fixed buckets.
- "Where does this come from / where does it go" — anything
  with a clear flow direction.

For a strictly-stepped conversion without crossings,
[funnel](./funnel.md) is the simpler shape.
```
