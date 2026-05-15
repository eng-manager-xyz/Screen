# MoveAlongPath

Animate a node along a polyline. Sample at `t` returns a
`PathPose { position, angle }` — position along the polyline,
plus tangent angle if `auto_rotate` is on.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/movepath-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=polar&amp;animate=move-path" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: polar plot orbiting a circle with auto-rotate"></iframe>
</div>

The demo orbits the polar plot along a 33-point circle path
(radius 0.4 NDC) over 3 seconds with `auto_rotate(true)` —
the chart's rotation always matches the tangent direction.

## API surface

```rust,ignore
use std::time::Duration;
use glam::Vec2;
use wisp_animation::{Animation, MoveAlongPath};

let circle = (0..=32)
    .map(|i| {
        let theta = (i as f32 / 32.0) * std::f32::consts::TAU;
        Vec2::new(0.4 * theta.cos(), 0.4 * theta.sin())
    })
    .collect();
let path = MoveAlongPath::new(circle, Duration::from_millis(3_000))
    .auto_rotate(true);

let pose = path.sample(Duration::from_millis(1_500));
// pose.position is the point halfway around; pose.angle is the
// tangent direction (radians).
```

## Test invariants

- `sample(Duration::ZERO)` returns the path's first vertex.
- `sample(duration)` returns the path's last vertex.
- With `auto_rotate(true)`, angle equals `atan2(dy, dx)` of the
  active segment.
- Empty path returns `PathPose::default()` without panicking.

Full source: [`crates/wisp-animation/src/move_along_path.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/move_along_path.rs).
