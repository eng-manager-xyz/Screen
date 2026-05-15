# Chart Enter / Exit primitives

`Enter` and `Exit` are opinionated entrance / exit animations
shaped by an [`AnimTheme`]. The chart author asks for "grow" or
"draw-in"; `wisp-animation` produces the right primitive.

| Enter | Channel | Curve |
|---|---|---|
| `Grow` | scale | `Tween(0 → 1, OutBack)` |
| `DrawIn` | path | `DrawIn` over the chart outline |
| `Sweep` | rotation | `Tween(0 → 2π, OutCubic)` |
| `Fade` | alpha | `Tween(0 → 1, OutCubic)` |

| Exit | Channel | Curve |
|---|---|---|
| `Shrink` | scale | `Tween(1 → 0, InBack)` |
| `FadeOut` | alpha | `Tween(1 → 0, InCubic)` |

## API surface

```rust,ignore
use wisp_animation::{AnimTheme, Enter, Exit};

let theme = AnimTheme::snappy();
let grow = Enter::Grow.scale_tween(&theme);     // Tween<f32>
let sweep = Enter::Sweep.rotation_tween(&theme); // Tween<f32>
let fade_out = Exit::FadeOut.alpha_tween(&theme);
```

Every variant returns a `Tween<f32>` (or `Tween<f32>` of
duration ZERO for the "no-op on this channel" case). Callers
compose them with `Parallel`/`Sequence` and apply the samples to
the chart's container directly, or via `NodeProperty`.

## Recipe for "bar chart grows in"

```rust,ignore
use wisp_animation::{AnimTheme, Animation, Enter};
use std::time::Duration;

let theme = AnimTheme::smooth();
let grow = Enter::Grow.scale_tween(&theme);

// In your render loop:
let scale_y = grow.sample(driver.elapsed());
if let Some(node) = stage.get_mut(bar_node_id) {
    node.container_mut().transform.scale.y = scale_y;
}
```

Stagger across N bars by combining with `theme.stagger()` and
adding `Duration` offsets per bar.

## Test invariants

- `Enter::Grow.scale_tween` starts at 0 and ends at 1.
- `Enter::Sweep.rotation_tween` ends near 2π.
- `Enter::Fade.alpha_tween` ends at 1.
- `Exit::Shrink` and `Exit::FadeOut` are inverses of `Grow` and
  `Fade` respectively.

Full source: [`crates/wisp-animation/src/chart.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/chart.rs).
