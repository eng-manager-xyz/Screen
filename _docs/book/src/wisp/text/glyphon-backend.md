# FlexibleText — Glyphon WGPU rasterizer — M-TEXT.3 / AUT-77

This chunk lands the **rasterization** half of `FlexibleText`. The
[`FlexibleTextEngine`](./flexible-cosmic.md) (M-TEXT.2) shapes a
`WispText` into a `FlexibleTextLayout` (a cosmic-text `Buffer`); this
chunk hands that buffer to [`glyphon`](https://crates.io/crates/glyphon)
to paint into a `wgpu::TextureView`.

[api](../../api/wisp/text/flexible_renderer/struct.FlexibleTextRenderer.html)

## Why `glyphon`

Glyphon is the de-facto wgpu rasterizer for cosmic-text. It owns the
glyph atlas (LRU, growable), the per-glyph instance buffer, and the
draw pipeline; we provide the `FontSystem` and the target view.
Building this ourselves would replicate ~800 LoC of carefully-tuned
atlas-packing code without a corresponding upside.

The crate is small (≈ 5k LoC), license-clean (MIT/Apache-2.0), and
pinned to `=0.8.0` to match wgpu 24 + cosmic-text 0.12 at the API
level (`=` not `^` because glyphon's wgpu version is exact, not
semver-driven).

## Shape — a sibling of `Renderer`, not a method on it

```text
Application ─┬─► Renderer            (sprites, graphics, masks, …)
             └─► FlexibleTextRenderer (glyphon — opt-in)
```

`FlexibleTextRenderer` is **not** automatically constructed by
`Renderer`. Two reasons:

1. **Cost.** Glyphon allocates its own atlas + pipeline (~few MB of
   GPU memory + shader compilation). Apps that never render flexible
   text (e.g. the storybook smoke harness) shouldn't pay it.
2. **Lifecycle.** The renderer needs an `Arc<Mutex<FontSystem>>`
   handle from the engine so layout-time and rasterization-time
   glyph metrics agree. Wiring this through `Renderer::new` would
   couple two opt-in subsystems.

The cost: callers wire the engine + renderer explicitly. The benefit:
zero footprint when unused.

## Lifecycle

```rust
use wisp::application::{AppConfig, Application};
use wisp::text::{FlexibleTextEngine, FlexibleTextRenderer, WispText};
use wisp::color::Color;
use glam::Vec2;

let app = pollster::block_on(Application::new(AppConfig::default()))?;

let engine = FlexibleTextEngine::new();
let mut renderer = FlexibleTextRenderer::new(
    &app,
    wgpu::TextureFormat::Rgba8Unorm,
    engine.font_system_handle(), // shared FontSystem
);
renderer.set_resolution(width_px, height_px);

let layout = engine.layout_concrete(&WispText::new("Hello"));
renderer.draw(
    target_view,
    &[(&layout, Vec2::new(-0.5, 0.5), Color::WHITE)],
    /* clear = */ true,
);
```

`set_resolution` is sticky — call it once per resize, not per frame.
`draw` accepts a slice of `(layout, position_ndc, color)` so multiple
text spans share one glyphon `prepare` + draw call.

## NDC ↔ pixel conversion

The engine shapes at a fixed `REFERENCE_PX = 1000` basis (see
[FlexibleText layout](./flexible-cosmic.md#ndc--pixel-basis)). At draw
time the renderer rescales:

```text
left_px = (pos_ndc.x * 0.5 + 0.5) * target_width_px
top_px  = (0.5 - pos_ndc.y * 0.5) * target_height_px   // +y flip
scale   = target_height_px / REFERENCE_PX
```

Same `FlexibleTextLayout` can be drawn into any-size target without
re-shaping — only the per-draw rescale changes.

## Atlas hygiene

`renderer.trim_atlas()` evicts LRU glyph cells that weren't touched
since the previous frame. Call between frames for long-running scenes
(editor / playback); short-lived contexts (export burn-in) can skip
it.

## Blend mode

Glyphon's pipeline ships with **normal alpha blending**
(pre-multiplied alpha → `OneMinusSrcAlpha` over destination). That
satisfies AUT-77's "supports at least Normal blend mode" requirement.
Additive / multiply / etc. modes are M-TEXT.13 territory and need
either a glyphon fork or a render-to-texture detour (M-TEXT.5).

## Tests

| Test | Asserts |
|---|---|
| `renderer_constructs_against_default_app` | `new()` returns a renderer wired against the default `Application`. |
| `empty_draw_does_not_panic` | Empty layouts list is a no-op (no glyphon `prepare` failure on zero areas). |
| `draw_hello_paints_some_non_zero_pixels` | Layout "Hello" → glyphon → `RenderTexture` → read pixels → at least one non-zero-alpha pixel. Catches the case where glyphon silently produces empty output (e.g. font system has no glyphs). |

The smoke test is a **pixel** test, not a snapshot test, on purpose:
system fonts vary by host (CI runners pick up Liberation Sans / DejaVu
/ Helvetica depending on platform), so a byte-for-byte snapshot would
churn on every CI bump. "Some pixels are non-zero" catches the
genuine regressions (glyphon broken, font system empty, atlas
allocation failed) without flapping on cosmetic font swaps.

## Known gaps (intentional)

- **Container transform + alpha.** AUT-77 calls for "respects
  container transform and alpha." Today the renderer takes
  `position_ndc` + per-default `Color`; the transform / alpha
  inheritance flows through scene composition, which the renderer
  doesn't yet participate in. M-TEXT.5 (RT cache integration) brings
  this in by drawing into an intermediate `RenderTexture` and
  composing through the existing sprite pipeline — at which point
  transform + alpha are free.
- **`render_stage` participation.** Same story — the renderer can be
  invoked between `Renderer::render_stage` calls today, but isn't a
  pass inside it. M-TEXT.5 makes the RT path the natural integration
  point.
- **`WispTextRenderer` trait impl.** The crate-level trait was
  designed before glyphon's "give me a target view + resolution"
  shape was understood. Implementing it would require widening the
  trait (target view, resolution, batch mode) — deferred until a
  second backend exists that would benefit from a uniform trait
  surface.

## Done when

- [x] `glyphon = "=0.8.0"` added to `crates/wisp/Cargo.toml`.
- [x] `FlexibleTextRenderer` exposed via `wisp::text::FlexibleTextRenderer`.
- [x] Renderer constructs against a default `Application`.
- [x] Renderer rasterizes a `FlexibleTextLayout` into a `wgpu::TextureView`.
- [x] Pixel test confirms non-zero glyph coverage end-to-end.
- [x] mdBook chapter (this one).
- [x] `just gate` green.
