# Dynamic mask textures

[Linear: AUT-43](https://linear.app/harwood/issue/AUT-43)

`Renderer::generate_mask_texture(shape, w, h)` and the path variant
`Renderer::generate_path_mask_texture(points, w, h)` produce
single-purpose coverage `RenderTexture`s. Output stores `(m, m, m, m)`
so consumers can sample as alpha (composition) or as RGB (display /
debug). An inverted variant (`generate_mask_texture_inverted`) does
the same with the mask flipped — useful for spotlight / dim-outside.

```rust
use wisp::{MaskShape, math::Rect};

let mask = renderer.generate_mask_texture(
    &app,
    MaskShape::rounded_rect(Rect::new(-0.5, -0.5, 1.0, 1.0), 0.2),
    256,
    256,
);
// `mask` is an RGBA8 RT with coverage in alpha (and mirrored in RGB
// for visual debugging). Sample `.a` for composition.
```

![](../../assets/wisp/mask-texture.png)

## Architecture

This primitive owns *only* coverage. The existing `apply_clip` /
`apply_privacy_blur` / `apply_solid_redaction` / `apply_spotlight` /
`apply_path_clip` primitives still compute SDF + foreground sample in
a single shader; M-DYN.1 introduces the **separated** path so future
work can:

1. **Cache** the mask (M-DYN.2 / AUT-44) — identical regions across
   frames don't regenerate.
2. **Reuse** one mask across multiple effects — privacy blur and
   redaction over the same region don't run the SDF twice.
3. **Drive masks from vector data** (M-VEC.3 / AUT-55) — vector shapes
   become alpha textures via the same primitive.
4. **Refactor** the existing combined-shader primitives onto this
   model in M-VEC.4-6.

```mermaid
flowchart LR
    Shape[shape data] --> Pipeline[MaskTexturePipeline]
    Pipeline --> Alpha["alpha RT (m, m, m, m)"]
    Alpha --> Mask["sample.a × foreground = masked"]
```

Two pipelines under the hood:

- **`MaskTexturePipeline`** — `mask_texture.wgsl`. Same SDF math as
  `clip.wgsl` (rounded-rect / ellipse + degenerate cases for rect /
  circle), output `vec4(m, m, m, m)`. No texture binding.
- **`PathMaskTexturePipeline`** — `path_mask_texture.wgsl`. Same
  uniform-buffered point-in-polygon as `path_clip.wgsl`, hard edges
  for V1, 32-vertex cap.

## API

- [`wisp::Renderer::generate_mask_texture`](../../api/wisp/render/struct.Renderer.html#method.generate_mask_texture)
- [`wisp::Renderer::generate_mask_texture_inverted`](../../api/wisp/render/struct.Renderer.html#method.generate_mask_texture_inverted)
- [`wisp::Renderer::generate_path_mask_texture`](../../api/wisp/render/struct.Renderer.html#method.generate_path_mask_texture)
