# Spotlight / highlight — M-MASK.6 / AUT-28

`Renderer::apply_spotlight(shape, dim_color, base, output)` is the
attention-guiding primitive: pixels inside `shape` show through
unchanged; pixels outside are blended toward `dim_color`.

The story renders a fake screen capture with grid lines and a yellow
"target button" in the lower right. The spotlight focuses a rounded
rectangle around the button and dims everything else with a 0.7-alpha
black overlay — exactly the cinematic walkthrough effect.

## Architecture

Same composition as solid redaction, with one bit flipped: the clip
pipeline runs in `apply_inverted` mode so the masked overlay covers
*outside* the shape instead of inside. The WGSL just inverts the SDF
mask via a uniform flag (`invert: f32`); same pipeline, no separate
shader.

This is the AUT-28 base primitive; AUT-29 (dim-outside inverse mask)
will be a thin wrapper that sets the dim strength explicitly. The
shape can be any `MaskShape` — rect / rounded-rect today, with future
circle / ellipse / freehand all plugging in.
