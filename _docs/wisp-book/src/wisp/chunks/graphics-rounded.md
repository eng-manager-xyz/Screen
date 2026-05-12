# Rounded rect with stroke — M0.12 / M0.13

![rounded rect](../../assets/wisp/graphics-rounded.png)

Three rectangles sharing one shader, one pipeline, one draw call.

The unified `graphics_solid.wgsl` handles rect (radius=0), rounded rect, and
ellipse via a `kind` flag. Anti-aliasing comes from `fwidth(d)` of the SDF,
giving clean edges at any zoom level without MSAA.

Stroke is rendered as a second instance with `mode=1`. The vertex shader
expands the bounding quad by `stroke_width/2` so the band has room to draw.
Fill + outline both batch into the same draw call.

For the recorder this primitive becomes: video padding/corners, keyboard
chip backgrounds, caption backgrounds, click ripples (as outlined ellipses),
mask highlights — all the chrome around the recording.

---

[`Graphics` API](../../api/wisp/scene/struct.Graphics.html) · [`Stroke`](../../api/wisp/scene/struct.Stroke.html) · [Stories index](../stories.md)
