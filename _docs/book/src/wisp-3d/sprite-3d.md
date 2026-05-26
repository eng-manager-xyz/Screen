# `Sprite3D`

Unlit alpha-blended primitives — ring, circle, quad — placed in 3D space. The wisp-3d equivalent of THREE's `MeshBasicMaterial`.

## The alpha-occlusion gotcha

Translucent geometry must NOT write to the depth buffer, or it punches "holes" through whatever's drawn after it. The 404 page's "eye of providence" composition (glow ellipse + iris ring + pupil) sits on the front face of the pyramid; if the eye sprites wrote depth, the wireframe drawn later would think the pyramid is closer than it is and disappear behind the eye's transparent regions.

```admonish important title="depth-test ON, depth-write OFF"
`SpritePipeline::new` hardwires:

- `depth_compare: LessEqual` — the sprite shows up AT its depth (opaque geometry in front will occlude it).
- `depth_write_enabled: false` — the sprite does NOT update the depth buffer (so whatever draws afterwards isn't fooled into thinking the sprite is solid).
- `cull_mode: None` — sprites are single-sided and seeable from either side.
- `BlendState::ALPHA_BLENDING`.

This is the CLASSIC 3D-rendering trap. Get it wrong and the visual is "the eye works but the wireframe disappears" or "the wireframe is fine but the eye glow has a black halo". Both are downstream of the same depth-write bug.
```

## Constructors

| Method | Output | Use |
|---|---|---|
| `Sprite3D::circle(radius, segments)` | filled disc, fan-triangulated | base glow, pupil |
| `Sprite3D::ring(inner, outer, segments)` | annulus, quad-strip triangulated | iris ring |
| `Sprite3D::quad(width, height)` | XY rectangle | flat glow, vignette |

All three return a [`Mesh3D`](./mesh-3d.md) carrying dummy `+Z` normals (discarded by the unlit shader). Reuses the mesh vertex layout so the buffer machinery is shared with the lit pipeline.

## Composing the eye

The 404 eye is `(Mat4::from_translation((0, -0.02, 0.79)) * Mat4::from_rotation_x(-0.45))` applied to:

1. `Sprite3D::circle(0.35, 48)` scaled `(1.55, 0.48, 1)` — orange glow.
2. `Sprite3D::ring(0.11, 0.18, 48)` scaled `(1.78, 0.58, 1)` — iris.
3. `Sprite3D::circle(0.055, 32)` scaled `(1, 1.2, 1)` — pupil.

Each draws via `SpritePipeline::draw_one(..., tint=[r, g, b, a])` in front-to-back order so the alpha-on-alpha layering reads correctly.
