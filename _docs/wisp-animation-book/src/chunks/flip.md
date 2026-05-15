# FLIP layout transitions

`Flip` makes chart data updates feel like *morphing* rather than
*blinking*. The technique (First, Last, Invert, Play — Paul
Lewis's CSS coinage) is three lines of glue:

1. `let state = Flip::capture(&stage)` — snapshot every reachable
   node's container transform.
2. Mutate the stage (data update, re-emit chart, reorder).
3. `let tweens = Flip::from(state, &stage, dur, ease)` — get one
   `NodeFlipTween` per node whose transform changed.

![FLIP mid-swap](../assets/wisp-animation/flip-hero.png)

The hero shows two ellipses mid-swap — both at `x = 0` halfway
between their start positions (ghosted at `x = ±0.5`). FLIP
trivially produces this interpolation for any number of nodes.

## API surface

```rust,ignore
use std::time::Duration;
use wisp_animation::{Animation, Ease, Flip};

// 1. capture
let state = Flip::capture(stage);

// 2. mutate — e.g. re-emit the chart with new sort order
re_emit_chart(stage);

// 3. produce per-node tweens
let tweens = Flip::from(state, stage, Duration::from_millis(300), Ease::OutCubic);

// 4. apply each frame
for tw in &tweens {
    if let Some(node) = stage.get_mut(tw.node) {
        node.container_mut().transform = tw.sample(driver.elapsed());
    }
}
```

## Why FLIP shines for charts

```admonish important title="No per-chart bookkeeping"
A naive "animate bar reorder" implementation has to know which
bar moved where. With FLIP, you don't — the snapshot before
mutation is enough. Re-emitting the chart fresh (the *normal*
shape for `wisp-chart`) is fully compatible. The chart code
stays declarative; the animation layer figures out what moved.
```

```admonish note title="Add/remove nodes"
v0.1 only handles transform changes on nodes that existed in
both states. Nodes added after `capture` won't get a fade-in
tween automatically; nodes destroyed before `from` won't get
a fade-out. That mid-construction shape is a follow-on once
the chart `Enter`/`Exit` primitives (M-ANIM.16) land.
```

## Test invariants

- Capture → no mutation → `from` emits zero tweens.
- Capture → translate one node → `from` emits exactly one
  tween targeting that node, with `from.position` = captured,
  `to.position` = post-mutation.
- `NodeFlipTween` samples the transform as a per-channel lerp
  (`position`, `scale`, `rotation`, `skew`) under the supplied
  ease.

Full source: [`crates/wisp-animation/src/flip.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/flip.rs).
