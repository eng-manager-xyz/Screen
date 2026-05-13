//! `VideoTexture` per-frame upload demo, headless.
//!
//! Synthesizes 8 procedural BGRA "frames" (a scrolling vertical gradient),
//! uploads each to a `VideoTexture`, renders, and saves all 8 PNGs to
//! `target/video_texture/`.
//!
//! Run: `cargo run -p screen-wisp --example video_texture`.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, RenderTexture, Sprite, Stage, VideoTexture};

const W: u32 = 480;
const H: u32 = 270;
const FRAMES: u32 = 8;

fn main() -> wisp::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("wisp=info")),
        )
        .init();

    let app = block_on(Application::new(AppConfig::default()))?;

    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let renderer = Renderer::new(&app, format).expect("renderer");
    let rt = RenderTexture::with_format(&app, W, H, format);

    let video = VideoTexture::new(&app, W, H);

    let out_dir = std::path::Path::new("target/video_texture");
    std::fs::create_dir_all(out_dir).ok();

    for frame in 0..FRAMES {
        let bgra = make_frame(W, H, frame);
        video.upload_bgra(&app, &bgra);

        let mut sprite =
            Sprite::from_texture(video.texture().clone()).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.scale = Vec2::splat(1.7);
        let mut stage = Stage::new();
        let _ = stage.add_child(stage.root(), sprite);

        let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);

        let bytes = rt.read_pixels(&app);
        let buffer = image::RgbaImage::from_raw(W, H, bytes).expect("dims");
        let path = out_dir.join(format!("frame_{frame:02}.png"));
        buffer.save(&path).expect("save png");
        println!("wrote {}", path.display());
    }

    Ok(())
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    reason = "byte values bounded by rem_euclid(256.0); single-letter names match the gradient math conventions"
)]
fn make_frame(w: u32, h: u32, frame: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((w * h * 4) as usize);
    let phase = (frame as f32) / (FRAMES as f32);
    for y in 0..h {
        for x in 0..w {
            let u = (x as f32) / (w as f32);
            let v = (y as f32) / (h as f32);
            let r = ((u + phase) * 255.0).rem_euclid(256.0) as u8;
            let g = ((v + phase) * 255.0).rem_euclid(256.0) as u8;
            let b = (((u + v + phase) * 0.5) * 255.0).rem_euclid(256.0) as u8;
            // VideoTexture is BGRA8.
            bytes.push(b);
            bytes.push(g);
            bytes.push(r);
            bytes.push(255);
        }
    }
    bytes
}
