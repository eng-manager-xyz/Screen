//! `cargo run -p wisp --example playback_demo`
//!
//! Drives the full **decode → upload → render** path end-to-end with a
//! mock video stream. Eight BGRA frames are pulled from
//! [`decode::mock::MockVideoStream`], uploaded into a `VideoTexture` per
//! frame, rendered through wisp's `Sprite` pipeline, and written to PNG
//! under `_docs/book/src/assets/decode/`.
//!
//! This is the architectural proof point for the chunk:
//! - `MockVideoStream` is an arbitrary [`decode::VideoStream`] — swapping
//!   in a real `AvFoundationStream` (M-DEC.2) is a one-line change.
//! - `VideoTexture::upload_bgra` consumes the same byte layout the real
//!   codec backends will produce.
//! - The final PNGs prove there's no GPU-side trickery between the decode
//!   contract and what the user will see in the player.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, RenderTexture, Sprite, Stage, VideoTexture};

use decode::VideoStream;
use decode::mock::MockVideoStream;

const W: u32 = 480;
const H: u32 = 270;
const FRAMES: u64 = 8;

fn main() -> wisp::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("wisp=info,decode=info")),
        )
        .init();

    let app = block_on(Application::new(AppConfig::default()))?;

    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let renderer = Renderer::new(&app, format).expect("renderer");
    let rt = RenderTexture::with_format(&app, W, H, format);

    let video = VideoTexture::new(&app, W, H);

    let out_dir = std::path::Path::new("_docs/book/src/assets/decode");
    std::fs::create_dir_all(out_dir).ok();

    let mut stream = MockVideoStream::scrolling_gradient(W, H, FRAMES);

    println!(
        "playing {}×{} @ {:.0} fps · {} frames",
        stream.width(),
        stream.height(),
        stream.frame_rate(),
        stream.frame_count_hint().unwrap_or(0),
    );

    while let Some(frame) = stream.next_frame() {
        // Upload the decoded BGRA frame to the GPU.
        video.upload_bgra(&app, &frame.bgra);

        // Build a stage with one sprite sourced from the live texture.
        let mut sprite =
            Sprite::from_texture(video.texture().clone()).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.scale = Vec2::splat(1.7);
        let mut stage = Stage::new();
        let _ = stage.add_child(stage.root(), sprite);

        let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);

        let bytes = rt.read_pixels(&app);
        let buffer = image::RgbaImage::from_raw(W, H, bytes).expect("dims");
        let path = out_dir.join(format!("frame_{:02}.png", frame.frame_index));
        buffer.save(&path).expect("save png");
        println!(
            "  ✓ frame {:02} pts={:.3}s → {}",
            frame.frame_index,
            frame.pts_seconds,
            path.display()
        );
    }

    println!("done.");
    Ok(())
}
