// filter_color_matrix.wgsl
// Apply a 4×5 color matrix to each fragment.
//   out = matrix · vec5(r, g, b, a, 1)

struct ColorMatrixUniforms {
    // Row-major 4×5 matrix stored as four vec4 + one vec4 of constants.
    row_r: vec4f,    // [r→r, g→r, b→r, a→r]
    row_g: vec4f,
    row_b: vec4f,
    row_a: vec4f,
    constants: vec4f, // [r constant, g constant, b constant, a constant]
}

struct VsOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var<uniform> u: ColorMatrixUniforms;
@group(1) @binding(0) var t_input: texture_2d<f32>;
@group(1) @binding(1) var t_sampler: sampler;

@vertex
fn main_vs(@builtin(vertex_index) vid: u32) -> VsOut {
    let pos = array<vec2f, 3>(
        vec2f(-1.0, -3.0),
        vec2f(-1.0,  1.0),
        vec2f( 3.0,  1.0),
    );
    let uv = array<vec2f, 3>(
        vec2f(0.0, 2.0),
        vec2f(0.0, 0.0),
        vec2f(2.0, 0.0),
    );
    var out: VsOut;
    out.clip_pos = vec4f(pos[vid], 0.0, 1.0);
    out.uv = uv[vid];
    return out;
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4f {
    let src = textureSample(t_input, t_sampler, in.uv);
    let r = dot(u.row_r, src) + u.constants.r;
    let g = dot(u.row_g, src) + u.constants.g;
    let b = dot(u.row_b, src) + u.constants.b;
    let a = dot(u.row_a, src) + u.constants.a;
    return vec4f(r, g, b, a);
}
