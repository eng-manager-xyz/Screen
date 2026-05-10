# Mask boolean ops — M-VEC.11 / AUT-63

`Renderer::combine_masks(a, b, op, output)` produces a third mask
texture from two inputs and a `MaskCombineOp` (Union / Intersect /
Subtract). Output is itself a regular alpha-mask `RenderTexture` and
flows into any downstream composition primitive
(`apply_mask_to_texture`, `compose_blur_through_mask`, etc.).

The story tiles two overlapping circles three ways:

- **Left** — `Union`: pixel covered by either circle.
- **Middle** — `Intersect`: pixel covered by both circles only.
- **Right** — `Subtract` (`a - b`): pixel covered by left circle but
  not by right circle.

Backed by `mask_combine.wgsl`: one shader, three op codes, branches
on a uniform `u32`.
