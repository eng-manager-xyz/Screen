# Textured quad — M0.6

![hello quad](../../assets/wisp/hello-quad.png)

A 64×64 procedural checker pattern uploaded as a `Texture` and rendered as a
single `Sprite`, anchored at the center, rotating slowly.

This is the M0.6 baseline: `Texture::from_rgba` constructs a GPU texture from
raw bytes, `Sprite::from_texture` wraps it in a scene-graph node, and
`Renderer::render_stage` composites it onto the canvas.

The unified quad shader (`quad.wgsl`) takes per-vertex UV and a per-instance
model matrix + tint. Anchor `(0.5, 0.5)` centers the sprite at its position so
rotation pivots around the sprite center rather than its top-left corner.

Once the recorder is wired up, the screen-capture frame becomes a
`VideoTexture` (M0.11) consumed by exactly this same `Sprite` path — the
recorded video is just another textured quad.

---

[`wisp` API](../../api/wisp/index.html) · [Stories index](../stories.md)
