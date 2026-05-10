// path_clip.wgsl — sample a foreground RT and multiply its alpha by
// a point-in-polygon test against a closed polygon (M-MASK / AUT-35
// freehand path mask).
//
// The polygon is stored in `u.points[0..u.count]` in NDC. Up to 32
// vertices for V1 (uniform buffer size limit). Hard edges — no SDF
// AA in this version; the upstream rasterization already produces
// integer-pixel-accurate boundaries via the standard crossing-test
// (Jordan curve theorem). AA can be retrofitted later via a
// distance-to-nearest-edge approximation.

const MAX_POINTS: u32 = 32u;

@group(0) @binding(0) var t_foreground: texture_2d<f32>;
@group(0) @binding(1) var s_sampler: sampler;

struct Uniforms {
    /// Number of valid points in `points[0..count]`.
    count: u32,
    /// 0 = mask normally (opaque inside polygon, transparent outside).
    /// 1 = inverse mask (transparent inside, opaque outside).
    invert: u32,
    _pad: vec2<u32>,
    points: array<vec4<f32>, MAX_POINTS>,
};
@group(0) @binding(2) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn main_vs(@builtin(vertex_index) vi: u32) -> VsOut {
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi & 2u) * 2 - 1);
    var out: VsOut;
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return out;
}

/// Standard crossings-test point-in-polygon. Returns true when `p`
/// is inside the closed polygon `points[0..count]`.
fn point_in_polygon(p: vec2<f32>) -> bool {
    var winding: i32 = 0;
    if (u.count < 3u) {
        return false;
    }
    for (var i: u32 = 0u; i < u.count; i = i + 1u) {
        let j: u32 = select(i + 1u, 0u, i + 1u >= u.count);
        let a = u.points[i].xy;
        let b = u.points[j].xy;
        // Half-open horizontal ray from p going +X. The edge crosses
        // the ray iff (a.y > p.y) != (b.y > p.y), at x where the
        // edge's parametric form intersects y = p.y.
        let cond_y = (a.y > p.y) != (b.y > p.y);
        if (cond_y) {
            let dy = b.y - a.y;
            // Avoid div-by-zero — cond_y already guarantees dy != 0.
            let x_cross = (b.x - a.x) * (p.y - a.y) / dy + a.x;
            if (p.x < x_cross) {
                winding = winding + 1;
            }
        }
    }
    return (winding & 1) == 1;
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4<f32> {
    let ndc = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    var inside = point_in_polygon(ndc);
    if (u.invert == 1u) {
        inside = !inside;
    }
    let mask: f32 = select(0.0, 1.0, inside);
    let fg = textureSample(t_foreground, s_sampler, in.uv);
    return vec4<f32>(fg.rgb, fg.a * mask);
}
