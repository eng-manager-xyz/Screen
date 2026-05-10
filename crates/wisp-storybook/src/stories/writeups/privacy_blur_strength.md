# Privacy blur strengths — M-MASK.4 / AUT-22

`BlurStrength::{Soft, Medium, Strong, Custom(f32)}` is the
renderer-data API for "how strong is this blur." The editor's
strength slider will snap between Soft/Medium/Strong; advanced users
get `Custom(f32)` (clamped to `[0, 64]`) as an escape hatch.

This story renders the same gradient + grid backdrop three times,
each time with a different `BlurStrength`, side by side. From left to
right: Soft, Medium, Strong. The grid lines are still legible in Soft,
hint-readable in Medium, and unrecognizable in Strong.

The strength enum exists separately from the `f32` because:

- The editor needs a stable, persistable identity. If we retune Soft
  from `r=6` to `r=8` later, every project file already saying
  `BlurStrength::Soft` Just Works.
- A symbolic preset prevents the slider's "decimal radius" trap (every
  user trying to find their preferred 13.7 px setting and then arguing
  about defaults).
- `Custom(f32)` keeps the door open for deterministic story snapshots
  that need an exact radius.

The end-to-end pixel test (`tests/privacy_blur_strength.rs`,
`higher_strength_produces_more_blur_evidence`) samples the red side of
a sharp red/blue split far from the seam, and asserts blue bleed
strictly grows: Soft < Medium < Strong. That's the contract — the
strength enum maps to actually-different blur kernels, not just
cosmetic labels.
