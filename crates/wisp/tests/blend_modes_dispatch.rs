//! Auto-dispatch tests for advanced blend modes (M-BLEND.2).
//!
//! Verifies that setting `container.blend_mode` to an advanced mode and
//! calling `Renderer::render_stage` produces the same pixel output as
//! manually invoking `apply_advanced_blend` with separately-rendered
//! backdrop and foreground RTs.
//!
//! Plus a regression guard: a native-only stage produces identical
//! output to the pre-M-BLEND.2 fast path (Normal/Multiply/etc still work
//! without offscreen indirection).

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{BlendMode, Color, Fill, Graphics, RenderTexture, Stage};

const W: u32 = 64;
const H: u32 = 64;

/// Fixed app dims so the slow path's internal RT sizes match the
/// caller's view.
fn boot() -> (Application, Renderer, RenderTexture) {
    let app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("app");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("renderer");
    let target = RenderTexture::with_format(&app, W, H, format);
    (app, renderer, target)
}

fn center_pixel(rt: &RenderTexture, app: &Application) -> [u8; 4] {
    let bytes = rt.read_pixels(app);
    let cx = (W / 2) as usize;
    let cy = (H / 2) as usize;
    let stride = (W as usize) * 4;
    let i = cy * stride + cx * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

fn assert_close(a: [u8; 4], b: [u8; 4], tol: i32, ctx: &str) {
    for ch in 0..4 {
        let diff = i32::from(a[ch]) - i32::from(b[ch]);
        assert!(diff.abs() <= tol, "{ctx} ch{ch}: {a:?} vs {b:?}");
    }
}

#[test]
fn auto_dispatch_matches_manual_apply_for_overlay() {
    // Setup: backdrop = horizontal red→blue gradient; foreground =
    // vertical yellow→cyan gradient. Compare auto-dispatch vs manual
    // apply_advanced_blend for Overlay mode.
    let (app, renderer, target) = boot();

    // Manual reference: render bg + fg into separate RTs, apply_advanced_blend.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let bg_rt = RenderTexture::with_format(&app, W, H, format);
    let fg_rt = RenderTexture::with_format(&app, W, H, format);
    let manual_rt = RenderTexture::with_format(&app, W, H, format);
    {
        let mut s = Stage::new();
        let mut g = Graphics::new();
        g.fill(Fill::Solid(Color::rgba(0.6, 0.2, 0.3, 1.0)));
        g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
        let _ = s.add_child(s.root(), g);
        let _ = renderer.render_stage(&app, bg_rt.view(), Color::BLACK, &s);
    }
    {
        let mut s = Stage::new();
        let mut g = Graphics::new();
        g.fill(Fill::Solid(Color::rgba(0.4, 0.7, 0.6, 1.0)));
        g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
        let _ = s.add_child(s.root(), g);
        let _ = renderer.render_stage(&app, fg_rt.view(), Color::rgba(0.0, 0.0, 0.0, 0.0), &s);
    }
    renderer.apply_advanced_blend(&app, BlendMode::Overlay, &bg_rt, &fg_rt, &manual_rt);
    let manual = center_pixel(&manual_rt, &app);

    // Auto-dispatch: same scene composed via the renderer with
    // container.blend_mode set on the foreground node.
    let mut stage = Stage::new();
    let mut bg = Graphics::new();
    bg.fill(Fill::Solid(Color::rgba(0.6, 0.2, 0.3, 1.0)));
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), bg);

    let mut fg = Graphics::new();
    fg.fill(Fill::Solid(Color::rgba(0.4, 0.7, 0.6, 1.0)));
    fg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    fg.container.blend_mode = BlendMode::Overlay;
    let _ = stage.add_child(stage.root(), fg);

    let _ = renderer.render_stage(&app, target.view(), Color::BLACK, &stage);
    let auto = center_pixel(&target, &app);

    assert_close(auto, manual, 3, "auto-dispatch vs manual overlay");
}

#[test]
fn auto_dispatch_handles_difference_with_solid_underlay() {
    let (app, renderer, target) = boot();

    let mut stage = Stage::new();
    let mut bg = Graphics::new();
    bg.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), bg);

    let mut fg = Graphics::new();
    fg.fill(Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)));
    fg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    fg.container.blend_mode = BlendMode::Difference;
    let _ = stage.add_child(stage.root(), fg);

    let _ = renderer.render_stage(&app, target.view(), Color::BLACK, &stage);
    let pixel = center_pixel(&target, &app);

    // |red - blue| = (1, 0, 1) → magenta.
    assert_close(pixel, [255, 0, 255, 255], 3, "difference auto-dispatch");
}

#[test]
fn native_only_stage_unchanged_by_dispatch_path() {
    // Multiply is GPU-native (Tier A) — auto-dispatch should leave the
    // fast path active, producing the same result as before M-BLEND.2.
    let (app, renderer, target) = boot();

    let mut stage = Stage::new();
    let mut bg = Graphics::new();
    bg.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(stage.root(), bg);

    let mut fg = Graphics::new();
    fg.fill(Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)));
    fg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    fg.container.blend_mode = BlendMode::Multiply;
    let _ = stage.add_child(stage.root(), fg);

    let _ = renderer.render_stage(&app, target.view(), Color::BLACK, &stage);
    let pixel = center_pixel(&target, &app);
    // red · blue = (0, 0, 0).
    assert_close(pixel, [0, 0, 0, 255], 2, "multiply on native fast path");
}

#[test]
fn empty_stage_with_advanced_blends_still_clears_to_color() {
    // No nodes — should just clear to red. No advanced dispatch needed.
    let (app, renderer, target) = boot();
    let stage = Stage::new();
    let _ = renderer.render_stage(&app, target.view(), Color::rgba(1.0, 0.0, 0.0, 1.0), &stage);
    let pixel = center_pixel(&target, &app);
    assert_close(pixel, [255, 0, 0, 255], 2, "clear-only");
}
