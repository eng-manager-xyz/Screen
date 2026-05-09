//! Headless render-to-PNG. M0.21 proof point: build a scene, render to a
//! `RenderTexture`, read pixels, save as PNG. No window, no winit.
//!
//! Run with: `cargo run -p wisp --example headless_export`. Output goes to
//! `target/headless_export.png` next to the example.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Font, Graphics, RenderTexture, Sprite, Stage, Stroke, Text, Texture};

const W: u32 = 800;
const H: u32 = 450;

fn main() -> wisp::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("wisp=info")),
        )
        .init();

    let app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))?;

    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let rt = RenderTexture::with_format(&app, W, H, format);
    let renderer = Renderer::new(&app, format).expect("renderer");

    // Build a scene that exercises sprites + graphics + text.
    let mut stage = Stage::new();
    let root = stage.root();

    // Background gradient panel.
    let mut bg = Graphics::new();
    bg.fill(Fill::LinearGradient {
        start: Vec2::new(0.0, 1.0),
        end: Vec2::new(0.0, -1.0),
        color_a: Color::rgba_u8(40, 50, 70, 255),
        color_b: Color::rgba_u8(20, 25, 35, 255),
    });
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(root, bg);

    // A "recording quad" placeholder.
    let mut recording = Graphics::new();
    recording.fill(Fill::Solid(Color::rgba_u8(180, 200, 230, 255)));
    recording.stroke(Some(Stroke::new(0.012, Color::rgba_u8(60, 80, 110, 255))));
    recording.draw_rounded_rect(Rect::new(-0.6, -0.4, 1.2, 0.8), 0.06);
    let _ = stage.add_child(root, recording);

    // A sprite (the "cursor" stand-in).
    let mut cursor_bytes = Vec::with_capacity(8 * 8 * 4);
    for _ in 0..(8 * 8) {
        cursor_bytes.extend_from_slice(&[255, 255, 255, 255]);
    }
    let cursor_tex = Texture::from_rgba(&app, 8, 8, &cursor_bytes);
    let mut cursor = Sprite::from_texture(cursor_tex).with_anchor(Vec2::splat(0.5));
    cursor.container.transform.position = Vec2::new(0.2, 0.1);
    cursor.container.transform.scale = Vec2::splat(0.04);
    let _ = stage.add_child(root, cursor);

    // Text label.
    let font = Font::bitmap_8x8(&app);
    let mut label = Text::new(font, "wisp headless export").with_cell_size(0.024);
    label.color = Color::rgba_u8(255, 240, 200, 255);
    label.container.transform.position = Vec2::new(-0.55, 0.5);
    let _ = stage.add_child(root, label);

    let stats = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    println!(
        "stats: draw_calls={}, sprites={}, graphics={}, glyphs={}, meshes={}",
        stats.draw_calls,
        stats.sprites_drawn,
        stats.graphics_drawn,
        stats.glyphs_drawn,
        stats.meshes_drawn
    );

    // Read back and save as PNG.
    let bytes = rt.read_pixels(&app);
    let buffer = image::RgbaImage::from_raw(W, H, bytes).expect("buffer dims match");
    let path = std::path::Path::new("target/headless_export.png");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    buffer.save(path).expect("save png");
    println!("wrote {}", path.display());

    Ok(())
}
