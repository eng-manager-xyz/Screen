// triangle.wgsl
// Hardcoded equilateral triangle in NDC, RGB-tinted by vertex index.
// No bind groups — vertex positions emitted from `vertex_index`.
// M0.5 hello-triangle pipeline.

struct VsOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) color: vec3f,
}

@vertex
fn main_vs(@builtin(vertex_index) idx: u32) -> VsOut {
    let positions = array<vec2f, 3>(
        vec2f( 0.0,  0.6),
        vec2f(-0.6, -0.4),
        vec2f( 0.6, -0.4),
    );
    let colors = array<vec3f, 3>(
        vec3f(1.0, 0.2, 0.2),
        vec3f(0.2, 1.0, 0.2),
        vec3f(0.2, 0.2, 1.0),
    );
    var out: VsOut;
    out.clip_pos = vec4f(positions[idx], 0.0, 1.0);
    out.color = colors[idx];
    return out;
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4f {
    return vec4f(in.color, 1.0);
}
