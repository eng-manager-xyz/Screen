# PathMorph + DrawIn

Two of the highest-leverage chart-entrance shapes:

- **`PathMorph`** linearly interpolates between two equal-length
  `Vec<Vec2>` lists. v0.1 requires equal vertex counts; auto-
  resampling lands in v0.2.
- **`DrawIn`** reveals a polyline over time — at `t = 0` returns
  an empty list; at `t = duration` returns the full path; in
  between, the last vertex is interpolated along the active
  segment.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/drawin-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=polar&amp;animate=drawin" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: polar plot drawn along an S-curve via DrawIn"></iframe>
</div>

The demo computes an 11-point S-curve in NDC, wraps it in
`DrawIn::new(path, 2s)`, and uses the last vertex of the
revealed prefix as the polar plot's `transform.position` each
frame. The chart "moves along the line being drawn".

## API surface

```rust,ignore
use std::time::Duration;
use glam::Vec2;
use wisp_animation::{Animation, DrawIn, PathMorph};

let morph = PathMorph::new(
    vec![Vec2::ZERO, Vec2::new(10.0, 0.0)],
    vec![Vec2::new(5.0, 5.0), Vec2::new(15.0, 5.0)],
    Duration::from_secs(1),
);
let mid = morph.sample(Duration::from_millis(500));

let reveal = DrawIn::new(
    vec![Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(20.0, 5.0)],
    Duration::from_secs(2),
);
let visible = reveal.sample(Duration::from_millis(1_000));
```

Both implement `Animation<Output = Vec<Vec2>>` — feed the result
into a caller-managed `wisp::Graphics::draw_line` chain (or the
forthcoming `Path::trimmed` helper if/when `wisp` exposes it).

## Test invariants

- `PathMorph` at `t = duration` returns the `to` list verbatim;
  midpoint averages component-wise.
- `DrawIn` at `t = 0` is empty; at `t = duration` is the full
  path; at `t = 0.25 · duration` includes the first endpoint
  plus an interpolated trailing vertex at 50% of the first
  segment.

Full source: [`crates/wisp-animation/src/path_morph.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/path_morph.rs).
