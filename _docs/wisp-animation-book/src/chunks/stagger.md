# Stagger

`Stagger` computes per-index time offsets so a group of
animations starts in a wave instead of all at once. Modelled on
anime.js v4's `stagger(value, options)`.

<div style="position: relative; aspect-ratio: 3 / 1; max-width: 540px; margin: 1rem 0; background: url('../assets/wisp-animation/stagger-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?animate=stagger" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: 5-dot row pulsing center-out via Stagger"></iframe>
</div>

The demo runs five dot nodes with `Stagger::each(120ms).from(Center)`.
A wave of alpha brightness flows outward from the middle dot
every 1.4 seconds. The still frame catches a wave moment with
the centre dot brightest and the edges dimmest.

## API surface

```rust,ignore
use std::time::Duration;
use wisp_animation::{Stagger, StaggerFrom};

let s = Stagger::each(Duration::from_millis(60))
    .from(StaggerFrom::Center);

// Offset for each index in a count-element list:
for i in 0..8 {
    let dt = s.offset_for(i, 8);
    // …schedule animation start = base + dt
}

// 2-D grid (L1 distance from origin):
let grid = Stagger::each(Duration::from_millis(40))
    .from(StaggerFrom::Center)
    .grid(/* rows */ 5, /* cols */ 7);
```

## From-points

| `StaggerFrom` | Origin |
|---|---|
| `Start` (default) | Index 0 → offset 0; grows linearly. |
| `End` | Last index → offset 0; grows back. |
| `Center` | Middle index → offset 0; grows outward. |
| `Index(n)` | Specified index → offset 0; grows in both directions. |

## Grid mode

```admonish info title="L1 Manhattan distance"
With `.grid(rows, cols)`, the index set is treated as a 2-D grid
and offset = `each × (|row - origin_row| + |col - origin_col|)`.
This produces concentric diamond waves rather than concentric
circles. Cheap, deterministic, and reads "as a wave" perfectly.
```

## Test invariants

- `StaggerFrom::Center` on 5 items: index 2 → 0; indices 1 and 3 →
  `each × 1`; indices 0 and 4 → `each × 2`.
- `StaggerFrom::End` on N items: last → 0; first → `each × (N-1)`.
- Grid mode uses L1 distance from the origin cell.
- `count = 0` returns `Duration::ZERO`.

Full source: [`crates/wisp-animation/src/stagger.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/stagger.rs).
