// mask_compose.wgsl — sample a foreground RT and a mask RT, output
// `(fg.rgb, fg.a * mask.a)`. The "mask × foreground" primitive that
// replaces the inline-SDF clip path for vector-driven masks
// (M-VEC.4..6 / AUT-56..58).
//
// This shader is *only* composition. The mask itself is generated
// upstream by `mask_texture.wgsl` (analytic SDF) or
// `path_mask_texture.wgsl` (polygon).

@group(0) @binding(0) var t_foreground: texture_2d<f32>;
@group(0) @binding(1) var t_mask: texture_2d<f32>;
@group(0) @binding(2) var s_sampler: sampler;

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
    let fg = textureSample(t_foreground, s_sampler, in.uv);
    let mask = textureSample(t_mask, s_sampler, in.uv);
    return vec4<f32>(fg.rgb, fg.a * mask.a);
}
