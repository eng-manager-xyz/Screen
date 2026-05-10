// advanced_blend.wgsl — TEMPLATE for the 21 advanced blend modes (Tier C).
//
// One full-screen triangle samples both `t_backdrop` (the previously-
// rendered destination) and `t_foreground` (this node's contribution
// rendered into its own RT), runs `blend_fn` per channel, and writes
// the composited result.
//
// The Rust side substitutes a per-mode `blend_fn(base, blend) -> vec3<f32>`
// body in place of the placeholder marker below. The Rust template
// resolver in `render/advanced_blend.rs` is the only place that
// generates valid WGSL from this file.

@group(0) @binding(0) var t_backdrop: texture_2d<f32>;
@group(0) @binding(1) var t_foreground: texture_2d<f32>;
@group(0) @binding(2) var s_sampler: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn main_vs(@builtin(vertex_index) vi: u32) -> VsOut {
    // Single-triangle fullscreen quad — covers NDC [-1, +1].
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi & 2u) * 2 - 1);
    var out: VsOut;
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    // UV in [0,1] with y flipped so the texture isn't upside down.
    out.uv = vec2<f32>((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return out;
}

// ─── HSL helpers (used by Saturation / Color / Luminosity) ────────────

fn lum(c: vec3<f32>) -> f32 {
    return 0.3 * c.r + 0.59 * c.g + 0.11 * c.b;
}

fn clip_color(c: vec3<f32>) -> vec3<f32> {
    let l = lum(c);
    let n = min(min(c.r, c.g), c.b);
    let x = max(max(c.r, c.g), c.b);
    var out = c;
    if (n < 0.0) {
        out = l + (((out - l) * l) / (l - n));
    }
    if (x > 1.0) {
        out = l + (((out - l) * (1.0 - l)) / (x - l));
    }
    return out;
}

fn set_lum(c: vec3<f32>, l: f32) -> vec3<f32> {
    let d = l - lum(c);
    return clip_color(c + vec3<f32>(d));
}

fn sat(c: vec3<f32>) -> f32 {
    return max(max(c.r, c.g), c.b) - min(min(c.r, c.g), c.b);
}

fn set_sat(c: vec3<f32>, s: f32) -> vec3<f32> {
    let cmin = min(min(c.r, c.g), c.b);
    let cmax = max(max(c.r, c.g), c.b);
    if (cmax > cmin) {
        return ((c - vec3<f32>(cmin)) * s) / (cmax - cmin);
    }
    return vec3<f32>(0.0);
}

// ─── PER-MODE BLEND FUNCTION (substituted by Rust) ────────────────────
// __BLEND_FN_PLACEHOLDER__

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4<f32> {
    let backdrop = textureSample(t_backdrop, s_sampler, in.uv);
    let foreground = textureSample(t_foreground, s_sampler, in.uv);

    let blended = blend_fn(backdrop.rgb, foreground.rgb);

    // PixiJS-style compositing: the foreground.alpha controls how much
    // of the blended result overrides the backdrop. Output alpha is
    // source-over (`fg.a + bg.a·(1 - fg.a)`).
    let mixed_rgb = mix(backdrop.rgb, blended, foreground.a);
    let out_alpha = foreground.a + backdrop.a * (1.0 - foreground.a);
    return vec4<f32>(mixed_rgb, out_alpha);
}
