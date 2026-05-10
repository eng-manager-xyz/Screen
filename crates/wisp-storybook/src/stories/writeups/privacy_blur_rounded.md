# Rounded privacy blur — M-MASK.3 / AUT-21

This is AUT-20's primitive with `MaskShape::RoundedRect` instead of
`MaskShape::Rect`. The `apply_privacy_blur` signature was generalized
to accept any `MaskShape`, so this story exercises the same code path
as the rectangle variant — just a different `shape` argument and a
non-zero corner radius.

The yellow outline approximates the four straight edges of the rounded
shape (the corners themselves sit just inside that outline; the
SDF-driven mask is what produces them). Notice that pixels just inside
the outline near the corners are NOT redacted — the rounded SDF
carved the corner away cleanly. AUT-22's slider over `radius` will
land on top of this primitive without further renderer work.

Three pixel-readback tests in `tests/privacy_blur_rounded.rs` lock the
contract:

- pixels outside the bounding rect match `base` bit-exactly,
- pixels *inside the bounding rect but outside the rounded corner*
  also match `base` (proves the SDF actually carved the corner),
- the seam pixel inside the rounded shape mixes both base colors via
  the blur kernel.
