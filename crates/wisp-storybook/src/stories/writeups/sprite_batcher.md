100 sprites, all sharing the same texture `Arc`, rendered in a single draw call.

This is the M0.9 anti-regression contract. The renderer's `collect_batches`
walks the scene in pre-order, groups instances by `(texture_id, blend_mode)`,
and emits one instance buffer per batch. When sprites share a texture they
collapse into one batch.

For the recorder this is what makes the cursor-trail-of-100-clicks scenario
feasible: even if the cursor effects layer fans out into many sprites in a
short window, draw-call cost stays constant.

`Texture::id()` is `Arc::as_ptr(&inner) as usize` — texture-pointer-equality.
Cloning a `Texture` shares the GPU resource and the batch key.
