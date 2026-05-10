**M-MASK.1 / AUT-31** — first mask primitive in `wisp`. The `Container::clip`
field accepts a `MaskShape::RoundedRect { rect, radius }`; the renderer
auto-dispatches clipped containers through an offscreen pass that:

1. Renders the subtree into a foreground `RenderTexture`.
2. Samples the foreground in a small SDF fragment shader and multiplies
   the alpha by the rounded-rect mask.
3. Source-over-composites the masked result onto the main destination.

The shape is in NDC `[-1, +1]²` — screen-space, not container-local.
The recording-quad use case (cinematic rounded-corner crop on a
fixed-position recording surface) drove this choice; transform-aware
clipping is a future enhancement.

This story shows the full path: gradient backdrop, then a clipped
container holding a horizontal gradient "recording surface", with
corner radius `0.14` in NDC units. The rounded edges are
SDF-anti-aliased using a 1-pixel-equivalent band.

The mask data is reused by every later mask issue (AUT-20 rectangle
privacy, AUT-21 rounded privacy, AUT-23 solid redaction, AUT-28
spotlight, AUT-29 dim-outside, AUT-30 webcam shape, AUT-34 oval, AUT-35
freehand). All future shapes are extensions of this same `MaskShape`
enum.
