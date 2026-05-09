// filter_drop_shadow_composite.wgsl
// Final pass of DropShadowFilter: alpha-over the source on top of the
// already-blurred shadow.

struct VsOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var t_shadow: texture_2d<f32>;
@group(0) @binding(1) var t_source: texture_2d<f32>;
@group(0) @binding(2) var t_sampler: sampler;

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
    let shadow = textureSample(t_shadow, t_sampler, in.uv);
    let source = textureSample(t_source, t_sampler, in.uv);

    // Alpha-over: source over shadow.
    let inv_src_a = 1.0 - source.a;
    let rgb = source.rgb + shadow.rgb * inv_src_a;
    let a = source.a + shadow.a * inv_src_a;
    return vec4f(rgb, a);
}
