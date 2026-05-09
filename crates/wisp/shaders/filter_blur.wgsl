// filter_blur.wgsl
// Two-pass separable Gaussian blur (called once per direction).
// Vertex stage: full-screen triangle covering the target.
// Fragment stage: 9-tap weighted sum along `direction`.

struct BlurUniforms {
    direction: vec2f,  // (1,0) for horizontal pass; (0,1) for vertical
    radius: f32,
    _pad: f32,
}

struct VsOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var<uniform> u: BlurUniforms;
@group(1) @binding(0) var t_input: texture_2d<f32>;
@group(1) @binding(1) var t_sampler: sampler;

@vertex
fn main_vs(@builtin(vertex_index) vid: u32) -> VsOut {
    // Full-screen triangle (3 verts cover [-1,+1]^2 of NDC).
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

    // 9-tap Gaussian, sigma chosen so radius spans ~2 sigma either side.
    let weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
    var color = textureSample(t_input, t_sampler, in.uv) * weights[0];
    for (var i = 1; i < 5; i++) {
        let offset = u.direction * texel * (f32(i) * u.radius);
        color += textureSample(t_input, t_sampler, in.uv + offset) * weights[i];
        color += textureSample(t_input, t_sampler, in.uv - offset) * weights[i];
    }
    return color;
}
