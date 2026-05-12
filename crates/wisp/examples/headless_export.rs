//! Headless render-to-PNG over **60 frames at 1080p** — the M0.21 proof
//! point that wisp can drive a future export pipeline (encode crate
//! consumes these PNGs, or feeds the same scene to a
//! [`GStreamer`](https://gstreamer.freedesktop.org/) `appsrc →
//! vtenc_h264_hw → mp4mux → filesink` pipeline directly).
//!
//! Builds the recorder-mock-shaped scene (background gradient, recording
//! quad, cursor sprite, text label) and animates it for 60 frames:
//!
//! - Recording quad rotates ±8° (1 cycle over 60 frames).
//! - Cursor oscillates horizontally.
//! - Text label pulses scale 1.0 → 1.15 → 1.0.
//!
//! Run: `cargo run -p screen-wisp --example headless_export`. Output: 60 PNGs
//! at `target/headless_export/frame_NN.png`. Also writes a single
//! representative frame (#30, the mid-animation peak) to
//! `_docs/book/src/assets/wisp/example-headless-export.png` so the
//! mdBook chapter can embed it without committing 60 large PNGs.

use std::path::Path;

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Font, Graphics, RenderTexture, Sprite, Stage, Stroke, Text, Texture};

const W: u32 = 1920;
const H: u32 = 1080;
const FRAMES: u32 = 60;
const HIGHLIGHT_FRAME: u32 = 30;

#[allow(
    clippy::too_many_lines,
    reason = "demo example; the per-frame scene assembly reads cleaner inline"
)]
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

    // The cursor-stand-in texture is allocated once and shared across all
    // 60 frames — `Texture` is Arc-backed so cloning into each frame's
    // Sprite is cheap.
    let mut cursor_bytes = Vec::with_capacity(8 * 8 * 4);
    for _ in 0..(8 * 8) {
        cursor_bytes.extend_from_slice(&[255, 255, 255, 255]);
    }
    let cursor_tex = Texture::from_rgba(&app, 8, 8, &cursor_bytes);
    let font = Font::bitmap_8x8(&app);

    let out_dir = Path::new("target/headless_export");
    std::fs::create_dir_all(out_dir).ok();
    let highlight_dir = Path::new("_docs/book/src/assets/wisp");
    std::fs::create_dir_all(highlight_dir).ok();

    let mut total_draw_calls = 0u32;

    for frame in 0..FRAMES {
        #[allow(clippy::cast_precision_loss, reason = "frame index well below 2^23")]
        let t = frame as f32 / FRAMES as f32; // 0.0 .. 1.0

        // Per-frame animation parameters.
        let recording_angle = (t * std::f32::consts::TAU).sin() * 0.14; // ±~8°
        let cursor_x = (t * std::f32::consts::TAU).sin() * 0.5;
        let cursor_y = (t * std::f32::consts::TAU * 2.0).cos() * 0.15;
        let text_scale = 1.0 + (t * std::f32::consts::TAU).sin().abs() * 0.15;

        let mut stage = Stage::new();
        let root = stage.root();

        // Layer 1: gradient background.
        let mut bg = Graphics::new();
        bg.fill(Fill::LinearGradient {
            start: Vec2::new(0.0, 1.0),
            end: Vec2::new(0.0, -1.0),
            color_a: Color::rgba_u8(40, 50, 70, 255),
            color_b: Color::rgba_u8(20, 25, 35, 255),
        });
        bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
        let _ = stage.add_child(root, bg);

        // Layer 2: recording quad with stroke + per-frame rotation.
        let mut recording = Graphics::new();
        recording.fill(Fill::Solid(Color::rgba_u8(180, 200, 230, 255)));
        recording.stroke(Some(Stroke::new(0.012, Color::rgba_u8(60, 80, 110, 255))));
        recording.draw_rounded_rect(Rect::new(-0.6, -0.4, 1.2, 0.8), 0.06);
        recording.container.transform.rotation = recording_angle;
        let _ = stage.add_child(root, recording);

        // Layer 3: cursor sprite oscillating across the recording quad.
        let mut cursor = Sprite::from_texture(cursor_tex.clone()).with_anchor(Vec2::splat(0.5));
        cursor.container.transform.position = Vec2::new(cursor_x, cursor_y);
        cursor.container.transform.scale = Vec2::splat(0.04);
        let _ = stage.add_child(root, cursor);

        // Layer 4: text label pulsing.
        let mut label = Text::new(font.clone(), "wisp · 60-frame headless export · 1080p")
            .with_cell_size(0.024);
        label.color = Color::rgba_u8(255, 240, 200, 255);
        label.container.transform.position = Vec2::new(-0.78, 0.6);
        label.container.transform.scale = Vec2::splat(text_scale);
        let _ = stage.add_child(root, label);

        let stats = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
        total_draw_calls += stats.draw_calls;

        let bytes = rt.read_pixels(&app);
        let buffer = image::RgbaImage::from_raw(W, H, bytes).expect("buffer dims match");
        let path = out_dir.join(format!("frame_{frame:02}.png"));
        buffer.save(&path).expect("save frame png");

        // The mid-animation frame doubles as the chapter asset.
        if frame == HIGHLIGHT_FRAME {
            let highlight = highlight_dir.join("example-headless-export.png");
            buffer.save(&highlight).expect("save highlight png");
            println!("  ✓ highlight frame: {}", highlight.display());
        }
    }

    println!(
        "rendered {FRAMES} frames at {W}×{H}; total draw_calls={total_draw_calls}; \
         dir: {}",
        out_dir.display()
    );

    Ok(())
}
