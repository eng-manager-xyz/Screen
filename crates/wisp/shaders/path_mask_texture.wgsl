// path_mask_texture.wgsl — generate an alpha-mask texture from a
// closed polygon (M-DYN.1 / AUT-43, freehand path variant).
//
// Sister to `mask_texture.wgsl`; runs the same crossings-test
// point-in-polygon as `path_clip.wgsl` but emits coverage directly
// instead of multiplying it into a foreground sample.

const MAX_POINTS: u32 = 32u;

struct Uniforms {
    count: u32,
    invert: u32,
    _pad: vec2<u32>,
    points: array<vec4<f32>, MAX_POINTS>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

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

fn point_in_polygon(p: vec2<f32>) -> bool {
    var winding: i32 = 0;
    if (u.count < 3u) {
        return false;
    }
    for (var i: u32 = 0u; i < u.count; i = i + 1u) {
        let j: u32 = select(i + 1u, 0u, i + 1u >= u.count);
        let a = u.points[i].xy;
        let b = u.points[j].xy;
        let cond_y = (a.y > p.y) != (b.y > p.y);
        if (cond_y) {
            let dy = b.y - a.y;
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
    let m: f32 = select(0.0, 1.0, inside);
    return vec4<f32>(m, m, m, m);
}
