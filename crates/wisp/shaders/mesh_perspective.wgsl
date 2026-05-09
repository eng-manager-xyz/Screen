// mesh_perspective.wgsl
// Textured quad rotated around the Y axis with applied perspective foreshortening.
// Per-instance:
//   model        — 4×4 transform from primitive-local quad to world (NDC pre-perspective)
//   tint         — color multiplied with the sampled texel
//   rotation_y   — Y-axis rotation in radians (applied in primitive-local space)
//   persp_strength — 0.0 = orthographic, ~1.0 = aggressive foreshortening

struct VsIn {
    @builtin(vertex_index) vid: u32,
    @location(0) model_0: vec4f,
    @location(1) model_1: vec4f,
    @location(2) model_2: vec4f,
    @location(3) model_3: vec4f,
    @location(4) tint: vec4f,
    @location(5) rotation_y: f32,
    @location(6) persp_strength: f32,
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

    let p = positions[in.vid];

    // Y-axis rotation: x and z rotate, y stays fixed. Quad is at z=0; rotation
    // pulls vertices into z = -p.x * sin(theta).
    let c = cos(in.rotation_y);
    let s = sin(in.rotation_y);
    let rotated = vec3f(p.x * c, p.y, -p.x * s);

    // Apply perspective foreshortening: scale x/y by (1 / (1 + z * strength)).
    let depth = 1.0 + rotated.z * in.persp_strength;
    let projected = vec3f(rotated.x / depth, rotated.y / depth, 0.0);

    let model = mat4x4f(in.model_0, in.model_1, in.model_2, in.model_3);

    var out: VsOut;
    out.clip_pos = model * vec4f(projected.xy, 0.0, 1.0);
    out.uv = uvs[in.vid];
    out.tint = in.tint;
    return out;
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4f {
    let texel = textureSample(t_texture, t_sampler, in.uv);
    return texel * in.tint;
}
