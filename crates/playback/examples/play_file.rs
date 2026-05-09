//! `cargo run -p playback --example play_file -- <path/to/video.mp4>`
//!
//! End-to-end playback of a real video file:
//! `GstreamerPipeStream` → `Player::tick` → `VideoTexture::upload_bgra` →
//! `wisp::Sprite` → `RenderTexture::read_pixels` → PNG.
//!
//! No CLI args = uses the committed test fixture
//! `crates/decode/tests/fixtures/sample.mp4` so the example is runnable
//! without the user hunting for a video.
//!
//! Output: per-tick PNGs under `target/play_file/tick_NN.png`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use decode::VideoStream;
use decode::gstreamer_pipe::GstreamerPipeStream;
use glam::Vec2;
use playback::{PlayState, Player};
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, RenderTexture, Sprite, Stage};

fn resolve_input() -> PathBuf {
    if let Some(arg) = std::env::args().nth(1) {
        return PathBuf::from(arg);
    }
    PathBuf::from("crates/decode/tests/fixtures/sample.mp4")
}

fn main() -> wisp::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("playback=info,decode=info,wisp=warn")
            }),
        )
        .init();

    let input = resolve_input();
    println!("opening {}", input.display());
    let stream = GstreamerPipeStream::open(&input).expect("open video");
    println!(
        "  → {}×{} @ {:.2} fps · frame count hint: {:?}",
        stream.width(),
        stream.height(),
        stream.frame_rate(),
        stream.frame_count_hint()
    );

    let app = block_on(Application::new(AppConfig::default()))?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let renderer = Renderer::new(&app, format).expect("renderer");
    let rt = RenderTexture::with_format(&app, stream.width(), stream.height(), format);

    let mut player = Player::new(&app, Box::new(stream));
    player.play();

    // Output goes into the docs assets so the chapter can embed them; the
    // existing `tick_NN.png` namespace belongs to `timed_playback`, so this
    // example uses the `playfile_NN.png` prefix.
    let out_dir = Path::new("_docs/book/src/assets/playback");
    std::fs::create_dir_all(out_dir).ok();

    let dt = Duration::from_secs_f64(1.0 / 30.0);
    let mut tick = 0u32;
    loop {
        let pulled = player.tick(&app, dt);
        if pulled > 0 {
            let mut sprite =
                Sprite::from_texture(player.texture().clone()).with_anchor(Vec2::splat(0.5));
            sprite.container.transform.scale = Vec2::splat(1.7);
            let mut stage = Stage::new();
            let _ = stage.add_child(stage.root(), sprite);
            let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
            let bytes = rt.read_pixels(&app);
            let buffer = image::RgbaImage::from_raw(rt.width(), rt.height(), bytes).expect("dims");
            let path = out_dir.join(format!("playfile_{tick:02}.png"));
            buffer.save(&path).expect("save png");
            println!(
                "  ✓ tick {tick:02} pulled={pulled} pts={:.3?} idx={:?}",
                player.last_uploaded_pts(),
                player.last_uploaded_index()
            );
        }
        if player.state() == PlayState::Ended {
            break;
        }
        tick += 1;
        if tick > 600 {
            eprintln!("safety break — tick > 600");
            break;
        }
    }

    println!("done. final state = {:?}", player.state());
    Ok(())
}
