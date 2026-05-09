//! M0.15 — bitmap text rendering integration tests.

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, Font, RenderTexture, Stage, Text};

fn boot() -> Application {
    block_on(Application::new(AppConfig::default())).expect("init wisp")
}

#[test]
fn empty_text_emits_no_glyphs() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, 32, 32, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let font = Font::bitmap_8x8(&app);
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), Text::new(font, ""));

    let stats = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    assert_eq!(stats.glyphs_drawn, 0);
}

#[test]
fn glyphs_count_matches_string_length() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, 64, 32, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let font = Font::bitmap_8x8(&app);
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), Text::new(font, "Hi!"));

    let stats = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    assert_eq!(stats.glyphs_drawn, 3);
    assert_eq!(stats.draw_calls, 1);
}

#[test]
fn shared_font_batches_into_one_draw_call() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, 128, 64, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let font = Font::bitmap_8x8(&app);
    let mut stage = Stage::new();
    let root = stage.root();
    let _ = stage.add_child(root, Text::new(font.clone(), "Hello"));
    let _ = stage.add_child(root, Text::new(font.clone(), "World"));

    let stats = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    // 2 Text nodes × 5 chars each = 10 glyphs.
    assert_eq!(stats.glyphs_drawn, 10);
    // Same atlas → one batch.
    assert_eq!(stats.draw_calls, 1);
}

#[test]
fn text_renders_visible_pixels() {
    // Render a single big 'X' near the origin and verify SOME pixels in its
    // bounding region are non-zero (text was actually drawn).
    let app = boot();
    let rt = RenderTexture::with_format(&app, 64, 64, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let font = Font::bitmap_8x8(&app);
    let mut text = Text::new(font, "X").with_cell_size(0.06);
    text.color = Color::rgba_u8(255, 0, 0, 255);
    text.container.transform.position = glam::Vec2::new(0.0, 0.2);

    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), text);

    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);

    let bytes = rt.read_pixels(&app);
    // Sum red channel across the full image — any non-trivial number means glyph rendered.
    let red_sum: u32 = bytes.chunks(4).map(|p| u32::from(p[0])).sum();
    assert!(
        red_sum > 200,
        "expected the X to deposit substantial red; got total R={red_sum}"
    );
}
