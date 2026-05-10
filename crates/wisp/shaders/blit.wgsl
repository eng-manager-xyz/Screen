// Minimal "fullscreen sampler" shader. Used by the auto-dispatch
// renderer to blit a `RenderTexture` onto an arbitrary target view at
// the end of a frame.

@group(0) @binding(0) var t_source: texture_2d<f32>;
@group(0) @binding(1) var s_sampler: sampler;

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
    return textureSample(t_source, s_sampler, in.uv);
}
