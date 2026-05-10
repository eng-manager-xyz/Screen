// clip.wgsl — sample a foreground RT and multiply its alpha by the
// rounded-rect signed-distance-field mask (M-MASK.1 / AUT-31).
//
// Coordinates: NDC `[-1, +1]²`. The mask's `center` and `half_extents`
// are in NDC units; UV from the fullscreen-triangle vertex is converted
// to NDC space at fragment time.

@group(0) @binding(0) var t_foreground: texture_2d<f32>;
@group(0) @binding(1) var s_sampler: sampler;

struct Uniforms {
    /// Center of the mask rect, NDC.
    center: vec2<f32>,
    /// Half-extents of the rect, NDC (rect.w/2, rect.h/2).
    half_extents: vec2<f32>,
    /// Corner radius, NDC units. Render-side clamps to min(half_extents).
    radius: f32,
    /// Anti-alias band width in NDC units. ~2/min(viewport_w, viewport_h)
    /// gives a 1-px band; the renderer computes this from the output
    /// RT's dimensions.
    aa: f32,
    /// 0.0 = mask normally (opaque inside the shape, transparent
    /// outside). 1.0 = inverse mask (transparent inside, opaque
    /// outside) — used by the spotlight / dim-outside primitives.
    invert: f32,
    _pad: f32,
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

/// Standard SDF for a rounded rectangle centered at origin.
/// Negative inside, positive outside.
fn sdf_rounded_rect(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4<f32> {
    // UV → NDC. UV (0,0) is top-left; (0,1) is bottom-left.
    let ndc = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    let local = ndc - u.center;
    let d = sdf_rounded_rect(local, u.half_extents, u.radius);

    // Anti-alias: smoothstep across a `aa`-width band straddling d=0.
    // mask = 1 inside, 0 outside, blended in the band.
    var mask = clamp(0.5 - d / max(u.aa, 1e-6), 0.0, 1.0);
    if (u.invert > 0.5) {
        mask = 1.0 - mask;
    }

    let fg = textureSample(t_foreground, s_sampler, in.uv);
    return vec4<f32>(fg.rgb, fg.a * mask);
}
