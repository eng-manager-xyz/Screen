// filter_drop_shadow_extract.wgsl
// Reads the source alpha at a UV-offset position and outputs it tinted
// with the shadow color. Used as the first pass of DropShadowFilter, before
// the separable Gaussian.

struct ExtractUniforms {
    offset: vec2f,   // texel offsets to shift the alpha (the shadow direction)
    color: vec4f,    // shadow color (rgb + alpha multiplier)
}

struct VsOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var<uniform> u: ExtractUniforms;
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
    let dims = vec2f(textureDimensions(t_input));
    let texel = 1.0 / dims;
    let sample_uv = in.uv - u.offset * texel;
    let src = textureSample(t_input, t_sampler, sample_uv);
    return vec4f(u.color.rgb, src.a * u.color.a);
}
