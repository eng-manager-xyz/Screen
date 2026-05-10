# Text → RenderTexture → Sprite (M-TEXT.5 / AUT-79)

Demonstrates the **render-to-texture** path for FlexibleText:

```text
WispText → FlexibleTextEngine → FlexibleTextRenderer
        → RenderTexture → Sprite → Renderer::render_stage
```

The story constructs a `TextTexturePipeline`, rasterizes two pieces
of text into separate `RenderTexture`s, and attaches each as a
`Sprite` to the scene graph. The standard sprite pipeline batches
both draws — same code path as any other textured quad.

## What this proves

- Text textures share the scene-graph integration of every other
  `Sprite`: transform, alpha, blend mode, parent/child inheritance,
  z-order — all free.
- Repeated rendering of the same content + style + dims returns the
  cached texture (no GPU re-rasterization). The pipeline carries a
  FIFO cache bounded at `MAX_ENTRIES = 64`.
- Any change to content, style (size / color / weight / italic /
  letter-spacing / line-height / alignment), `font_family`,
  `max_width_ndc`, or texture dimensions invalidates the entry.

## Why "Sprite-of-text" matters

The recorder's editor needs to caption, highlight, label, and overlay
text on top of video. Doing this through textured sprites means:

1. **Filters, masks, blends, exports** apply to text the same way
   they apply to any sprite (M-TEXT.6 territory).
2. **Per-frame layout cost** is paid once, not every frame. The cache
   short-circuits when the inspector hasn't changed anything.
3. **Backend swaps stay invisible** — `WispText` is the input,
   `Sprite` is the output, the backend behind `FlexibleText` (cosmic
   + glyphon today) can change without app-side rewrites.
