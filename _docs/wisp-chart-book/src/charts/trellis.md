# Trellis / small multiples

Tile a grid of mini sub-plots — one chart per category in a
row / column / grid layout. Tufte's small multiples.

## Public surface

```rust,ignore
use wisp_chart::multi::{Trellis, TrellisCell};

// 1. Decide grid dimensions.
let trellis = Trellis::new(2, 3, Vec::new());
let cell_viewport = trellis.cell_viewport_px(Vec2::new(600.0, 360.0));

// 2. Build per-cell Graphics at `cell_viewport` sizing.
let mut cells = Vec::new();
for label in ["Q1", "Q2", "Q3", "Q4", "Q5", "Q6"] {
    let plot = Plot::new(fixture_for(label))
        .axes(false)
        .mark(Mark::Bar { value_labels: false })
        .encode(plot::x("category", ScaleKind::Band))
        .encode(plot::y("value", ScaleKind::Linear));
    let g = plot.render(&theme, cell_viewport);
    cells.push(TrellisCell::new(label, g));
}
let trellis = Trellis::new(2, 3, cells);

// 3. Add positioned cells to the stage:
for g in trellis.positioned_cells(Vec2::new(600.0, 360.0)) {
    stage.add_child(root, g);
}
// 4. Add grid borders as a separate Graphics:
let borders = trellis.emit_grid_borders(&theme, Vec2::new(600.0, 360.0));
stage.add_child(root, borders);
```

## Why v1 takes pre-built Graphics

```admonish info
A "true" faceting API would re-build each sub-chart from a
filtered slice of the source DataFrame. That requires the chart
to expose its render path through an interface — which is
specific per chart family. v1 takes the simpler **"caller
builds the cell, we just tile"** approach. The caller can use
any chart type (bar / scatter / line / etc.) for each cell.
```

## Cell sizing

```admonish important
The cell's `Graphics` must be built using
`trellis.cell_viewport_px(outer)` as its viewport so its NDC
range maps cleanly onto the cell rectangle. `positioned_cells`
applies translation + scale; if the cell was built against the
wrong viewport its content will be off-centre or clipped.
```

## When to use trellis vs other multi-view options

| Use case                            | Pick                  |
|-------------------------------------|-----------------------|
| One chart per category              | Trellis               |
| Pairwise scatters of N dims         | [SPLOM](./splom.md)   |
| Many entities × time → colour grid  | [Lasagna](./lasagna.md) |
| Layered axes on one chart           | Multi-encoding `Plot` |
