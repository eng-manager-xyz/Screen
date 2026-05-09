// graphics_solid.wgsl
// Instanced SDF primitive renderer covering rect / rounded rect / ellipse /
// line (line is just a rotated rect). Stroked primitives emit a second
// instance with mode=1 to render the outline band.
//
// Per-instance:
//   model         — mat4×4 transform from primitive-local to clip space
//   color         — fill or stroke color (linear srgb f32)
//   half_extents  — (width/2, height/2) or (radius_x, radius_y) for ellipse
//   radius        — corner radius (rect only; ignored for ellipse)
//   stroke_width  — outline thickness in primitive-local units (mode=1 only)
//   kind          — 0 = rounded rect, 1 = ellipse
//   mode          — 0 = filled interior, 1 = outline band

struct VsIn {
    @builtin(vertex_index) vid: u32,
    @location(0) model_0: vec4f,
    @location(1) model_1: vec4f,
    @location(2) model_2: vec4f,
    @location(3) model_3: vec4f,
    @location(4) color: vec4f,
    @location(5) half_extents: vec2f,
    @location(6) radius: f32,
    @location(7) stroke_width: f32,
    @location(8) kind: u32,
    @location(9) mode: u32,
}

struct VsOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) color: vec4f,
    @location(1) local_pos: vec2f,
    @location(2) half_extents: vec2f,
    @location(3) radius: f32,
    @location(4) stroke_width: f32,
    @location(5) @interpolate(flat) kind: u32,
    @location(6) @interpolate(flat) mode: u32,
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
    // Outline mode expands the bounding box by stroke_width/2 on each side
    // so the band has room to render.
    let expand = select(0.0, in.stroke_width * 0.5, in.mode == 1u);
    let extents = in.half_extents + vec2f(expand);
    let local = unit[in.vid] * extents;
    let model = mat4x4f(in.model_0, in.model_1, in.model_2, in.model_3);

    var out: VsOut;
    out.clip_pos = model * vec4f(local, 0.0, 1.0);
    out.color = in.color;
    out.local_pos = local;
    out.half_extents = in.half_extents;
    out.radius = in.radius;
    out.stroke_width = in.stroke_width;
    out.kind = in.kind;
    out.mode = in.mode;
    return out;
}

fn sdf_rounded_rect(p: vec2f, h: vec2f, r: f32) -> f32 {
    let d = abs(p) - h + vec2f(r);
    return length(max(d, vec2f(0.0))) + min(max(d.x, d.y), 0.0) - r;
}

// Approximate ellipse SDF — scales the point into circle space and back.
// Visually correct for moderate eccentricities.
fn sdf_ellipse(p: vec2f, r: vec2f) -> f32 {
    let pr = p / max(r, vec2f(1e-6));
    return (length(pr) - 1.0) * min(r.x, r.y);
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4f {
    var d: f32;
    if (in.kind == 0u) {
        d = sdf_rounded_rect(in.local_pos, in.half_extents, in.radius);
    } else {
        d = sdf_ellipse(in.local_pos, in.half_extents);
    }

    let aa = max(fwidth(d), 1e-6);
    var alpha: f32;
    if (in.mode == 0u) {
        // Filled interior: alpha rises as d goes negative.
        alpha = clamp(0.5 - d / aa, 0.0, 1.0);
    } else {
        // Outline band centered on d=0 with half-width stroke_width/2.
        let band = abs(d) - in.stroke_width * 0.5;
        alpha = clamp(0.5 - band / aa, 0.0, 1.0);
    }
    return vec4f(in.color.rgb, in.color.a * alpha);
}
