// quad.wgsl
// Textured quad with model transform + tint, single-instance.
// Vertices generated from vertex_index — 6 verts, 2 triangles, unit square in NDC.
// Bindings:
//   group(0) binding(0): QuadUniforms — model: mat4x4f, tint: vec4f
//   group(1) binding(0): texture_2d<f32>
//   group(1) binding(1): sampler

struct QuadUniforms {
    model: mat4x4f,
    tint: vec4f,
}

struct VsOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var<uniform> u: QuadUniforms;
@group(1) @binding(0) var t_texture: texture_2d<f32>;
@group(1) @binding(1) var t_sampler: sampler;

@vertex
fn main_vs(@builtin(vertex_index) idx: u32) -> VsOut {
    let positions = array<vec2f, 6>(
        vec2f(-1.0, -1.0),
        vec2f( 1.0, -1.0),
        vec2f(-1.0,  1.0),
        vec2f( 1.0, -1.0),
        vec2f( 1.0,  1.0),
        vec2f(-1.0,  1.0),
    );
    let uvs = array<vec2f, 6>(
        vec2f(0.0, 1.0),
        vec2f(1.0, 1.0),
        vec2f(0.0, 0.0),
        vec2f(1.0, 1.0),
        vec2f(1.0, 0.0),
        vec2f(0.0, 0.0),
    );

    var out: VsOut;
    out.clip_pos = u.model * vec4f(positions[idx], 0.0, 1.0);
    out.uv = uvs[idx];
    return out;
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4f {
    let texel = textureSample(t_texture, t_sampler, in.uv);
    return texel * u.tint;
}
