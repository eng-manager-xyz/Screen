// graphics_solid.wgsl
// Instanced solid-fill rect / rounded-rect renderer.
//
// One unified shader: `radius == 0.0` produces a sharp axis-aligned rect.
// SDF + screen-space AA gives clean edges at any zoom.
//
// Per-instance:
//   model        — mat4x4 transform from primitive-local to clip space
//   color        — fill color (linear srgb f32)
//   half_extents — (width/2, height/2) in primitive-local coords
//   radius       — corner radius (0 = sharp)

struct VsIn {
    @builtin(vertex_index) vid: u32,
    @location(0) model_0: vec4f,
    @location(1) model_1: vec4f,
    @location(2) model_2: vec4f,
    @location(3) model_3: vec4f,
    @location(4) color: vec4f,
    @location(5) half_extents: vec2f,
    @location(6) radius: f32,
}

struct VsOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) color: vec4f,
    @location(1) local_pos: vec2f,
    @location(2) half_extents: vec2f,
    @location(3) radius: f32,
}

@vertex
fn main_vs(in: VsIn) -> VsOut {
    let unit = array<vec2f, 6>(
        vec2f(-1.0, -1.0),
        vec2f( 1.0, -1.0),
        vec2f(-1.0,  1.0),
        vec2f( 1.0, -1.0),
        vec2f( 1.0,  1.0),
        vec2f(-1.0,  1.0),
    );
    let local = unit[in.vid] * in.half_extents;
    let model = mat4x4f(in.model_0, in.model_1, in.model_2, in.model_3);

    var out: VsOut;
    out.clip_pos = model * vec4f(local, 0.0, 1.0);
    out.color = in.color;
    out.local_pos = local;
    out.half_extents = in.half_extents;
    out.radius = in.radius;
    return out;
}

fn sdf_rounded_rect(p: vec2f, h: vec2f, r: f32) -> f32 {
    let d = abs(p) - h + vec2f(r);
    return length(max(d, vec2f(0.0))) + min(max(d.x, d.y), 0.0) - r;
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4f {
    let d = sdf_rounded_rect(in.local_pos, in.half_extents, in.radius);
    let aa = max(fwidth(d), 1e-6);
    let alpha = clamp(0.5 - d / aa, 0.0, 1.0);
    return vec4f(in.color.rgb, in.color.a * alpha);
}
