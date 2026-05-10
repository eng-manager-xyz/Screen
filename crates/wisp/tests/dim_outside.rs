//! AUT-29 — dim-outside inverse mask. Validates the renderer-data
//! API (`DimOutside` + `DimStrength`):
//!
//! 1. `DimStrength` presets are monotonic: Light < Medium < Heavy.
//! 2. `Custom(f32)` clamps to `[0, 1]`.
//! 3. End-to-end: rendering with a *higher* dim strength produces a
//!    *darker* outside region. Sample a pixel outside the focus shape
//!    over an all-red base; R should drop monotonically as strength
//!    grows.

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, DimOutside, DimStrength, Fill, Graphics, RenderTexture, Stage};

const W: u32 = 128;
const H: u32 = 128;

fn boot() -> (Application, Renderer) {
    let app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("app");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("renderer");
    (app, renderer)
}

fn outside_red(rt: &RenderTexture, app: &Application) -> u8 {
    let bytes = rt.read_pixels(app);
    let stride = (W as usize) * 4;
    // Top-left pixel — far outside the focus region.
    let i = 4 * stride + 4 * 4;
    bytes[i]
}

fn render_red_base(app: &Application, renderer: &Renderer) -> RenderTexture {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let base = RenderTexture::with_format(app, W, H, format);
    let mut stage = Stage::new();
    let mut bg = Graphics::new();
    bg.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), bg);
    let _ = renderer.render_stage(app, base.view(), Color::BLACK, &stage);
    base
}

#[test]
fn dim_presets_are_monotonic() {
    assert!(DimStrength::Light.alpha() < DimStrength::Medium.alpha());
    assert!(DimStrength::Medium.alpha() < DimStrength::Heavy.alpha());
}

#[test]
fn dim_custom_is_clamped() {
    assert!(DimStrength::Custom(-1.0).alpha().abs() < 1e-6);
    assert!((DimStrength::Custom(5.0).alpha() - 1.0).abs() < 1e-6);
    assert!((DimStrength::Custom(0.42).alpha() - 0.42).abs() < 1e-6);
}

#[test]
fn higher_dim_makes_outside_darker() {
    let (app, renderer) = boot();
    let base = render_red_base(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let region = Rect::new(-0.4, -0.4, 0.8, 0.8);

    let make = |strength: DimStrength| -> u8 {
        let rt = RenderTexture::with_format(&app, W, H, format);
        let dim = DimOutside::rect(region).with_strength(strength);
        renderer.apply_dim_outside_data(&app, &dim, &base, &rt);
        outside_red(&rt, &app)
    };

    let light = make(DimStrength::Light);
    let medium = make(DimStrength::Medium);
    let heavy = make(DimStrength::Heavy);

    assert!(
        light > medium,
        "Medium should darken more than Light; light={light}, medium={medium}"
    );
    assert!(
        medium > heavy,
        "Heavy should darken more than Medium; medium={medium}, heavy={heavy}"
    );
}
