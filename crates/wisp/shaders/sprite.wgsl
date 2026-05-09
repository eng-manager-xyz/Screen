// sprite.wgsl
// Instanced sprite rendering — 6-vertex unit quad in `[0, 1]²`, per-instance
// model matrix + tint + anchor. Anchor shifts the unit-quad's local origin so
// that `(anchor.x, anchor.y)` lands at `(0, 0)` before the model is applied.
//
// Bindings:
//   group(0) binding(0): unused — projection is baked into the per-instance
//                       model for M0.9 (NDC space). M0.10+ moves projection
//                       to a stage uniform.
//   group(1) binding(0): texture_2d<f32>
//   group(1) binding(1): sampler

struct VsIn {
    @builtin(vertex_index) vid: u32,
    @location(0) model_0: vec4f,
    @location(1) model_1: vec4f,
    @location(2) model_2: vec4f,
    @location(3) model_3: vec4f,
    @location(4) tint: vec4f,
    @location(5) anchor: vec2f,
}

struct VsOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) uv: vec2f,
    @location(1) tint: vec4f,
}

@group(0) @binding(0) var t_texture: texture_2d<f32>;
@group(0) @binding(1) var t_sampler: sampler;

@vertex
fn main_vs(in: VsIn) -> VsOut {
    let positions = array<vec2f, 6>(
        vec2f(0.0, 0.0),
        vec2f(1.0, 0.0),
        vec2f(0.0, 1.0),
        vec2f(1.0, 0.0),
        vec2f(1.0, 1.0),
        vec2f(0.0, 1.0),
    );
    let uvs = array<vec2f, 6>(
        vec2f(0.0, 0.0),
        vec2f(1.0, 0.0),
        vec2f(0.0, 1.0),
        vec2f(1.0, 0.0),
        vec2f(1.0, 1.0),
        vec2f(0.0, 1.0),
    );

    let model = mat4x4f(in.model_0, in.model_1, in.model_2, in.model_3);
    let local = positions[in.vid] - in.anchor;

    var out: VsOut;
    out.clip_pos = model * vec4f(local, 0.0, 1.0);
    out.uv = uvs[in.vid];
    out.tint = in.tint;
    return out;
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4f {
    let texel = textureSample(t_texture, t_sampler, in.uv);
    return texel * in.tint;
}
