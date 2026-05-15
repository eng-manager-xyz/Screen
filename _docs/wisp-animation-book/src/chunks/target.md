# Target trait + NodeProperty

`Target<V>` binds an animation's output to a mutable slot
somewhere in the world. `NodeProperty` is the built-in
implementation that talks to `wisp::Stage` and writes one of the
four `Container` properties: alpha, translation, rotation, scale.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/target-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=polar&amp;animate=slide" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: polar plot sliding via Target<Vec2> on Translation"></iframe>
</div>

The demo wraps `Tween<Vec2>::new((-0.3, 0), (0.3, 0), 900ms)` with
infinite `MirroredRepeat` and writes each sample through the
equivalent of `<NodeProperty as Target<Vec2>>::write` to the
polar plot node's `transform.position`.

## The trait

```rust,ignore
pub trait Target<V> {
    fn read(&self, stage: &Stage) -> V;
    fn write(&self, stage: &mut Stage, value: V);
}
```

Generic over the value type. One `NodeProperty` implements
`Target<f32>` (for alpha/rotation), `Target<Vec2>` (for
translation/scale), and `Target<Transform>` (for the whole
matrix). The `Property` enum discriminates which field gets
read/written.

## Typed witnesses, not reflection

```admonish important title="No string lookups, no `Any`"
`Property` is an enum: `Alpha`, `Translation`, `Rotation`, `Scale`.
No `&'static str` field name; no `Any`-style runtime trait
object; no reflection. The dispatch cost per write is one match
arm + a field assignment — the compiler often inlines the whole
chain.

The trade-off is that adding a fifth property type means adding
a fifth enum variant + a fifth match arm in each `Target` impl.
That's a deliberate floor on growth — we're not building a
property system, we're animating containers.
```

## Stale-node safety

```admonish warning title="Writing to a destroyed node is a no-op"
If a chart re-emits and the old `NodeId` no longer resolves
(`Stage::get_mut` returns `None`), the `Target::write` call
silently does nothing. This is the right shape for animation —
a stale animation shouldn't crash the renderer; it should just
fail to land.

The unit test
[`write_to_stale_node_is_noop`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/target.rs)
asserts this — destroy the node, then call `.write`; no panic.
```

## Constructors

```rust,ignore
use wisp_animation::{NodeProperty, Target};

let alpha   = NodeProperty::alpha(node_id);       // Target<f32>
let pos     = NodeProperty::translation(node_id); // Target<Vec2>
let rot     = NodeProperty::rotation(node_id);    // Target<f32>
let scale   = NodeProperty::scale(node_id);       // Target<Vec2>
```

`Target<Transform>` is also implemented on `NodeProperty` —
useful for the upcoming [FLIP](./flip.md) layout transitions
(M-ANIM.14) where you want to push a whole transform delta at
once.

## Test invariants

- Round-trip: writing a value then reading returns the same
  value (modulo `f32::EPSILON`).
- Stale-node writes do not panic.
- Rotation writes land on `transform.rotation`, not on alpha,
  even though both are `Target<f32>`.

Full source: [`crates/wisp-animation/src/target.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/target.rs).
