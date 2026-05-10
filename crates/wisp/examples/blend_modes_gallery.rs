//! Contact-sheet generator for all 28 blend modes.
//!
//! For each [`BlendMode`], composes a known backdrop (red→blue gradient)
//! with a known foreground (yellow→cyan gradient) using that mode and
//! renders the result as a labelled tile. The 28 tiles are arranged in
//! a 7×4 grid.
//!
//! Output: `_docs/book/src/assets/wisp/blend-modes.png` — the chapter
//! at `_docs/book/src/wisp/chunks/blend-modes.md` embeds it.
//!
//! Run: `cargo run -p wisp --example blend_modes_gallery`.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{BlendMode, Color, Fill, Font, Graphics, RenderTexture, Sprite, Stage, Text, Texture};

const TILE_W: u32 = 200;
const TILE_H: u32 = 140;
const COLS: u32 = 7;
const ROWS: u32 = 4;

#[allow(
    clippy::too_many_lines,
    reason = "single-shot example; the per-tile compose loop reads cleaner inline than split helpers"
)]
fn main() -> wisp::Result<()> {
    let app = block_on(Application::new(AppConfig::default()))?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("renderer");

    let modes: Vec<BlendMode> = BlendMode::all().collect();
    assert_eq!(
        u32::try_from(modes.len()).unwrap(),
        COLS * ROWS,
        "tile grid must fit the BlendMode catalog"
    );

    let total_w = COLS * TILE_W;
    let total_h = ROWS * TILE_H;
    let sheet_rt = RenderTexture::with_format(&app, total_w, total_h, format);

    // Reusable per-tile RTs.
    let bg_rt = RenderTexture::with_format(&app, TILE_W, TILE_H, format);
    let fg_rt = RenderTexture::with_format(&app, TILE_W, TILE_H, format);
    let comp_rt = RenderTexture::with_format(&app, TILE_W, TILE_H, format);

    // Render the warm-cool gradient backdrop ONCE — used as the bg
    // for all 28 tiles.
    {
        let mut stage = Stage::new();
        let mut bg = Graphics::new();
        bg.fill(Fill::LinearGradient {
            start: Vec2::new(-1.0, 0.0),
            end: Vec2::new(1.0, 0.0),
            color_a: Color::rgba(0.85, 0.15, 0.25, 1.0),
            color_b: Color::rgba(0.15, 0.25, 0.85, 1.0),
        });
        bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
        let _ = stage.add_child(stage.root(), bg);
        let _ = renderer.render_stage(&app, bg_rt.view(), Color::BLACK, &stage);
    }

    // Load the foreground image (Apollo 17 Earth photo, public
    // domain) ONCE. Real-world chroma + luminance makes blend modes
    // far more legible than synthetic gradients — black space punches
    // through with multiply/min, oceans tint under hue/color, etc.
    let earth_image = image::open("crates/wisp-storybook/assets/images/blue-marble-320.jpg")
        .expect("blue-marble-320.jpg missing — run from workspace root or check the asset");
    let earth_texture = Texture::from_image(&app, &earth_image);
    // Render the Earth into fg_rt for the advanced-blend path.
    {
        let mut stage = Stage::new();
        let mut earth = Sprite::from_texture(earth_texture.clone()).with_anchor(Vec2::splat(0.5));
        earth.container.transform.scale = Vec2::splat(2.0); // fills the tile
        let _ = stage.add_child(stage.root(), earth);
        let _ = renderer.render_stage(&app, fg_rt.view(), Color::BLACK, &stage);
    }

    // Build the contact sheet by reading each composite back as bytes
    // and pasting into the right tile of a CPU-side image buffer.
    let stride = (total_w as usize) * 4;
    let mut sheet_bytes = vec![16u8; (total_w as usize) * (total_h as usize) * 4];
    for (i, mode) in modes.iter().enumerate() {
        let i_u32 = u32::try_from(i).unwrap();
        let col = i_u32 % COLS;
        let row = i_u32 / COLS;

        // Composite this mode into comp_rt.
        if let Some(blend_state) = mode.native_blend_state() {
            // Standard mode: build a stage with the pre-rendered bg
            // as a sprite, the Earth image as a sprite with the
            // current blend mode on top. (A Graphics backdrop would
            // paint AFTER sprites in render_stage's batched order, so
            // we use sprite-on-sprite.)
            let mut stage = Stage::new();
            let bg_tex = bg_rt.as_texture();
            let mut bg_sprite = Sprite::from_texture(bg_tex).with_anchor(Vec2::splat(0.5));
            bg_sprite.container.transform.scale = Vec2::splat(2.0);
            let _ = stage.add_child(stage.root(), bg_sprite);

            let mut earth =
                Sprite::from_texture(earth_texture.clone()).with_anchor(Vec2::splat(0.5));
            earth.container.transform.scale = Vec2::splat(2.0);
            earth.container.blend_mode = *mode;
            let _ = stage.add_child(stage.root(), earth);

            let _ = renderer.render_stage(&app, comp_rt.view(), Color::BLACK, &stage);
            let _ = blend_state; // silence unused
        } else {
            renderer.apply_advanced_blend(&app, *mode, &bg_rt, &fg_rt, &comp_rt);
        }

        let tile_bytes = comp_rt.read_pixels(&app);
        // Paste into sheet at (col, row).
        let dx = (col * TILE_W) as usize;
        let dy = (row * TILE_H) as usize;
        for ty in 0..(TILE_H as usize) {
            let src = ty * (TILE_W as usize) * 4;
            let dst = (dy + ty) * stride + dx * 4;
            let len = (TILE_W as usize) * 4;
            sheet_bytes[dst..dst + len].copy_from_slice(&tile_bytes[src..src + len]);
        }
    }

    // Overlay tile labels using the bitmap-text pipeline. Render labels
    // into a separate full-sheet RT, then alpha-composite via simple
    // CPU-side overwrite (cheaper than a real GPU pass for a one-shot
    // example).
    let labels_rt = RenderTexture::with_format(&app, total_w, total_h, format);
    {
        let mut stage = Stage::new();
        let font = Font::bitmap_8x8(&app);
        for (i, mode) in modes.iter().enumerate() {
            let i_u32 = u32::try_from(i).unwrap();
            let col = i_u32 % COLS;
            let row = i_u32 / COLS;
            #[allow(
                clippy::cast_precision_loss,
                reason = "small grid coordinates fit f32 easily"
            )]
            let cx = (col as f32 + 0.5) * (1.0 / COLS as f32) * 2.0 - 1.0;
            #[allow(
                clippy::cast_precision_loss,
                reason = "small grid coordinates fit f32 easily"
            )]
            let cy = 1.0 - (row as f32 + 1.0) * (1.0 / ROWS as f32) * 2.0 + 0.04;

            let mut label = Text::new(font.clone(), mode.css_name()).with_cell_size(0.018);
            label.color = Color::rgba(1.0, 1.0, 1.0, 1.0);
            #[allow(
                clippy::cast_precision_loss,
                reason = "name length fits comfortably in f32"
            )]
            let label_w = mode.css_name().len() as f32 * 0.018 * 8.0; // 8 px per glyph
            label.container.transform.position = Vec2::new(cx - label_w * 0.5, cy);
            let _ = stage.add_child(stage.root(), label);
        }
        let _ = renderer.render_stage(
            &app,
            labels_rt.view(),
            Color::rgba(0.0, 0.0, 0.0, 0.0),
            &stage,
        );
    }
    let label_bytes = labels_rt.read_pixels(&app);
    for (i, dst) in sheet_bytes.iter_mut().enumerate() {
        // Simple over-composite: where the label has alpha, draw a
        // solid white-on-translucent-shadow.
        let a = label_bytes[(i / 4) * 4 + 3];
        if a > 80 {
            *dst = 255;
        }
    }

    // Save.
    let out = std::path::Path::new("_docs/book/src/assets/wisp/blend-modes.png");
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let img = image::RgbaImage::from_raw(total_w, total_h, sheet_bytes).expect("dims");
    img.save(out).expect("save");
    println!(
        "wrote {} ({}×{}, {} modes)",
        out.display(),
        total_w,
        total_h,
        modes.len()
    );
    let _ = sheet_rt; // sheet_rt is unused but documents intent
    Ok(())
}
