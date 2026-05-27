# `wisp-interaction` overview

`wisp-interaction` is the input + hit-test + camera-controller layer for the wisp family. It does NOT add input handling to each library individually — that produces N inconsistent APIs. It owns the vocabulary once.

```admonish info title="The Mother of All Demos — December 9, 1968"
On the morning of December 9, 1968, Douglas Engelbart stood at the
Fall Joint Computer Conference in San Francisco and spent ninety
minutes demonstrating almost every interactive-computing primitive
we still use. He moved a wooden box on rollers and a cursor tracked
on a video projection. He chord-keyed text. He dragged regions
between windows, followed hyperlinks, and held a real-time video
conference with collaborators in Menlo Park.

That was the founding moment of *direct manipulation* — the idea
that a computer can present a scene the user touches with a pointer,
and the scene responds. Sixty years later, the gap between "this
thing draws pixels" and "this thing responds to a user" still needs
to be bridged by code. `wisp-interaction` is that bridge.
```

## Where it fits

```mermaid
flowchart LR
  host["winit / web-sys / tauri<br/>(input source)"]
  subgraph wi["wisp-interaction"]
    inp["ButtonInput&lt;T&gt;"]
    ptr["Pointer&lt;E&gt; dispatcher"]
    hit["HitTestBackend trait"]
    cam["Camera controllers"]
  end
  host --> inp
  inp --> ptr
  ptr --> hit
  hit -.consumed by.-> wisp2d["wisp (2D)"]
  hit -.consumed by.-> wispchart["wisp-chart"]
  cam -.consumed by.-> wisp3d["wisp-3d"]
  ptr -.consumed by.-> wispanim["wisp-animation triggers"]
```

## The three engines we cross-referenced

We built `wisp-interaction` after deep research on PixiJS v8, Three.js r170+, and Bevy 0.18. The full memos live in the [`wisp-interaction` Linear project](https://linear.app/harwood/project/wisp-interaction-cf9a6b07ec52)'s WI.0 ticket description. The synthesis:

| Concern | Pattern adopted | Source |
|---|---|---|
| Keyboard / mouse-button state | `ButtonInput<T>` with three sets (`pressed` / `just_pressed` / `just_released`) generic over key kind | Bevy `crates/bevy_input/src/button_input.rs:12-60` |
| Pointer event taxonomy | `Pointer<E>` typed enum (15 variants: `Over` / `Out` / `Press` / `Release` / `Click` / `Move` / `Drag*` / `Scroll` / `Cancel`) | Bevy `crates/bevy_picking/src/events.rs:139-340` |
| Multi-touch state | `PointerId::{Mouse, Touch(u64), Custom(u128)}` keying every dispatch stage | Bevy `pointer.rs:32-46` |
| Drag without OS pointer-capture | Press-path bookkeeping (remember the ancestor chain at press, replay at release) | PixiJS `src/events/EventBoundary.ts:677-708, 1092-1133` |
| Cursor style | Stored on node, applied to host via callback indirection | PixiJS `EventSystem.ts:539-590` |
| 3D orbit camera | Spherical-coords state machine + damping + touch handlers | Three.js `examples/jsm/controls/OrbitControls.js` |
| Hit-test backend trait | Backend emits `(NodeId, HitData)` lists; core sorts + dedupes | Bevy `crates/bevy_picking/src/backend.rs:60-85` |

Explicit non-goals:

- **No ECS dependency.** Bevy proves observers are great UX; porting Bevy's archetype machinery into wisp is not. Closure-on-NodeId registration is the equivalent.
- **No 3-phase DOM event propagation.** Bubble-only. The capture phase is a DOM artifact that adds complexity without payoff for non-DOM scene graphs.
- **No 5-state `eventMode` enum.** Bevy's orthogonal 2-bit `Pickable { should_block_lower, is_hoverable }` captures the same semantics with less ceremony.
- **No brute-force per-triangle ray scan.** When a wisp-3d picking backend lands (follow-up), it'll need a BVH from day one — Three.js's naive Möller-Trumbore brute scan is a known footgun for any non-trivial mesh.

## Quickstart

```rust,no_run
use glam::Vec2;
use wisp_interaction::{
    CallbackRegistry, HitShape, MouseButton, PickableMap,
    PointerDispatcher, PointerId, PointerLocation, Wisp2dHitTest,
    HitTestBackend, Click, Pointer, ModifierState,
};
use wisp::math::Rect;
use wisp::scene::{Container, Stage};

let mut stage = Stage::new();
let button = stage.add_child(stage.root(), Container::new()).unwrap();
let mut pickable = PickableMap::new();
pickable.insert_shape(button, HitShape::Rect(Rect::new(0.0, 0.0, 100.0, 40.0)));

let mut registry = CallbackRegistry::new();
registry.on_click(button, |_: &Pointer<Click>| {
    println!("clicked!");
});

let backend = Wisp2dHitTest::new(&stage, &pickable);
let mut dispatcher = PointerDispatcher::new();
let loc = PointerLocation { viewport: Vec2::new(50.0, 20.0), modifiers: ModifierState::none() };
let hits = backend.pick(loc.viewport);
dispatcher.on_pointer_press(PointerId::Mouse, loc, MouseButton::Left, &hits, &stage, &registry);
dispatcher.on_pointer_release(PointerId::Mouse, loc, MouseButton::Left, &hits, &stage, &registry);
```

## Read next

This chapter is the architecture summary. The detailed surface lands in:

- `button-input.md` — `ButtonInput<T>` state machine (Hunt the Wumpus 1972 historical narrative)
- `pointer-events.md` — `Pointer<E>` taxonomy + dispatcher (Sketchpad 1963)
- `hit-test.md` — `HitTestBackend` + `Wisp2dHitTest` (MacPaint bucket fill 1984)
- `orbit-controller.md` — Three.js port (Pixar Luxo Jr. 1986)
- `pan-zoom-controller.md` — Figma-style zoom-around-pointer (Eames Powers of Ten 1977)
- `adapters.md` — winit + web-sys (pointer-event lineage 1968→2013)
- `animation-triggers.md` — Pointer → Tween (Disney 12 Principles 1981)
