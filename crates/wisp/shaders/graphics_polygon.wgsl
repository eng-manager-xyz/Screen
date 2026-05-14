// graphics_polygon.wgsl
// Triangle-list pipeline for `Graphics::draw_polygon`. Each polygon
// is fan-triangulated CPU-side (convex assumption) and emitted as a
// flat list of per-vertex (position, color) tuples — the world
// matrix is already baked into the position, and the color is
// per-vertex (uniform across a polygon's triangles, but per-vertex
// in the format so the same shader handles a future per-vertex
// gradient mode without a layout change).

struct VsIn {
    @location(0) position: vec2f,
    @location(1) color: vec4f,
}

struct VsOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) color: vec4f,
}

@vertex
fn main_vs(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip_pos = vec4f(in.position, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4f {
    return in.color;
}
