# API cheat-sheet

Every public primitive at a glance, with the chapter to read
for detail.

## Core

| Type | Output | Constructor | Chapter |
|---|---|---|---|
| `Animation` trait | `Self::Output` | implement on any value | [trait + Driver](./animation-trait-driver.md) |
| `Driver` | clock | `Driver::realtime()` / `Driver::fixed(dt)` | [trait + Driver](./animation-trait-driver.md) |
| `Animatable` trait | `Self` | impl on `f32`/`Vec2`/`Color`/… | [Animatable](./animatable.md) |
| `Tween<V>` | `V` | `Tween::new(from, to, dur).ease(Ease::OutCubic)` | [Tween + Ease](./tween.md) |
| `Ease` | `f32` | `Ease::OutBack` (31 named + 4 parametric) | [Tween + Ease](./tween.md) |

## Composition

| Type | Shape | Chapter |
|---|---|---|
| `Sequence<O>` | `a.then(b)` — duration sums | [composition](./composition.md) |
| `Parallel<O>` | `a.with(b)` — duration is max | [composition](./composition.md) |
| `Delay` | spacer of `Output = ()` | [composition](./composition.md) |
| `Repeat` | wrap with `.repeat(RepeatCount::Infinite)` | [repeat](./repeat.md) |
| `RepeatStrategy::MirroredRepeat` | yoyo | [repeat](./repeat.md) |

## Motion primitives

| Type | Output | Use | Chapter |
|---|---|---|---|
| `Spring` | `f32` | overshoot / settle | [spring](./spring.md) |
| `Decay` | `f32` | fling / inertia glide | [decay](./decay.md) |
| `LinearRamp` | `f32` | placeholder for one-off ramps | (used internally) |

## Curves + paths + text

| Type | Output | Use | Chapter |
|---|---|---|---|
| `Track<V>` | `V` | N-key waypoint walk | [keyframe-track](./keyframe-track.md) |
| `Curve` | `Vec2` | Catmull-Rom / Bezier | [keyframe-track](./keyframe-track.md) |
| `PathMorph` | `Vec<Vec2>` | morph polyline → polyline | [path-morph](./path-morph.md) |
| `DrawIn` | `Vec<Vec2>` | reveal a polyline 0..=1 | [path-morph](./path-morph.md) |
| `MoveAlongPath` | `PathPose { position, angle }` | follow a path with tangent rotation | [move-along-path](./move-along-path.md) |
| `TypeWriter` | `usize` | reveal characters | [typewriter](./typewriter.md) |
| `ColorTween` | `Color` | RGB / Oklab / Oklch | [color-space](./color-space.md) |

## Targets + dispatch

| Type | Use | Chapter |
|---|---|---|
| `Target<V>` trait | abstract write to "somewhere" | [target](./target.md) |
| `NodeProperty` | concrete write to a `wisp::Stage` node | [target](./target.md) |
| `BatchDriver` | last-wins multi-animation tick | [multi-animation](./multi-animation.md) |
| `BoundScalar` | `(Box<dyn Animation<Output=f32>>, NodeProperty)` pair | [multi-animation](./multi-animation.md) |
| `Stagger` | per-index offset generator | [stagger](./stagger.md) |

## Lifecycle + integration

| Type | Use | Chapter |
|---|---|---|
| `WithCallbacks` | wrap with on_start / on_complete | [lifecycle](./lifecycle.md) |
| `EventReader` | poll Started/Cycle/Completed events | [lifecycle](./lifecycle.md) |
| `Flip` / `FlipState` | layout transitions | [flip](./flip.md) |
| `AnimTheme` | snappy() / smooth() presets | [theme](./theme.md) |
| `Enter` / `Exit` | chart entrance / exit constructors | [chart-enter-exit](./chart-enter-exit.md) |

## Minimal pattern reference

```rust,ignore
// 1. Build an animation value (cheap, composable).
let tween = Tween::new(0.0_f32, 1.0, Duration::from_millis(500))
    .ease(Ease::OutCubic);

// 2. Drive it.
let mut driver = Driver::realtime();
driver.play();

// 3. Sample each frame.
fn frame(driver: &mut Driver, tween: &impl Animation<Output = f32>, dt: Duration) -> f32 {
    driver.tick(dt);
    driver.sample(tween)
}

// 4. Apply to your scene-graph node.
stage.get_mut(node).unwrap().container_mut().alpha = value;

// 5. Render once.
renderer.render_stage(&app, &view, wisp::Color::WHITE, app.stage());
```

For multi-animation scenes, replace steps 3–4 with one call to
`BatchDriver::tick_scalars(dt, &mut anims, &mut stage)` —
deterministic last-wins, zero-alloc, [multi-animation](./multi-animation.md).

## Test invariants for the whole crate

All primitives share:

- **Endpoint correctness** — `sample(0)` returns `from`,
  `sample(duration)` returns `to`.
- **Determinism** — same `t`, same output, every run.
- **Allocation discipline** — `sample` never allocates;
  `tick` allocates only during the first frame's buffer growth.
- **Stale-target safety** — writes to destroyed nodes silently
  no-op.

Run them all with `cargo nextest run -p wisp-animation` — 115
unit + 4 determinism + 2 perf tests as of this writing.
