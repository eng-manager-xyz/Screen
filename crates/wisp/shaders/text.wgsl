// text.wgsl
// Instanced bitmap-glyph renderer. Each instance is one glyph; we sample the
// atlas alpha and tint with the text color.
//
// Per-instance:
//   model     — mat4×4 transform from unit quad [-1,+1]² to glyph quad in NDC
//   color     — glyph tint (linear srgb f32)
//   uv_rect   — (u_min, v_min, u_max, v_max) within the font atlas

struct VsIn {
    @builtin(vertex_index) vid: u32,
    @location(0) model_0: vec4f,
    @location(1) model_1: vec4f,
    @location(2) model_2: vec4f,
    @location(3) model_3: vec4f,
    @location(4) color: vec4f,
    @location(5) uv_rect: vec4f,
}

struct VsOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
}

@group(0) @binding(0) var t_atlas: texture_2d<f32>;
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
    // NDC y=-1 is the visual bottom of the screen; the atlas is stored with
    // y=0 at the top. Map NDC top of quad → atlas top of glyph.
    let uvs = array<vec2f, 6>(
        vec2f(in.uv_rect.x, in.uv_rect.w),
        vec2f(in.uv_rect.z, in.uv_rect.w),
        vec2f(in.uv_rect.x, in.uv_rect.y),
        vec2f(in.uv_rect.z, in.uv_rect.w),
        vec2f(in.uv_rect.z, in.uv_rect.y),
        vec2f(in.uv_rect.x, in.uv_rect.y),
    );

    let model = mat4x4f(in.model_0, in.model_1, in.model_2, in.model_3);

    var out: VsOut;
    out.clip_pos = model * vec4f(positions[in.vid], 0.0, 1.0);
    out.uv = uvs[in.vid];
    out.color = in.color;
    return out;
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4f {
    let mask = textureSample(t_atlas, t_sampler, in.uv).a;
    return vec4f(in.color.rgb, in.color.a * mask);
}
