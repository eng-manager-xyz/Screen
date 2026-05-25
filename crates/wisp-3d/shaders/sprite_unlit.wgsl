// sprite_unlit.wgsl — W3D.6 unlit alpha-blended primitive.
//
// Same vertex layout as Mesh3D (position + normal) so Sprite3D can
// share the vertex buffer machinery. Normal is unused — we discard
// it in the vertex stage so the sprite is genuinely unlit.

struct ViewProj {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct Model {
    matrix: mat4x4<f32>,
    tint: vec4<f32>,
}

@group(0) @binding(0) var<uniform> view_proj: ViewProj;
@group(1) @binding(0) var<uniform> model: Model;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
}

@vertex
fn main_vs(in: VsIn) -> VsOut {
    let world_pos4 = model.matrix * vec4<f32>(in.position, 1.0);
    let _ignored_normal = in.normal;
    var out: VsOut;
    out.clip_pos = view_proj.view_proj * world_pos4;
    return out;
}

@fragment
fn main_fs(_in: VsOut) -> @location(0) vec4<f32> {
    return model.tint;
}
