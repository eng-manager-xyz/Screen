// graphics_solid.wgsl
// Instanced SDF primitive renderer covering rect / rounded rect / ellipse /
// line / annular sector, with solid + linear + radial gradient fills.
// Stroked primitives emit a second instance with mode=1 to render the
// outline band.
//
// Per-instance:
//   model         — mat4×4 transform from primitive-local to clip space
//   color, color_b — fill colors (solid uses `color` only; gradients blend a→b)
//   half_extents  — (width/2, height/2) — also (r_outer, r_outer) for AS
//   radius        — corner radius (rect only; ignored for ellipse / AS)
//   stroke_width  — outline thickness in primitive-local units (mode=1 only)
//   grad_a        — gradient start (linear) or center (radial)
//   grad_b        — gradient end (linear) or (radius, _) (radial)
//   kind          — 0 = rounded rect, 1 = ellipse, 2 = annular sector
//   mode          — 0 = filled interior, 1 = outline band
//   fill_kind     — 0 = solid, 1 = linear gradient, 2 = radial gradient
//   arc_radii     — (r_inner, r_outer) for annular sectors
//   arc_angles    — (mid_angle, half_angle) — wedge centerline +
//                   half-span (radians, CCW from +x)

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
    /// `(kind, mode, fill_kind, _padding)` packed.
    @location(11) kind_pack: vec4<u32>,
    /// `(r_inner, r_outer, mid_angle, half_angle)` for annular
    /// sectors. Zeroed for other kinds.
    @location(12) arc_data: vec4f,
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
    @location(11) @interpolate(flat) arc_data: vec4f,
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
    let kind = in.kind_pack.x;
    let mode = in.kind_pack.y;
    let fill_kind = in.kind_pack.z;
    let expand = select(0.0, in.stroke_width * 0.5, mode == 1u);
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
    out.kind = kind;
    out.mode = mode;
    out.fill_kind = fill_kind;
    out.arc_data = in.arc_data;
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

// SDF for an annular sector. Wedge axis aligns with +y in the
// rotated frame; we rotate the local point by `(π/2 - mid_angle)`
// CCW so the wedge centerline lands on +y. After that, IQ-style
// formulas compute distance.
//
// Handles three geometric cases naturally:
// * `r_inner = 0`        — pie slice (disc interior at origin).
// * `r_inner > 0`        — annular band (hole at origin).
// * `half_angle ≥ π`     — full ring / disc (entire angular range).
//
// Returns positive distance outside the band; negative inside.
fn sdf_annular_sector(
    p_local: vec2f,
    r_inner: f32,
    r_outer: f32,
    mid_angle: f32,
    half_angle: f32,
) -> f32 {
    // Rotate by (π/2 - mid_angle) CCW so wedge axis = +y.
    let theta = 1.5707963267948966 - mid_angle;
    let ct = cos(theta);
    let st = sin(theta);
    let p = vec2f(p_local.x * ct - p_local.y * st, p_local.x * st + p_local.y * ct);

    // Mirror around +y axis — wedge is symmetric about that axis
    // after rotation, so abs(x) lets us handle just the +x half.
    let p_abs = vec2f(abs(p.x), p.y);

    // Inside / outside wedge angular range. `sc` is the unit
    // outward-normal direction of the wedge edge in the
    // mirrored frame.
    let sc = vec2f(sin(half_angle), cos(half_angle));
    let outside_wedge = sc.y * p_abs.x - sc.x * p_abs.y;

    let r = length(p_abs);

    if (outside_wedge <= 0.0) {
        // Inside the angular wedge. Distance to nearest radial
        // boundary:
        // - r_inner = 0 → only the outer arc is a boundary; the
        //   primitive is filled from origin to r_outer.
        // - r_inner > 0 → an annulus; distance is `max(r - r_outer,
        //   r_inner - r)` so the result is signed correctly across
        //   the inner hole / band / outside transition.
        if (r_inner < 1e-6) {
            return r - r_outer;
        }
        return max(r - r_outer, r_inner - r);
    }

    // Outside the angular wedge. Closest point is on the radial
    // wedge edge (a line segment from `sc * r_inner` to
    // `sc * r_outer`). Distance is always non-negative.
    let edge_pt = sc * clamp(dot(p_abs, sc), r_inner, r_outer);
    return length(p_abs - edge_pt);
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
    } else if (in.kind == 1u) {
        d = sdf_ellipse(in.local_pos, in.half_extents);
    } else {
        d = sdf_annular_sector(
            in.local_pos,
            in.arc_data.x,
            in.arc_data.y,
            in.arc_data.z,
            in.arc_data.w,
        );
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
