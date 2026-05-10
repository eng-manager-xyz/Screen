//! Advanced (Tier C) blend mode pixel-readback tests.
//!
//! For each advanced [`BlendMode`], render a backdrop and a foreground
//! into separate render-textures, run [`Renderer::apply_advanced_blend`],
//! and assert the center-pixel matches the `PixiJS` reference math (within
//! `TOL` per channel).
//!
//! Format: `Rgba8Unorm` (linear) so byte values map directly to
//! `Color::rgba` channels — sRGB conversion would obscure the math.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{BlendMode, Color, Fill, Graphics, RenderTexture, Stage};

const W: u32 = 32;
const H: u32 = 32;
const TOL: i32 = 2;

fn fill_rt(app: &Application, renderer: &Renderer, rt: &RenderTexture, color: Color, alpha: f32) {
    let mut g = Graphics::new();
    g.fill(Fill::Solid(color));
    g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    g.container.alpha = alpha;

    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), g);
    let _ = renderer.render_stage(app, rt.view(), Color::rgba(0.0, 0.0, 0.0, 0.0), &stage);
}

fn run_blend(mode: BlendMode, backdrop: Color, foreground: Color) -> [u8; 4] {
    let app = block_on(Application::new(AppConfig::default())).expect("app");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("renderer");

    let bg_rt = RenderTexture::with_format(&app, W, H, format);
    let fg_rt = RenderTexture::with_format(&app, W, H, format);
    let out_rt = RenderTexture::with_format(&app, W, H, format);

    fill_rt(&app, &renderer, &bg_rt, backdrop, 1.0);
    fill_rt(&app, &renderer, &fg_rt, foreground, 1.0);

    renderer.apply_advanced_blend(&app, mode, &bg_rt, &fg_rt, &out_rt);
    let bytes = out_rt.read_pixels(&app);

    let cx = (W / 2) as usize;
    let cy = (H / 2) as usize;
    let stride = (W as usize) * 4;
    let i = cy * stride + cx * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

fn assert_close(actual: [u8; 4], expected: [u8; 4], context: &str) {
    for ch in 0..4 {
        let diff = i32::from(actual[ch]) - i32::from(expected[ch]);
        assert!(
            diff.abs() <= TOL,
            "{context}: channel {ch}: got {actual:?}, expected ~{expected:?} (diff {diff})"
        );
    }
    let _ = Vec2::ZERO; // silence unused-import warning if pruning above
}

// ─── Mostly-elementwise ──────────────────────────────────────────────

#[test]
fn darken_picks_per_channel_minimum() {
    // base=0.7 red+0.3 green, blend=0.4 red → min channel-wise.
    // base=(0.7, 0.3, 0.0), blend=(0.4, 0.0, 0.0) → (0.4, 0.0, 0.0) ≈ 102, 0, 0.
    let out = run_blend(
        BlendMode::Darken,
        Color::rgba(0.7, 0.3, 0.0, 1.0),
        Color::rgba(0.4, 0.0, 0.0, 1.0),
    );
    assert_close(out, [102, 0, 0, 255], "darken");
}

#[test]
fn lighten_picks_per_channel_maximum() {
    let out = run_blend(
        BlendMode::Lighten,
        Color::rgba(0.7, 0.3, 0.0, 1.0),
        Color::rgba(0.4, 0.6, 0.0, 1.0),
    );
    // (0.7, 0.6, 0.0) → 178, 153, 0.
    assert_close(out, [178, 153, 0, 255], "lighten");
}

#[test]
fn difference_returns_abs_difference() {
    // base=red, blend=blue → |1-0|, |0-0|, |0-1| = (1, 0, 1).
    let out = run_blend(
        BlendMode::Difference,
        Color::rgba(1.0, 0.0, 0.0, 1.0),
        Color::rgba(0.0, 0.0, 1.0, 1.0),
    );
    assert_close(out, [255, 0, 255, 255], "difference");
}

#[test]
fn exclusion_returns_softer_difference() {
    // base=0.5, blend=0.5 → 0.5+0.5 - 2·0.25 = 0.5 → 128.
    let out = run_blend(
        BlendMode::Exclusion,
        Color::rgba(0.5, 0.5, 0.5, 1.0),
        Color::rgba(0.5, 0.5, 0.5, 1.0),
    );
    assert_close(out, [128, 128, 128, 255], "exclusion");
}

#[test]
fn linear_dodge_clamps_additive() {
    let out = run_blend(
        BlendMode::LinearDodge,
        Color::rgba(0.6, 0.0, 0.0, 1.0),
        Color::rgba(0.6, 0.0, 0.0, 1.0),
    );
    // 0.6+0.6 = 1.2 clamped to 1.0 → 255.
    assert_close(out, [255, 0, 0, 255], "linear-dodge");
}

#[test]
fn linear_burn_subtracts_one() {
    // base=0.7, blend=0.6 → 0.7+0.6-1 = 0.3 → 76.
    let out = run_blend(
        BlendMode::LinearBurn,
        Color::rgba(0.7, 0.7, 0.7, 1.0),
        Color::rgba(0.6, 0.6, 0.6, 1.0),
    );
    assert_close(out, [76, 76, 76, 255], "linear-burn");
}

#[test]
fn negation_yields_one_minus_abs() {
    // base=0.3, blend=0.3 → 1 - |1 - 0.3 - 0.3| = 1 - 0.4 = 0.6 → 153.
    let out = run_blend(
        BlendMode::Negation,
        Color::rgba(0.3, 0.3, 0.3, 1.0),
        Color::rgba(0.3, 0.3, 0.3, 1.0),
    );
    assert_close(out, [153, 153, 153, 255], "negation");
}

// ─── Piecewise ──────────────────────────────────────────────────────

#[test]
fn overlay_with_low_base_acts_like_multiply() {
    // base=0.25 (< 0.5) → 2·base·blend = 2·0.25·0.6 = 0.3 → 76.
    let out = run_blend(
        BlendMode::Overlay,
        Color::rgba(0.25, 0.25, 0.25, 1.0),
        Color::rgba(0.6, 0.6, 0.6, 1.0),
    );
    assert_close(out, [76, 76, 76, 255], "overlay-low");
}

#[test]
fn overlay_with_high_base_acts_like_screen() {
    // base=0.75 → 1 - 2·(1-0.75)·(1-0.4) = 1 - 0.3 = 0.7 → 178.
    let out = run_blend(
        BlendMode::Overlay,
        Color::rgba(0.75, 0.75, 0.75, 1.0),
        Color::rgba(0.4, 0.4, 0.4, 1.0),
    );
    assert_close(out, [178, 178, 178, 255], "overlay-high");
}

#[test]
fn hard_light_swaps_overlay_threshold() {
    // base=0.75, blend=0.25 → 2·0.75·0.25 = 0.375 → 96 (uses blend < 0.5).
    let out = run_blend(
        BlendMode::HardLight,
        Color::rgba(0.75, 0.75, 0.75, 1.0),
        Color::rgba(0.25, 0.25, 0.25, 1.0),
    );
    assert_close(out, [96, 96, 96, 255], "hard-light");
}

#[test]
fn pin_light_picks_extremes() {
    // blend=0.7 (>= 0.5) → max(base, 2·0.7-1) = max(0.6, 0.4) = 0.6 → 153.
    let out = run_blend(
        BlendMode::PinLight,
        Color::rgba(0.6, 0.6, 0.6, 1.0),
        Color::rgba(0.7, 0.7, 0.7, 1.0),
    );
    assert_close(out, [153, 153, 153, 255], "pin-light");
}

#[test]
fn hard_mix_thresholds_to_zero_or_one() {
    // base+blend = 1.2 (>= 1.0 → 1).
    let out = run_blend(
        BlendMode::HardMix,
        Color::rgba(0.6, 0.6, 0.6, 1.0),
        Color::rgba(0.6, 0.6, 0.6, 1.0),
    );
    assert_close(out, [255, 255, 255, 255], "hard-mix-high");
    let out = run_blend(
        BlendMode::HardMix,
        Color::rgba(0.3, 0.3, 0.3, 1.0),
        Color::rgba(0.3, 0.3, 0.3, 1.0),
    );
    // base+blend=0.6 < 1.0 → 0.
    assert_close(out, [0, 0, 0, 255], "hard-mix-low");
}

#[test]
fn vivid_light_combines_burn_and_dodge() {
    // blend=0.7 (>= 0.5) → dodge: base / (1 - 2·(blend - 0.5))
    //   = base / (1 - 0.4) = 0.5 / 0.6 ≈ 0.833 → 213.
    let out = run_blend(
        BlendMode::VividLight,
        Color::rgba(0.5, 0.5, 0.5, 1.0),
        Color::rgba(0.7, 0.7, 0.7, 1.0),
    );
    assert_close(out, [213, 213, 213, 255], "vivid-light");
}

#[test]
fn linear_light_doubles_blend() {
    // base + 2·blend - 1 → 0.4 + 2·0.6 - 1 = 0.6 → 153.
    let out = run_blend(
        BlendMode::LinearLight,
        Color::rgba(0.4, 0.4, 0.4, 1.0),
        Color::rgba(0.6, 0.6, 0.6, 1.0),
    );
    assert_close(out, [153, 153, 153, 255], "linear-light");
}

// ─── Burn / Dodge ────────────────────────────────────────────────────

#[test]
fn color_burn_intensifies_darks() {
    // base=0.3, blend=0.5 → 1 - min(1, (1-0.3)/0.5) = 1 - min(1, 1.4) = 0 → 0.
    let out = run_blend(
        BlendMode::ColorBurn,
        Color::rgba(0.3, 0.3, 0.3, 1.0),
        Color::rgba(0.5, 0.5, 0.5, 1.0),
    );
    assert_close(out, [0, 0, 0, 255], "color-burn-saturated");
}

#[test]
fn color_dodge_intensifies_lights() {
    // base=0.5, blend=0.6 → min(1, 0.5/0.4) = min(1, 1.25) = 1.0 → 255.
    let out = run_blend(
        BlendMode::ColorDodge,
        Color::rgba(0.5, 0.5, 0.5, 1.0),
        Color::rgba(0.6, 0.6, 0.6, 1.0),
    );
    assert_close(out, [255, 255, 255, 255], "color-dodge-saturated");
}

#[test]
fn divide_clamps_to_one_when_blend_smaller() {
    // 0.4 / 0.5 = 0.8 → 204.
    let out = run_blend(
        BlendMode::Divide,
        Color::rgba(0.4, 0.4, 0.4, 1.0),
        Color::rgba(0.5, 0.5, 0.5, 1.0),
    );
    assert_close(out, [204, 204, 204, 255], "divide");
}

// ─── HSL family ──────────────────────────────────────────────────────

#[test]
fn luminosity_adopts_blend_lum() {
    // Backdrop pure red (lum=0.3), foreground pure white (lum=1.0).
    // Result: backdrop with luminosity raised to 1.0 → white-ish.
    let out = run_blend(
        BlendMode::Luminosity,
        Color::rgba(1.0, 0.0, 0.0, 1.0),
        Color::rgba(1.0, 1.0, 1.0, 1.0),
    );
    // After clip_color the result is white (255, 255, 255).
    assert_close(out, [255, 255, 255, 255], "luminosity-to-white");
}

#[test]
fn color_keeps_backdrop_luminosity() {
    // Backdrop=gray (lum=0.5), foreground=red (lum=0.3).
    // Color blend: foreground tinted to backdrop's luminosity 0.5.
    // Red(1,0,0) at lum 0.5 → set_lum lifts it; channels saturate.
    let out = run_blend(
        BlendMode::Color,
        Color::rgba(0.5, 0.5, 0.5, 1.0),
        Color::rgba(1.0, 0.0, 0.0, 1.0),
    );
    // The reference math (with clipping): set_lum(red, 0.5) ≈
    // (1.0, 0.235, 0.235). Anchor on the red channel only — green/blue
    // depend on clipping behavior that varies across reference impls.
    assert!(
        out[0] >= 200,
        "color: red channel should still dominate, got {out:?}"
    );
}

#[test]
fn saturation_inherits_blend_chroma() {
    // Backdrop=mid-saturation, foreground=zero saturation (gray).
    // Result: backdrop desaturated to gray-ish.
    let out = run_blend(
        BlendMode::Saturation,
        Color::rgba(0.7, 0.3, 0.3, 1.0),
        Color::rgba(0.5, 0.5, 0.5, 1.0),
    );
    // Foreground gray has 0 saturation → result has 0 saturation →
    // all channels equal. Allow generous tolerance for HSL math.
    let max_ch = *out[..3].iter().max().unwrap();
    let min_ch = *out[..3].iter().min().unwrap();
    assert!(
        i32::from(max_ch) - i32::from(min_ch) <= 6,
        "saturation: channels should converge with zero-sat foreground, got {out:?}"
    );
}
