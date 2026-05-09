//! `cargo run -p playback --example timed_playback`
//!
//! Drives a `Player` for 1 second of wallclock at a 60 Hz render cadence
//! against a 30 fps mock stream. Each frame the player uploads is also
//! rendered through wisp and saved to disk under
//! `_docs/book/src/assets/playback/`. End-of-run reports reproduce the
//! contract: ~30 frames pulled, state `Playing` (we exhaust before `Ended`
//! since the mock stream has 60 total frames).

use std::time::Duration;

use decode::mock::MockVideoStream;
use glam::Vec2;
use playback::{PlayState, Player};
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, RenderTexture, Sprite, Stage};

const W: u32 = 480;
const H: u32 = 270;

fn main() -> wisp::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("playback=info,wisp=warn")),
        )
        .init();

    let app = block_on(Application::new(AppConfig::default()))?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let renderer = Renderer::new(&app, format).expect("renderer");
    let rt = RenderTexture::with_format(&app, W, H, format);

    let stream = MockVideoStream::scrolling_gradient(W, H, 60).with_frame_rate(30.0);
    let mut player = Player::new(&app, Box::new(stream));
    player.play();

    let out_dir = std::path::Path::new("_docs/book/src/assets/playback");
    std::fs::create_dir_all(out_dir).ok();

    let dt = Duration::from_micros(16_667); // 60 Hz render
    let mut total_uploaded = 0u32;

    for tick in 0..60 {
        let pulled = player.tick(&app, dt);
        total_uploaded += pulled;
        if pulled > 0 {
            // Render the current player texture to disk so the chapter has
            // something to show.
            let mut sprite =
                Sprite::from_texture(player.texture().clone()).with_anchor(Vec2::splat(0.5));
            sprite.container.transform.scale = Vec2::splat(1.7);
            let mut stage = Stage::new();
            let _ = stage.add_child(stage.root(), sprite);
            let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
            let bytes = rt.read_pixels(&app);
            let buffer = image::RgbaImage::from_raw(W, H, bytes).expect("dims");
            let path = out_dir.join(format!("tick_{tick:02}.png"));
            buffer.save(&path).expect("save png");
        }
    }

    println!(
        "after 1s of wallclock @ 60Hz render against 30fps source: {} frames pulled, state = {:?}",
        total_uploaded,
        player.state()
    );
    assert_eq!(player.state(), PlayState::Playing);
    assert!((29..=31).contains(&total_uploaded));
    Ok(())
}
