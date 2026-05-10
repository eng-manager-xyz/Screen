//! Integration tests for M-TEXT.6 / AUT-80 — text participates in
//! mask, filter, blend, and export composition.
//!
//! Builds on M-TEXT.5: `TextTexturePipeline::render` returns a
//! `RenderTexture` that can be:
//!   - wrapped as a sprite via `RenderTexture::as_texture()` and
//!     drawn through `Renderer::render_stage` (the headless-export
//!     code path);
//!   - given a non-Normal `BlendMode` on its container so it
//!     composites differently than a sibling sprite;
//!   - fed straight into `Renderer::apply_filter` because it already
//!     is a `RenderTexture`.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::blend::BlendMode;
use wisp::color::Color;
use wisp::filter::ColorMatrixFilter;
use wisp::render::Renderer;
use wisp::text::{TextTexturePipeline, WispText, WispTextStyle};
use wisp::texture::render_texture::RenderTexture;
use wisp::{Sprite, Stage, WispFontWeight};

const SIZE: u32 = 256;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn boot() -> Application {
    block_on(Application::new(AppConfig::default())).expect("init")
}

fn text_sprite(pipeline: &TextTexturePipeline, app: &Application, blend: BlendMode) -> Sprite {
    let text = WispText::new("ABC").with_style(
        WispTextStyle::default()
            .with_size(0.5)
            .with_weight(WispFontWeight::Bold)
            .with_color(Color::WHITE),
    );
    let rt = pipeline.render(app, &text, 512, 256);
    let mut sprite = Sprite::from_texture(rt.as_texture()).with_anchor(Vec2::new(0.5, 0.5));
    sprite.container.transform.scale = Vec2::new(1.6, -0.8); // -y to flip glyphon -> sprite UV
    sprite.container.blend_mode = blend;
    sprite
}

fn render_to_pixels(stage: &Stage, app: &Application, clear: Color) -> Vec<u8> {
    let renderer = Renderer::new(app, FORMAT).expect("Renderer::new");
    let target = RenderTexture::with_format(app, SIZE, SIZE, FORMAT);
    let _ = renderer.render_stage(app, target.view(), clear, stage);
    target.read_pixels(app)
}

#[test]
fn text_renders_through_render_stage() {
    let app = boot();
    let pipeline = TextTexturePipeline::new(&app, FORMAT);

    let mut stage = Stage::new();
    let sprite = text_sprite(&pipeline, &app, BlendMode::Normal);
    let _ = stage.add_child(stage.root(), sprite);

    let pixels = render_to_pixels(&stage, &app, Color::BLACK);
    // Black clear + white glyph → expect ANY pixel above mid-gray
    // means render_stage actually drew the text. (Sprite covers the
    // canvas; anti-aliased glyph edges contribute partial alpha.)
    let bright = pixels
        .chunks_exact(4)
        .filter(|p| p[0] > 128 || p[1] > 128 || p[2] > 128)
        .count();
    assert!(
        bright > 100,
        "expected render_stage to paint glyph pixels (got {bright} bright pixels)"
    );
}

#[test]
fn text_with_multiply_blend_differs_from_normal() {
    let app = boot();
    let pipeline = TextTexturePipeline::new(&app, FORMAT);

    // Stage A: text sprite at Normal over a red backdrop.
    let mut stage_normal = Stage::new();
    let mut backdrop_a = Sprite::from_texture(
        // Make a small solid-red texture via TextTexturePipeline rendering
        // an empty layout: we just need a colored quad. Easier — use
        // a textured quad from a generated render texture.
        {
            let rt = RenderTexture::with_format(&app, 8, 8, FORMAT);
            let renderer = Renderer::new(&app, FORMAT).expect("Renderer::new");
            let empty = Stage::new();
            let _ = renderer.render_stage(&app, rt.view(), Color::rgba(0.8, 0.2, 0.2, 1.0), &empty);
            rt.as_texture()
        },
    )
    .with_anchor(Vec2::new(0.5, 0.5));
    backdrop_a.container.transform.scale = Vec2::new(2.0, 2.0); // fills canvas
    let _ = stage_normal.add_child(stage_normal.root(), backdrop_a.clone());
    let _ = stage_normal.add_child(
        stage_normal.root(),
        text_sprite(&pipeline, &app, BlendMode::Normal),
    );
    let pixels_normal = render_to_pixels(&stage_normal, &app, Color::BLACK);

    // Stage B: text sprite at Multiply over the same backdrop.
    let mut stage_mul = Stage::new();
    let _ = stage_mul.add_child(stage_mul.root(), backdrop_a);
    let _ = stage_mul.add_child(
        stage_mul.root(),
        text_sprite(&pipeline, &app, BlendMode::Multiply),
    );
    let pixels_mul = render_to_pixels(&stage_mul, &app, Color::BLACK);

    // Multiply against a red backdrop should kill the green + blue
    // channels of the white glyph; Normal should keep them. So some
    // pixel must differ.
    let total_diff: u32 = pixels_normal
        .iter()
        .zip(pixels_mul.iter())
        .map(|(a, b)| u32::from(a.abs_diff(*b)))
        .sum();
    assert!(
        total_diff > 1000,
        "expected Multiply blend to produce a different image than Normal (total diff {total_diff})"
    );
}

#[test]
fn text_filtered_through_color_matrix_grayscale_produces_gray_pixels() {
    let app = boot();
    let pipeline = TextTexturePipeline::new(&app, FORMAT);

    // Render colored text into an RT.
    let text = WispText::new("Hue").with_style(
        WispTextStyle::default()
            .with_size(0.5)
            .with_weight(WispFontWeight::Bold)
            .with_color(Color::rgba(1.0, 0.4, 0.1, 1.0)), // saturated orange
    );
    let input = pipeline.render(&app, &text, 512, 256);

    // Apply grayscale color matrix.
    let renderer = Renderer::new(&app, FORMAT).expect("Renderer::new");
    let output = RenderTexture::with_format(&app, 512, 256, FORMAT);
    renderer.apply_filter(&app, &ColorMatrixFilter::grayscale(), &input, &output);

    let bytes = output.read_pixels(&app);

    // For grayscale, R == G == B (within a small tolerance for
    // rounding through the f32 → u8 round-trip in the shader).
    // Pick the brightest 50 pixels (the glyph body) and verify they
    // satisfy that.
    let mut by_brightness: Vec<&[u8]> = bytes.chunks_exact(4).filter(|p| p[3] > 0).collect();
    by_brightness
        .sort_by_key(|p| std::cmp::Reverse(u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])));
    assert!(
        by_brightness.len() >= 50,
        "expected ≥50 opaque pixels in the filtered RT, got {}",
        by_brightness.len()
    );
    for p in by_brightness.iter().take(50) {
        let r = i32::from(p[0]);
        let g = i32::from(p[1]);
        let b = i32::from(p[2]);
        assert!(
            (r - g).abs() <= 2 && (g - b).abs() <= 2,
            "grayscale should yield R≈G≈B; got ({r}, {g}, {b})"
        );
    }
}

#[test]
fn text_present_in_headless_export_pixels() {
    // "Headless export" in wisp is render_stage → RenderTexture →
    // read_pixels. Same path the storybook exporter uses. Confirm
    // text shows up in the byte buffer.
    let app = boot();
    let pipeline = TextTexturePipeline::new(&app, FORMAT);

    let mut stage = Stage::new();
    let _ = stage.add_child(
        stage.root(),
        text_sprite(&pipeline, &app, BlendMode::Normal),
    );

    let pixels = render_to_pixels(&stage, &app, Color::rgba(0.05, 0.05, 0.1, 1.0));

    // Background is near-black-blue; glyphs are white. Count pixels
    // brighter than the background by >50 in any channel.
    let bright = pixels
        .chunks_exact(4)
        .filter(|p| p[0] > 60 || p[1] > 60 || p[2] > 60)
        .count();
    assert!(
        bright > 100,
        "expected headless export to capture the text texture (got {bright})"
    );
}
