# Keyframe Track + Curve

`Track<V>` is a multi-keyframe animation — each key has a `(t,
value, ease)` triple, and the animation linearly interpolates
through them. Per-segment easing lets one waypoint arrive with
`OutBack` overshoot while another arrives with `InCubic`
acceleration.

`Curve` is the spatial analogue: a smooth 2-D path through
control points, sampled as `Vec2` over normalised parameter.
Two flavours: Catmull-Rom (passes through every control point)
and cubic-Bezier chain (control polygon defines tangents).

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/keyframe-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=polar&amp;animate=keyframe" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: polar plot walking through 4 scale keyframes via Track"></iframe>
</div>

The demo runs a 4-waypoint scale walk over 2 seconds:
`1.0 → 0.5 (InCubic) → 1.2 (OutBack) → 0.8 (InOutQuad)`. Each
segment uses a different ease. The still frame catches the
`1.2` waypoint at the OutBack overshoot peak.

## Track API

```rust,ignore
use std::time::Duration;
use wisp_animation::{Track, Ease};

let track: Track<f32> = Track::new()
    .key(Duration::ZERO, 1.0)
    .key_eased(Duration::from_millis(500), 0.5, Ease::InCubic)
    .key_eased(Duration::from_millis(1_200), 1.2, Ease::OutBack)
    .key_eased(Duration::from_millis(2_000), 0.8, Ease::InOutQuad);
```

Sampling at any time linearly walks the keys, finds the active
segment, applies the *destination* key's ease to the
`0..=1` progress within that segment, and lerps via `Animatable`.

## Curve API

```rust,ignore
use glam::Vec2;
use std::time::Duration;
use wisp_animation::Curve;

let path = Curve::catmull_rom(
    vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(50.0, 100.0),
        Vec2::new(100.0, 0.0),
        Vec2::new(150.0, 100.0),
    ],
    Duration::from_secs(2),
);
let bezier = Curve::bezier_chain(
    vec![
        Vec2::new(0.0, 0.0),    // P0
        Vec2::new(0.0, 100.0),  // P1 (control)
        Vec2::new(100.0, 100.0), // P2 (control)
        Vec2::new(100.0, 0.0),  // P3
    ],
    Duration::from_secs(2),
);
```

Both implement `Animation<Output = Vec2>` — feed the sampled
point into `Target<Vec2>` for "node follows a path" demos
(M-ANIM.11 will give this its own MoveAlongPath constructor).

## Test invariants

- Sampling a `Track` exactly at a key time returns that key's
  value.
- Per-segment ease applies to the segment *arriving* at the
  next key (so `Ease::InQuad` on the second key shapes the
  trajectory FROM key 1 TO key 2).
- Catmull-Rom passes through every control point (sampled at
  `s = 0.0` and `s = 1.0`).
- Bezier chain endpoints land on `P0` (at `s = 0`) and the last
  `P_n` (at `s = 1`).

Full source: [`crates/wisp-animation/src/track.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/track.rs).
