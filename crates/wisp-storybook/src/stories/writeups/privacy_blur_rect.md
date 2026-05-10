# Rectangle privacy blur — M-MASK.2 / AUT-20

`Renderer::apply_privacy_blur(region, radius, base, output)` is the first
masked filter primitive. It composes three existing pieces:

1. **Blur** the entire `base` RT with `BlurFilter::new(radius)` into a
   scratch RT.
2. **Clip** the blurred RT to `MaskShape::Rect { region }` — pixels
   outside the rect drop to alpha 0, inside stays opaque.
3. **Compose** the masked overlay over a fresh copy of the base via the
   blit pipeline's `compose_over` (alpha-blending). Outside the rect the
   base shows through pixel-perfect; inside the rect the blurred copy
   wins.

This story renders a "fake desktop" (gradient + white grid lines) into
a capture RT, then applies the privacy blur over a center-right
rectangle. The grid lines vanish inside the rect (they're now ~12px
blurred mush) but stay sharp outside. The yellow outline marks the
input `region`.

The three pixel-readback tests in `tests/privacy_blur_rect.rs` lock in:

- pixels outside the region are bit-exact equal to `base`,
- pixels straddling a strong color seam pick up both colors via the
  blur kernel (mixing → privacy guarantee), and
- the blur falloff stays bounded — pixels well inside one half still
  favor that half's dominant color (so this is a privacy *blur*, not a
  privacy *fill*).

Up next (AUT-21) generalizes the rect to a rounded rect, reusing the
same `apply_privacy_blur` signature with a `MaskShape::RoundedRect`.
