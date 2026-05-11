# Wisp text architecture

[Linear: AUT-75](https://linear.app/harwood/issue/AUT-75)

Wisp **owns** the text data model. App, editor, project state, and
storybook code never see `cosmic_text::*` or `glyphon::*` types — they
see `WispText`, `WispTextStyle`, and a few related value types.
Backends (`WispTextEngine` + `WispTextRenderer`) plug in behind this
trait surface; the project format and inspector controls stay
backend-stable when we swap or upgrade them.

## Type relationships

```mermaid
sequenceDiagram
    participant Caller
    participant Engine as WispTextEngine
    participant Layout as Box&lt;dyn WispTextLayout&gt;<br/>(backend-specific, opaque)
    participant Renderer as WispTextRenderer<br/>(GPU side)

    Caller ->> Engine: layout(WispText { content, style,<br/>position, max_width_ndc })
    Note over Engine,Layout: line-break, shape,<br/>per-glyph metrics
    Engine -->> Caller: Box&lt;dyn WispTextLayout&gt;
    Caller ->> Layout: metrics() (Caller-visible)
    Caller ->> Renderer: draw(layout, text)
    Note over Renderer,Layout: renderer downcasts<br/>to concrete backend type
    Renderer ->> Layout: read backend-private buffer
```

For most callers `WispText` is the only type they construct directly.
Backends are selected through whichever method on `Renderer`
consumes the text — today the M0.15 bitmap path; after M-TEXT.4 it'll
be `AtlasText`; after M-TEXT.3 it'll also be `FlexibleText`.

## Value types

| Type | Purpose |
|---|---|
| `WispFontHandle(u32)` | Opaque font reference. Atlas backend treats it as a slot id; Cosmic Text backend treats it as a `Family + Weight + Style` query. |
| `WispFontWeight` | `Thin / Light / Regular / Medium / Bold / Black / Custom(u16)` clamped to `[100, 900]`. CSS-compatible. |
| `WispFontStyle` | `Normal / Italic`. |
| `WispTextAlign` | `Left / Center / Right`. |
| `WispTextStyle` | Bundle: font + size_ndc + color + line_height + letter_spacing + weight + style + align. |
| `WispTextMetrics` | Layout output: line_count, max_width, total_height, baseline. |
| `WispText` | The user-facing primitive: content + style + position + optional wrap. |

`WispTextStyle` exposes a builder (`with_font`, `with_size`,
`with_color`, `with_weight`, `italic`, `with_align`) so callers can
assemble styles inline. `WispText::new(content)` defaults to white,
`size_ndc = 0.06`, line height `1.2`, regular weight, normal style,
left-aligned, no wrap.

## Trait surface

```rust
pub trait WispTextLayout: Debug + Send + Sync {
    fn metrics(&self) -> WispTextMetrics;
}

pub trait WispTextEngine {
    fn layout(&self, text: &WispText) -> Box<dyn WispTextLayout>;
}

pub trait WispTextRenderer {
    fn draw(&self, layout: &dyn WispTextLayout, text: &WispText);
}
```

Backends pair their own engine + renderer implementations and
exchange a backend-specific concrete layout type behind the trait.
The renderer side downcasts when needed; callers never see the
concrete layout.

`Send + Sync` on `WispTextLayout` is required so caches (M-DYN.2-style)
can hold layouts across frames. Engines and renderers may live behind
`&self` so the renderer struct can hold them without interior mutability
contention.

## Backends

| Backend | Engine + Renderer | Status |
|---|---|---|
| `AtlasText` | bitmap font atlas, M0.15 era | M-TEXT.4 — repackages the existing `Text` node. |
| `FlexibleText` | `cosmic_text` layout + `glyphon` render | M-TEXT.2 + M-TEXT.3. |

Future backends (e.g. `MsdfText` for resolution-independent glyph
rendering, `SvgText` for vector outlines) drop in behind the same
trait without touching app/editor code.

## Why two backends

- **`AtlasText`** preserves the M0.15 contract: bytemap atlases, one
  draw call per font, deterministic batch. Cheap, fast, fixed-size,
  works without external font files.
- **`FlexibleText`** opens up real font fallback, BiDi, line breaking,
  shaping. Necessary for captions, callouts, and any user-typed text.
  Heavier per-frame, mitigated by M-TEXT.5 render-to-texture caching.

The boundary lets stories, tests, and the recorder pick the right
backend per use case without a project-format change.

## Done when

- [x] Public Wisp text data model exists (`WispText`, `WispTextStyle`,
  `WispTextMetrics`, `WispFontHandle`).
- [x] Trait surface (`WispTextLayout`, `WispTextEngine`,
  `WispTextRenderer`) defined.
- [x] No third-party engine type leaks through any app-facing struct.
- [x] Existing bitmap text can be represented through the abstraction
  (M-TEXT.4 lands the actual repackaging; the trait surface accepts
  the existing data shape).
- [x] mdBook page: `wisp/text/architecture.md` (this page).
- [x] `just gate` green.

## API

- [`wisp::WispText`](../../api/wisp/text/struct.WispText.html)
- [`wisp::WispTextStyle`](../../api/wisp/text/struct.WispTextStyle.html)
- [`wisp::WispTextMetrics`](../../api/wisp/text/struct.WispTextMetrics.html)
- [`wisp::WispTextLayout`](../../api/wisp/text/trait.WispTextLayout.html)
- [`wisp::WispTextEngine`](../../api/wisp/text/trait.WispTextEngine.html)
- [`wisp::WispTextRenderer`](../../api/wisp/text/trait.WispTextRenderer.html)
