// mesh_default.wgsl — W3D.3 default shader for `Render3DPass`.
//
// Vertex: pass through position/normal through (model × view_proj),
//         transform the normal by the upper-3x3 of the model matrix.
// Fragment: flat lambert with a fixed directional light matching the
//           `vec3(-0.25, 0.55, 0.78)` term in the engmanager.xyz 404
//           shader, mixed against a tint color uniform. No textures.

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
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
}

@vertex
fn main_vs(in: VsIn) -> VsOut {
    let world_pos4 = model.matrix * vec4<f32>(in.position, 1.0);
    let world_pos = world_pos4.xyz;
    // Approximate normal-matrix as the upper-3x3 of the model matrix.
    // Correct for rigid-body transforms (rotation + translation +
    // uniform scale). Non-uniform scale would need a full
    // transpose(inverse(M)). Out of scope for W3D.3.
    let n_world = normalize((model.matrix * vec4<f32>(in.normal, 0.0)).xyz);

    var out: VsOut;
    out.clip_pos = view_proj.view_proj * world_pos4;
    out.world_pos = world_pos;
    out.world_normal = n_world;
    return out;
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4<f32> {
    // Directional light matching the 404 shader.
    let light_dir = normalize(vec3<f32>(-0.25, 0.55, 0.78));
    let lambert = clamp(dot(normalize(in.world_normal), light_dir), 0.0, 1.0);
    // Same intensity ramp as the 404 shader: 0.68 base + 0.5×lambert.
    let shade = 0.68 + lambert * 0.5;
    let base = model.tint.rgb * shade;
    return vec4<f32>(base, model.tint.a);
}
