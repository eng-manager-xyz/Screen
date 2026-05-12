# AtlasText vs FlexibleText

[Linear: AUT-78](https://linear.app/harwood/issue/AUT-78)

Wisp ships **two** text backends behind the [`WispTextEngine`] +
[`WispTextLayout`] trait surface (M-TEXT.1 / AUT-75). Pick by use
case. Both are first-class — neither replaces the other.

[`WispTextEngine`]: ../../api/wisp/text/trait.WispTextEngine.html
[`WispTextLayout`]: ../../api/wisp/text/trait.WispTextLayout.html

## When to use which

| Use case | Backend |
|---|---|
| Static labels / HUD overlays / FPS counters | **AtlasText** |
| Watermarks (a logo line, a fixed credit) | **AtlasText** |
| Thousands of identical glyphs per frame (debug, telemetry) | **AtlasText** |
| Captions on recorded clips | **FlexibleText** |
| Callout / annotation text the user types | **FlexibleText** |
| Anything needing fallback fonts, BiDi, CJK, ligatures | **FlexibleText** |
| Anything needing weight / italic to actually change the rasterization | **FlexibleText** |

Rule of thumb: **`AtlasText` for things the codebase puts on screen.
`FlexibleText` for things the user types.**

## Comparison table

| Property | `AtlasText` | `FlexibleText` |
|---|---|---|
| Layout engine | font-cell metric walk | `cosmic_text` |
| Rasterization | bitmap atlas (font8x8 today) | `glyphon` (sub-pixel, per-frame) |
| Word wrap (`text.max_width_ndc`) | ❌ ignored — only `\n` breaks | ✅ |
| BiDi / shaping | ❌ | ✅ |
| Weight / italic respected | ❌ ignored at layout | ✅ |
| Letter spacing | ✅ (`style.letter_spacing_ndc`) | ✅ |
| Alignment | ✅ Left / Center / Right | ✅ |
| Line height | ✅ (`style.line_height` × `size_ndc`) | ✅ |
| Color | ✅ solid tint (`style.color`) | ✅ + per-span (M-TEXT.13) |
| Non-ASCII | ❌ silently dropped | ✅ |
| Font fallback | ❌ | ✅ |
| Performance per frame | O(glyphs), one atlas | O(glyphs) + cache lookup |
| Determinism for snapshot tests | ✅ pixel-perfect | ✅ once cached |
| Project-format storage | `WispText` + `style.font` slot | `WispText` + `style.font` query |
| GPU memory | one 128×128 atlas | dynamic, glyph-cache sized |
| External font files | ❌ embedded | ✅ system + bundled |

## Layout semantics — `AtlasText`

The atlas backend treats `style.size_ndc` as a *cell side length* in NDC.
Every font8x8 cell is square, so:

```text
glyph width  = size_ndc
glyph height = size_ndc
horizontal advance = size_ndc + style.letter_spacing_ndc
line step  = size_ndc * style.line_height
```

Lines are split on `\n`. `text.max_width_ndc` is **ignored** —
soft-wrapping isn't part of `AtlasText` (use `FlexibleText`).
Codepoints absent from the bitmap atlas (anything ≥ 128) are silently
dropped; this matches the M0.15 `scene::Text` node behavior.

`style.weight` and `style.style` (italic) **don't change the
rasterization** — there's only one atlas. The fields are accepted so a
single `WispTextStyle` can be authored in code that may switch backends
later. `FlexibleText` will honor them.

## Trait surface alignment

[`AtlasTextEngine`] implements [`WispTextEngine`] and produces an
[`AtlasTextLayout`] (which implements [`WispTextLayout`]). The
concrete `AtlasTextLayout::glyphs()` method exposes the per-glyph
NDC quad + atlas UVs to the renderer side without requiring a
downcast — the trait `metrics()` keeps the dyn-friendly entrypoint.

[`AtlasTextEngine`]: ../../api/wisp/text/atlas/struct.AtlasTextEngine.html
[`AtlasTextLayout`]: ../../api/wisp/text/atlas/struct.AtlasTextLayout.html

The M0.15 `scene::Text` node + `text_pipeline` continue to drive the
on-GPU draws today; `AtlasTextEngine` formalizes the layout half so
M-TEXT.5 can route the same data through the upcoming
render-to-texture path when text needs to participate in masks /
filters / blends.
