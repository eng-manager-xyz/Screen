// material_palette.wgsl — W3D.4 reference material.
//
// Port of the engmanager.xyz 404 fragment shader:
//   - 5-stop palette ramp driven by position along a fixed diagonal
//   - warm-band overlay (smoothstep difference along the same diagonal)
//   - fake directional lambert (matches Render3DPass's default)
//   - rim (1 - |dot(normal, +Z)|)^2
//   - value-noise grain in screen space
//
// Shares the same ViewProj / Model bind groups as the default mesh
// pipeline, plus a per-material `Palette` UBO at group 2 binding 0
// with the time uniform and 5 stop colors.

struct ViewProj {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct Model {
    matrix: mat4x4<f32>,
    tint: vec4<f32>,
}

struct Palette {
    // 5 stop colors in order: peach, maroon, pink, mauve, blue (RGBA).
    stops: array<vec4<f32>, 5>,
    // .x = elapsed seconds, .yzw padding for 16-byte align.
    time: vec4<f32>,
}

@group(0) @binding(0) var<uniform> view_proj: ViewProj;
@group(1) @binding(0) var<uniform> model: Model;
@group(2) @binding(0) var<uniform> palette: Palette;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) local_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
}

@vertex
fn main_vs(in: VsIn) -> VsOut {
    let world_pos4 = model.matrix * vec4<f32>(in.position, 1.0);
    let n_world = normalize((model.matrix * vec4<f32>(in.normal, 0.0)).xyz);
    var out: VsOut;
    out.clip_pos = view_proj.view_proj * world_pos4;
    out.local_pos = in.position; // palette ramp keys off local-space coords
    out.world_normal = n_world;
    return out;
}

// Hash + value-noise port of the 404 shader's hash() / noise().
fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    var f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i),                       hash(i + vec2<f32>(1.0, 0.0)), f.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), f.x),
        f.y
    );
}

// 5-stop ramp matching the JS `palette(t)` switch.
fn ramp(t: f32) -> vec3<f32> {
    let s0 = palette.stops[0].rgb; // peach
    let s1 = palette.stops[1].rgb; // maroon
    let s2 = palette.stops[2].rgb; // pink
    let s3 = palette.stops[3].rgb; // mauve
    let s4 = palette.stops[4].rgb; // blue
    let u = fract(t);
    if (u < 0.22) {
        return mix(s0, s1, smoothstep(0.0, 0.22, u));
    } else if (u < 0.44) {
        return mix(s1, s2, smoothstep(0.22, 0.44, u));
    } else if (u < 0.65) {
        return mix(s2, s3, smoothstep(0.44, 0.65, u));
    } else if (u < 0.84) {
        return mix(s3, s4, smoothstep(0.65, 0.84, u));
    }
    return mix(s4, s0, smoothstep(0.84, 1.0, u));
}

@fragment
fn main_fs(in: VsOut) -> @location(0) vec4<f32> {
    let time = palette.time.x;
    let v_pos = in.local_pos;

    let diagonal = dot(v_pos, normalize(vec3<f32>(0.95, 0.52, -0.38)));
    let t = diagonal * 0.28 + 0.58 + sin(time * 0.24) * 0.04;
    var color = ramp(t);

    let band_pos = v_pos.y + v_pos.x * 0.3 - v_pos.z * 0.16;
    let warm = smoothstep(-0.14, 0.1, band_pos) * (1.0 - smoothstep(0.28, 0.7, band_pos));
    color = mix(color, palette.stops[0].rgb, warm * 0.55);

    let n = normalize(in.world_normal);
    let facing = clamp(dot(n, normalize(vec3<f32>(-0.25, 0.55, 0.78))), 0.0, 1.0);
    color *= 0.68 + facing * 0.5;

    let rim = pow(1.0 - abs(dot(n, vec3<f32>(0.0, 0.0, 1.0))), 2.0);
    color = mix(color, vec3<f32>(1.0, 1.0, 1.0), rim * 0.14);

    let grain = (noise(in.clip_pos.xy * 0.72 + vec2<f32>(time * 11.0)) - 0.5) * 0.075;
    color += vec3<f32>(grain);

    return vec4<f32>(color, model.tint.a * 0.98);
}
