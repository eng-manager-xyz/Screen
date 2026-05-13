//! Mock recorder scene — the full compositor stack rendered headless.
//!
//! Layers (back → front):
//!   1. Gradient background panel
//!   2. Recording quad (a checker placeholder for the screen capture)
//!   3. Camera bubble (rounded sprite in the corner)
//!   4. Cursor sprite + click ripple
//!   5. Keyboard chip (rounded rect + text)
//!   6. Caption text under the recording
//!
//! Run: `cargo run -p screen-wisp --example recorder_mock`. Output: `target/recorder_mock.png`.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{
    Color, DropShadowFilter, Fill, Font, Graphics, RenderTexture, Sprite, Stage, Stroke, Text,
    Texture,
};

const W: u32 = 1280;
const H: u32 = 720;

#[allow(
    clippy::too_many_lines,
    reason = "this is a demo example — composing the recorder scene reads better as one function than as helpers"
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
    let renderer = Renderer::new(&app, format).expect("renderer");

    // Pre-bake the recording quad with a drop shadow into a texture, then sprite it in.
    let recording_texture = checker_texture(&app, 256, [180, 200, 230, 255], [60, 90, 120, 255]);
    let pre_input = RenderTexture::with_format(&app, 1024, 1024, format);
    let pre_output = RenderTexture::with_format(&app, 1024, 1024, format);
    let mut pre_stage = Stage::new();
    let mut pre_sprite = Sprite::from_texture(recording_texture).with_anchor(Vec2::splat(0.5));
    pre_sprite.container.transform.scale = Vec2::splat(0.7);
    let _ = pre_stage.add_child(pre_stage.root(), pre_sprite);
    let _ = renderer.render_stage(&app, pre_input.view(), Color::TRANSPARENT, &pre_stage);
    renderer.apply_filter(
        &app,
        &DropShadowFilter {
            offset: Vec2::new(8.0, 12.0),
            blur: 8.0,
            color: Color::rgba(0.0, 0.0, 0.0, 0.6),
        },
        &pre_input,
        &pre_output,
    );
    let recording_with_shadow_bytes = pre_output.read_pixels(&app);
    let recording_with_shadow = Texture::from_rgba(&app, 1024, 1024, &recording_with_shadow_bytes);

    // Now build the final scene.
    let rt = RenderTexture::with_format(&app, W, H, format);
    let mut stage = Stage::new();
    let root = stage.root();

    // Background gradient panel.
    let mut bg = Graphics::new();
    bg.fill(Fill::LinearGradient {
        start: Vec2::new(0.0, 1.0),
        end: Vec2::new(0.0, -1.0),
        color_a: Color::rgba_u8(50, 70, 110, 255),
        color_b: Color::rgba_u8(15, 20, 30, 255),
    });
    bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = stage.add_child(root, bg);

    // Recording (with baked drop shadow).
    let mut recording_sprite =
        Sprite::from_texture(recording_with_shadow).with_anchor(Vec2::splat(0.5));
    recording_sprite.container.transform.scale = Vec2::splat(0.85);
    let _ = stage.add_child(root, recording_sprite);

    // Camera bubble in the bottom-right.
    let camera_texture = checker_texture(&app, 64, [255, 220, 200, 255], [200, 140, 130, 255]);
    let mut camera = Sprite::from_texture(camera_texture).with_anchor(Vec2::splat(0.5));
    camera.container.transform.position = Vec2::new(0.7, -0.55);
    camera.container.transform.scale = Vec2::splat(0.18);
    let _ = stage.add_child(root, camera);

    let mut camera_outline = Graphics::new();
    camera_outline.stroke(Some(Stroke::new(0.012, Color::rgba_u8(255, 240, 220, 230))));
    camera_outline.fill(Fill::Solid(Color::TRANSPARENT));
    camera_outline.draw_ellipse(Vec2::new(0.7, -0.55), Vec2::splat(0.13));
    let _ = stage.add_child(root, camera_outline);

    // Cursor + click ripple near the center of the recording.
    let cursor_pos = Vec2::new(-0.1, 0.05);
    let mut cursor_bytes = Vec::with_capacity(8 * 8 * 4);
    for _ in 0..(8 * 8) {
        cursor_bytes.extend_from_slice(&[255, 255, 255, 255]);
    }
    let cursor_tex = Texture::from_rgba(&app, 8, 8, &cursor_bytes);
    let mut cursor = Sprite::from_texture(cursor_tex).with_anchor(Vec2::splat(0.5));
    cursor.container.transform.position = cursor_pos;
    cursor.container.transform.scale = Vec2::splat(0.025);
    let _ = stage.add_child(root, cursor);

    let mut ripple = Graphics::new();
    ripple.stroke(Some(Stroke::new(0.008, Color::rgba(1.0, 1.0, 1.0, 0.7))));
    ripple.fill(Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 0.18)));
    ripple.draw_ellipse(cursor_pos, Vec2::splat(0.06));
    let _ = stage.add_child(root, ripple);

    // Keyboard shortcut chip.
    let mut chip_bg = Graphics::new();
    chip_bg.fill(Fill::Solid(Color::rgba(0.0, 0.0, 0.0, 0.78)));
    chip_bg.stroke(Some(Stroke::new(0.005, Color::rgba_u8(120, 140, 180, 220))));
    chip_bg.draw_rounded_rect(Rect::new(-0.85, -0.85, 0.45, 0.12), 0.04);
    let _ = stage.add_child(root, chip_bg);

    let font = Font::bitmap_8x8(&app);
    let mut chip_label = Text::new(font.clone(), "Cmd + Shift + .").with_cell_size(0.014);
    chip_label.color = Color::rgba_u8(220, 230, 250, 255);
    chip_label.container.transform.position = Vec2::new(-0.79, -0.78);
    let _ = stage.add_child(root, chip_label);

    // Caption.
    let mut caption = Text::new(font, "auto-zoom triggered on click").with_cell_size(0.018);
    caption.color = Color::rgba_u8(255, 240, 220, 255);
    caption.container.transform.position = Vec2::new(-0.4, 0.95);
    let _ = stage.add_child(root, caption);

    let stats = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    println!(
        "stats: draw_calls={}, sprites={}, graphics={}, glyphs={}, meshes={}",
        stats.draw_calls,
        stats.sprites_drawn,
        stats.graphics_drawn,
        stats.glyphs_drawn,
        stats.meshes_drawn
    );

    let bytes = rt.read_pixels(&app);
    let buffer = image::RgbaImage::from_raw(W, H, bytes).expect("buffer dims match");
    let path = std::path::Path::new("target/recorder_mock.png");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    buffer.save(path).expect("save png");
    println!("wrote {}", path.display());

    Ok(())
}

fn checker_texture(app: &Application, size: u32, light: [u8; 4], dark: [u8; 4]) -> Texture {
    let mut bytes = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let cell_x = (x / 8) % 2;
            let cell_y = (y / 8) % 2;
            let pick = if (cell_x ^ cell_y) == 1 { dark } else { light };
            bytes.extend_from_slice(&pick);
        }
    }
    Texture::from_rgba(app, size, size, &bytes)
}
