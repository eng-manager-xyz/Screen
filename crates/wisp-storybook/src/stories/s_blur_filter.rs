//! Story: `BlurFilter` — sharp source vs blurred result, side by side (M0.16).

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{BlurFilter, Color, Fill, Graphics, RenderTexture, Sprite, Stage, Texture};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "filter-blur",
        category: "Filters",
        title: "Blur filter",
        milestone: "M0.16",
        writeup: include_str!("writeups/blur_filter.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    // Pre-render a "blurred" sprite onto a render texture, sample THAT here.
    // The story builds the scene into the live render — but we want the
    // viewer to see both the sharp version and the blurred one.
    //
    // For storybook simplicity: do the two-step ahead of time, then draw the
    // blurred result as a Sprite alongside a sharp Sprite from the same
    // source texture.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");
    let source = checker_texture(app);

    // Render the source onto an RT for blurring.
    let mut blur_input_stage = Stage::new();
    let mut sprite = Sprite::from_texture(source.clone()).with_anchor(Vec2::splat(0.5));
    sprite.container.transform.scale = Vec2::splat(0.7);
    let _ = blur_input_stage.add_child(blur_input_stage.root(), sprite);

    let input_rt = RenderTexture::with_format(app, 256, 256, format);
    let _ = renderer.render_stage(app, input_rt.view(), Color::BLACK, &blur_input_stage);
    let output_rt = RenderTexture::with_format(app, 256, 256, format);
    renderer.apply_filter(app, &BlurFilter::new(8.0), &input_rt, &output_rt);

    // Convert the blurred RT into a sampleable Texture for in-scene drawing.
    let blurred_bytes = output_rt.read_pixels(app);
    let blurred_texture = Texture::from_rgba(app, 256, 256, &blurred_bytes);

    // Left: sharp source. Right: blurred.
    let mut sharp = Sprite::from_texture(source).with_anchor(Vec2::splat(0.5));
    sharp.container.transform.position = Vec2::new(-0.45, 0.0);
    sharp.container.transform.scale = Vec2::splat(0.35);
    let _ = stage.add_child(stage.root(), sharp);

    let mut blurred = Sprite::from_texture(blurred_texture).with_anchor(Vec2::splat(0.5));
    blurred.container.transform.position = Vec2::new(0.45, 0.0);
    blurred.container.transform.scale = Vec2::splat(0.35);
    let _ = stage.add_child(stage.root(), blurred);

    // Labels under each.
    let mut label_left = Graphics::new();
    label_left.fill(Fill::Solid(Color::rgba_u8(120, 200, 255, 180)));
    label_left.draw_rounded_rect(Rect::new(-0.65, -0.55, 0.4, 0.06), 0.02);
    let mut label_right = Graphics::new();
    label_right.fill(Fill::Solid(Color::rgba_u8(255, 180, 120, 180)));
    label_right.draw_rounded_rect(Rect::new(0.25, -0.55, 0.4, 0.06), 0.02);
    let _ = stage.add_child(stage.root(), label_left);
    let _ = stage.add_child(stage.root(), label_right);
}

fn checker_texture(app: &Application) -> Texture {
    let size = 64u32;
    let mut bytes = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let cell_x = (x / 8) % 2;
            let cell_y = (y / 8) % 2;
            let dark = (cell_x ^ cell_y) == 1;
            if dark {
                bytes.extend_from_slice(&[255, 200, 120, 255]);
            } else {
                bytes.extend_from_slice(&[60, 40, 80, 255]);
            }
        }
    }
    Texture::from_rgba(app, size, size, &bytes)
}
