# Animatable trait

`Animatable` is the typeclass that says "this value type can be
interpolated between two endpoints at parameter `t`." Everything
the [`Tween`](./tween.md), [`Track`](./keyframe-track.md), and
[`PathMorph`](./path-morph.md) types build on goes through this
one trait.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/animatable-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=contour&amp;animate=fade" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: contour plot alpha-fading via Animatable for f32"></iframe>
</div>

The demo is a `wisp-chart` contour plot whose container alpha is
ramped through `0 → 1 → 0` every two seconds — the lerp going
through `Animatable::lerp(&0.0, &1.0, t)` on `f32`.

## The trait

```rust,ignore
pub trait Animatable: Clone {
    /// Linear interpolation `a + (b - a) * t`.
    fn lerp(a: &Self, b: &Self, t: f32) -> Self;
}
```

One method. Implementations are pure, allocation-free, and produce
`a` at `t = 0.0` and `b` at `t = 1.0`. Out-of-range `t` is *not*
clamped by the trait — the caller (`Driver`, `Tween`) clamps.

## Built-in impls

| Type | Impl |
|---|---|
| `f32` / `f64` | textbook lerp; `f64` widens `t` to `f64` for the multiply. |
| `i32` | rounded lerp — endpoints exact, midpoints snap to-nearest. |
| `glam::Vec2` / `Vec3` / `Vec4` | component-wise via glam's inherent `Vec::lerp`. |
| `wisp::Color` | component-wise on `(r, g, b, a)` in **linear sRGB**. |
| `(A, B)` where `A, B: Animatable` | tuple impl for composite "position + alpha" tweens. |

```admonish important title="Linear sRGB is the default colour space"
`Animatable for wisp::Color` interpolates in **linear sRGB** because
`wisp` clears and composes in linear sRGB. That's what makes the
animation pipeline "obvious" — no per-tween colour-space surprise.

Perceptual blends (Oklab / Oklch) come in [`color-space`](./color-space.md)
(M-ANIM.13). You opt into them per-Tween via `Tween::in_oklab()`.
```

## Tuple impls

`(A, B): Animatable where A: Animatable, B: Animatable` is there so
you can tween a composite property without writing a fresh struct:

```rust,ignore
use glam::Vec2;
use wisp_animation::Animatable;

type PositionAndAlpha = (Vec2, f32);

let start: PositionAndAlpha = (Vec2::ZERO, 0.0);
let end:   PositionAndAlpha = (Vec2::new(100.0, 100.0), 1.0);

let mid = <PositionAndAlpha as Animatable>::lerp(&start, &end, 0.5);
// mid == (Vec2::new(50.0, 50.0), 0.5)
```

## Implementing for your own types

```rust,ignore
use wisp_animation::Animatable;

#[derive(Clone)]
struct Rect { x: f32, y: f32, w: f32, h: f32 }

impl Animatable for Rect {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self {
            x: f32::lerp(&a.x, &b.x, t),
            y: f32::lerp(&a.y, &b.y, t),
            w: f32::lerp(&a.w, &b.w, t),
            h: f32::lerp(&a.h, &b.h, t),
        }
    }
}
```

Stay pure: no clock reads, no IO, no allocations. The trait method
is called many times per frame — once per active `Tween`, once per
keyframe sub-segment, once per stagger child.

## Test invariants

- Each built-in impl is boundary-tested at `t = 0.0`, `t = 0.5`,
  `t = 1.0`.
- The `Vec2` / `Vec3` impls are property-checked component-wise:
  `lerp(a, b, t).x == lerp(a.x, b.x, t)`.
- Tuple-of-Animatable lerp propagates correctly to both elements.

Full source: [`crates/wisp-animation/src/animatable.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/animatable.rs).
