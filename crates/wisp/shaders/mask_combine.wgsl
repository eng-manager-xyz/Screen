// mask_combine.wgsl — boolean operations on alpha-mask textures
// (M-VEC.11 / AUT-63).
//
// Op encoding (uniform.op):
//   0 = Union     → max(a, b)
//   1 = Intersect → a * b
//   2 = Subtract  → a * (1 - b)
//
// Output stores `(m, m, m, m)` — same convention as the underlying
// mask textures so consumers can keep sampling alpha or RGB.

@group(0) @binding(0) var t_a: texture_2d<f32>;
@group(0) @binding(1) var t_b: texture_2d<f32>;
@group(0) @binding(2) var s_sampler: sampler;

struct Uniforms {
    op: u32,
    _pad: vec3<u32>,
};
@group(0) @binding(3) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn main_vs(@builtin(vertex_index) vi: u32) -> VsOut {
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi & 2u) * 2 - 1);
    var out: VsOut;
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return out;
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(t_a, s_sampler, in.uv).a;
    let b = textureSample(t_b, s_sampler, in.uv).a;
    var m: f32;
    if (u.op == 1u) {
        m = a * b;                  // Intersect
    } else if (u.op == 2u) {
        m = a * (1.0 - b);          // Subtract (a minus b)
    } else {
        m = max(a, b);              // Union (default)
    }
    m = clamp(m, 0.0, 1.0);
    return vec4<f32>(m, m, m, m);
}
