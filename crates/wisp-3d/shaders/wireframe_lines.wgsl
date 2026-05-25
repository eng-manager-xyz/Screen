// wireframe_lines.wgsl — W3D.5 line-list wireframe overlay.
//
// Per-vertex `position: vec3<f32>`. View-proj at group 0 binding 0.
// Color + opacity uniform at group 1 binding 0.

struct ViewProj {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct LineColor {
    // .rgba — alpha is honoured in the fragment.
    color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> view_proj: ViewProj;
@group(1) @binding(0) var<uniform> line: LineColor;

struct VsIn {
    @location(0) position: vec3<f32>,
}

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
}

@vertex
fn main_vs(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip_pos = view_proj.view_proj * vec4<f32>(in.position, 1.0);
    return out;
}

@fragment
fn main_fs(_in: VsOut) -> @location(0) vec4<f32> {
    return line.color;
}
