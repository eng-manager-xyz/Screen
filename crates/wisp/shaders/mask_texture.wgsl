// mask_texture.wgsl — generate a single-purpose alpha-mask texture
// from `MaskShape` data (M-DYN.1 / AUT-43). No foreground texture,
// no composition logic — just coverage.
//
// Output is `vec4(m, m, m, m)` where `m ∈ [0, 1]` is SDF coverage.
// Storing the same value in RGB and A means the texture works both
// as an alpha-multiply matte (sample `.a`) and as a grayscale
// silhouette (sample `.r` or display via alpha blending).

struct Uniforms {
    /// Center of the shape, NDC.
    center: vec2<f32>,
    /// Half-extents of the bounding rect, NDC.
    half_extents: vec2<f32>,
    /// Corner radius (rounded-rect mode), NDC. Render-side clamps to
    /// half the smaller side.
    radius: f32,
    /// Anti-alias band width in NDC units. ~2/min(w, h) for ~1 px.
    aa: f32,
    /// 0.0 = mask normally (opaque inside, transparent outside).
    /// 1.0 = inverse mask.
    invert: f32,
    /// 0.0 = rounded-rect / rect / circle (degenerate cases of one
    /// SDF). 1.0 = ellipse (anisotropic SDF).
    shape_kind: f32,
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

fn sdf_rounded_rect(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn sdf_ellipse(p: vec2<f32>, half: vec2<f32>) -> f32 {
    let s = p / max(half, vec2<f32>(1e-6));
    let inside = dot(s, s) - 1.0;
    return inside * min(half.x, half.y);
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4<f32> {
    let ndc = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    let local = ndc - u.center;
    var d: f32;
    if (u.shape_kind > 0.5) {
        d = sdf_ellipse(local, u.half_extents);
    } else {
        d = sdf_rounded_rect(local, u.half_extents, u.radius);
    }
    var m = clamp(0.5 - d / max(u.aa, 1e-6), 0.0, 1.0);
    if (u.invert > 0.5) {
        m = 1.0 - m;
    }
    return vec4<f32>(m, m, m, m);
}
