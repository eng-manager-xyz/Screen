// graphics_solid.wgsl
// Instanced SDF primitive renderer covering rect / rounded rect / ellipse /
// line, with solid + linear + radial gradient fills. Stroked primitives emit
// a second instance with mode=1 to render the outline band.
//
// Per-instance:
//   model         — mat4×4 transform from primitive-local to clip space
//   color, color_b — fill colors (solid uses `color` only; gradients blend a→b)
//   half_extents  — (width/2, height/2) or (radius_x, radius_y) for ellipse
//   radius        — corner radius (rect only; ignored for ellipse)
//   stroke_width  — outline thickness in primitive-local units (mode=1 only)
//   grad_a        — gradient start (linear) or center (radial)
//   grad_b        — gradient end (linear) or (radius, _) (radial)
//   kind          — 0 = rounded rect, 1 = ellipse
//   mode          — 0 = filled interior, 1 = outline band
//   fill_kind     — 0 = solid, 1 = linear gradient, 2 = radial gradient

struct VsIn {
    @builtin(vertex_index) vid: u32,
    @location(0)  model_0: vec4f,
    @location(1)  model_1: vec4f,
    @location(2)  model_2: vec4f,
    @location(3)  model_3: vec4f,
    @location(4)  color: vec4f,
    @location(5)  color_b: vec4f,
    @location(6)  half_extents: vec2f,
    @location(7)  radius: f32,
    @location(8)  stroke_width: f32,
    @location(9)  grad_a: vec2f,
    @location(10) grad_b: vec2f,
    @location(11) kind: u32,
    @location(12) mode: u32,
    @location(13) fill_kind: u32,
}

struct VsOut {
    @builtin(position) clip_pos: vec4f,
    @location(0) color: vec4f,
    @location(1) color_b: vec4f,
    @location(2) local_pos: vec2f,
    @location(3) half_extents: vec2f,
    @location(4) radius: f32,
    @location(5) stroke_width: f32,
    @location(6) grad_a: vec2f,
    @location(7) grad_b: vec2f,
    @location(8) @interpolate(flat) kind: u32,
    @location(9) @interpolate(flat) mode: u32,
    @location(10) @interpolate(flat) fill_kind: u32,
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
    let expand = select(0.0, in.stroke_width * 0.5, in.mode == 1u);
    let extents = in.half_extents + vec2f(expand);
    let local = unit[in.vid] * extents;
    let model = mat4x4f(in.model_0, in.model_1, in.model_2, in.model_3);

    var out: VsOut;
    out.clip_pos = model * vec4f(local, 0.0, 1.0);
    out.color = in.color;
    out.color_b = in.color_b;
    out.local_pos = local;
    out.half_extents = in.half_extents;
    out.radius = in.radius;
    out.stroke_width = in.stroke_width;
    out.grad_a = in.grad_a;
    out.grad_b = in.grad_b;
    out.kind = in.kind;
    out.mode = in.mode;
    out.fill_kind = in.fill_kind;
    return out;
}

fn sdf_rounded_rect(p: vec2f, h: vec2f, r: f32) -> f32 {
    let d = abs(p) - h + vec2f(r);
    return length(max(d, vec2f(0.0))) + min(max(d.x, d.y), 0.0) - r;
}

fn sdf_ellipse(p: vec2f, r: vec2f) -> f32 {
    let pr = p / max(r, vec2f(1e-6));
    return (length(pr) - 1.0) * min(r.x, r.y);
}

fn evaluate_fill(in: VsOut) -> vec4f {
    if (in.fill_kind == 1u) {
        // Linear gradient: project local_pos onto (grad_a → grad_b).
        let dir = in.grad_b - in.grad_a;
        let len_sq = max(dot(dir, dir), 1e-6);
        let t = clamp(dot(in.local_pos - in.grad_a, dir) / len_sq, 0.0, 1.0);
        return mix(in.color, in.color_b, t);
    } else if (in.fill_kind == 2u) {
        // Radial gradient: grad_a = center, grad_b.x = radius.
        let r = max(in.grad_b.x, 1e-6);
        let t = clamp(length(in.local_pos - in.grad_a) / r, 0.0, 1.0);
        return mix(in.color, in.color_b, t);
    }
    return in.color;
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
        alpha = clamp(0.5 - d / aa, 0.0, 1.0);
    } else {
        let band = abs(d) - in.stroke_width * 0.5;
        alpha = clamp(0.5 - band / aa, 0.0, 1.0);
    }

    let fill_color = evaluate_fill(in);
    return vec4f(fill_color.rgb, fill_color.a * alpha);
}
