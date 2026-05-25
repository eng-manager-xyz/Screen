# `Material3D` + `PaletteRampMaterial`

`Material3D` lets consumers bring their own WGSL fragment + uniform UBO without forking the pipeline. The runtime caches built pipelines keyed on `(TypeId, output_format, msaa_samples)` so the same material type doesn't recompile on every frame.

## Trait shape

```rust,no_run
# use bytemuck::{Pod, Zeroable};
# use wisp_3d::Material3D;
# #[repr(C)] #[derive(Clone, Copy, Pod, Zeroable)]
# struct MyUniform { tint: [f32; 4] }
struct MyMaterial { tint: [f32; 4] }
impl Material3D for MyMaterial {
    type Uniforms = MyUniform;
    fn wgsl_source() -> &'static str { include_str!("path/to/shader.wgsl") }
    fn uniforms(&self) -> MyUniform { MyUniform { tint: self.tint } }
}
```

The user-supplied WGSL must declare three bind groups: `view_proj` at group 0, `model` at group 1, and the material's own UBO at group 2. See `crates/wisp-3d/shaders/material_palette.wgsl` for the canonical shape.

## PaletteRampMaterial — the 404 shader port

`PaletteRampMaterial::engmanager_404()` constructs with the five hex stops from `not-found.js` (`#fe640b, #e64553, #ea76cb, #8839ef, #1e66f5`) + a `time_seconds` knob for the time-dependent palette offset.

```admonish important title="The palette ramp keys off local-space coords"
The fragment computes `t = dot(local_position, vec3(0.95, 0.52, -0.38)) * 0.28 + 0.58 + sin(time * 0.24) * 0.04`. Local-space — not world, not view. That means rotating the model rotates the palette WITH it (the colours stay glued to the geometry), which is the visual that ships in the THREE version. Don't switch to world-space coords for "neatness" — you'll lose the painterly effect.
```

## What the shader does

1. Five-stop palette ramp along the model-local diagonal.
2. Warm-band overlay (peach pushed into the upper-frontal band).
3. Fake directional lambert against `vec3(-0.25, 0.55, 0.78)` (same light as `Render3DPass`'s default).
4. Rim term (`pow(1 - |dot(n, +Z)|, 2)`).
5. Value-noise grain in screen space, time-modulated.

The `PaletteUniform` carries the 5 RGBA stops + a `vec4` time slot (packed for 16-byte alignment). 96 bytes; layout-tested.
